use bevy::prelude::*;
use mlua::prelude::*;
use crate::scripting::userdata::cframe::CFrame;
use crate::scripting::userdata::color3::Color3;
use crate::scripting::userdata::vector3::Vector3;
use crate::scripting::vm::scheduler::{SchedulerRef, LuaTask, yielded_to_wake};

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

fn lua_value_to_string(lua: &Lua, value: &LuaValue) -> String {
    match value {
        LuaValue::Nil => "nil".to_string(),
        LuaValue::String(s) => s.to_string_lossy(),
        _ => lua
            .globals()
            .get::<LuaFunction>("tostring")
            .and_then(|f| f.call::<LuaString>(value.clone()))
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|_| format!("{:?}", value)),
    }
}

fn caller_source(lua: &Lua) -> (String, Option<u32>) {
    match lua
        .globals()
        .get::<LuaFunction>("__vertigo_callerinfo")
        .and_then(|f| f.call::<(Option<String>, Option<u32>)>(()))
    {
        Ok((Some(source), line)) => (source, line),
        _ => ("Script".to_string(), None),
    }
}

pub fn setup_globals(lua: &Lua) -> Result<(), mlua::Error> {
    let task_table = lua.create_table()?;

    let wait_fn: LuaFunction = lua.load("function(seconds) return coroutine.yield(seconds or 0) end").eval()?;
    task_table.set("wait", wait_fn.clone())?;

    let spawn_fn = lua.create_function(|lua, val: LuaValue| {
        let thread = match val {
            LuaValue::Function(f) => lua.create_thread(f)?,
            LuaValue::Thread(t) => t,
            _ => return Err(mlua::Error::RuntimeError("task.spawn expects function or thread".to_string())),
        };
        match thread.resume::<LuaValue>(()) {
            Ok(yielded) => {
                if thread.status() == LuaThreadStatus::Resumable {
                    let scheduler_ref = lua.app_data_ref::<SchedulerRef>().unwrap();
                    let mut scheduler = scheduler_ref.0.lock().unwrap();
                    if let Ok(key) = lua.create_registry_value(thread) {
                        scheduler.tasks.push(LuaTask {
                            thread_key: key,
                            wake_time: yielded_to_wake(yielded, std::time::Instant::now()),
                        });
                    }
                }
            }
            Err(e) => {
                error!("Luau task.spawn error: {}", e);
                crate::scripting::output::push_error("task.spawn", e.to_string());
            }
        }
        Ok(())
    })?;
    task_table.set("spawn", spawn_fn.clone())?;

    let defer_fn = lua.create_function(|lua, f: LuaFunction| {
        let scheduler_ref = lua.app_data_ref::<SchedulerRef>().unwrap();
        let mut scheduler = scheduler_ref.0.lock().unwrap();
        let thread = lua.create_thread(f)?;
        let key = lua.create_registry_value(thread)?;
        scheduler.deferred.push_back(key);
        Ok(())
    })?;
    task_table.set("defer", defer_fn)?;

    let delay_fn = lua.create_function(|lua, (seconds, f): (f32, LuaFunction)| {
        let scheduler_ref = lua.app_data_ref::<SchedulerRef>().unwrap();
        let mut scheduler = scheduler_ref.0.lock().unwrap();
        let thread = lua.create_thread(f)?;
        let key = lua.create_registry_value(thread)?;
        scheduler.tasks.push(LuaTask {
            thread_key: key,
            wake_time: Some(std::time::Instant::now() + std::time::Duration::from_secs_f64(seconds as f64)),
        });
        Ok(())
    })?;
    task_table.set("delay", delay_fn)?;

    lua.globals().set("task", task_table)?;

    lua.globals().set("wait", wait_fn)?;
    lua.globals().set("spawn", spawn_fn)?;
    let delay_global: LuaFunction = lua.load(
        "function(seconds, f, ...) local args = {...} task.delay(seconds, function() f(table.unpack(args)) end) end",
    ).eval()?;
    lua.globals().set("delay", delay_global)?;

    let print_fn = lua.create_function(|lua, args: LuaMultiValue| {
        let parts: Vec<String> = args.iter().map(|v| lua_value_to_string(lua, v)).collect();
        let message = parts.join("\t");
        info!("LUA_PRINT: {}", message);
        let (source, line) = caller_source(lua);
        crate::scripting::output::push(
            crate::scripting::output::OutputLevel::Info,
            &source,
            line,
            message,
            None,
        );
        Ok(())
    })?;
    lua.globals().set("print", print_fn)?;

    let warn_fn = lua.create_function(|lua, args: LuaMultiValue| {
        let parts: Vec<String> = args.iter().map(|v| lua_value_to_string(lua, v)).collect();
        let message = parts.join("\t");
        warn!("LUA_WARN: {}", message);
        let (source, line) = caller_source(lua);
        crate::scripting::output::push(
            crate::scripting::output::OutputLevel::Warn,
            &source,
            line,
            message,
            None,
        );
        Ok(())
    })?;
    lua.globals().set("warn", warn_fn)?;

    let vector3_class = lua.create_table()?;
    vector3_class.set("new", lua.create_function(|_, (x, y, z): (f32, f32, f32)| {
        Ok(Vector3(Vec3::new(x, y, z)))
    })?)?;
    lua.globals().set("Vector3", vector3_class)?;

    let color3_class = lua.create_table()?;
    color3_class.set("new", lua.create_function(|_, (r, g, b): (f32, f32, f32)| {
        Ok(Color3(Color::Srgba(Srgba::new(r, g, b, 1.0))))
    })?)?;
    color3_class.set("fromRGB", lua.create_function(|_, (r, g, b): (f32, f32, f32)| {
        Ok(Color3(Color::Srgba(Srgba::new(r / 255.0, g / 255.0, b / 255.0, 1.0))))
    })?)?;
    color3_class.set("fromHSV", lua.create_function(|_, (h, s, v): (f32, f32, f32)| {
        let (r, g, b) = hsv_to_rgb(h, s, v);
        Ok(Color3(Color::Srgba(Srgba::new(r, g, b, 1.0))))
    })?)?;
    color3_class.set("fromHex", lua.create_function(|_, hex: String| {
        let hex = hex.trim_start_matches('#');
        let value = u32::from_str_radix(hex, 16)
            .map_err(|_| mlua::Error::RuntimeError(format!("Color3.fromHex: invalid hex color '{}'", hex)))?;
        let r = ((value >> 16) & 0xFF) as f32 / 255.0;
        let g = ((value >> 8) & 0xFF) as f32 / 255.0;
        let b = (value & 0xFF) as f32 / 255.0;
        Ok(Color3(Color::Srgba(Srgba::new(r, g, b, 1.0))))
    })?)?;
    lua.globals().set("Color3", color3_class)?;

    let cframe_class = lua.create_table()?;
    cframe_class.set("new", lua.create_function(|lua, args: LuaMultiValue| {
        if args.is_empty() {
            return lua.create_userdata(CFrame {
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            }).map(LuaValue::UserData);
        }
        if let Some(LuaValue::UserData(ud)) = args.get(0) {
            if let Ok(vector) = ud.borrow::<Vector3>() {
                return lua.create_userdata(CFrame {
                    position: vector.0,
                    rotation: Quat::IDENTITY,
                }).map(LuaValue::UserData);
            }
        }
        let (x, y, z): (f32, f32, f32) = match (args.get(0), args.get(1), args.get(2)) {
            (Some(LuaValue::Number(x)), Some(LuaValue::Number(y)), Some(LuaValue::Number(z))) => {
                (*x as f32, *y as f32, *z as f32)
            }
            (Some(LuaValue::Integer(x)), Some(LuaValue::Integer(y)), Some(LuaValue::Integer(z))) => {
                (*x as f32, *y as f32, *z as f32)
            }
            _ => {
                return Err(mlua::Error::RuntimeError(
                    "CFrame.new expects (Vector3) or (x, y, z)".to_string(),
                ))
            }
        };
        lua.create_userdata(CFrame {
            position: Vec3::new(x, y, z),
            rotation: Quat::IDENTITY,
        }).map(LuaValue::UserData)
    })?)?;
    cframe_class.set("angles", lua.create_function(|lua, (rx, ry, rz): (f32, f32, f32)| {
        lua.create_userdata(CFrame {
            position: Vec3::ZERO,
            rotation: Quat::from_euler(EulerRot::XYZ, rx, ry, rz),
        }).map(LuaValue::UserData)
    })?)?;
    cframe_class.set("lookAt", lua.create_function(|lua, (from, to): (LuaAnyUserData, LuaAnyUserData)| {
        let from = from.borrow::<Vector3>()
            .map_err(|_| mlua::Error::RuntimeError("CFrame.lookAt expects two Vector3s".to_string()))?;
        let to = to.borrow::<Vector3>()
            .map_err(|_| mlua::Error::RuntimeError("CFrame.lookAt expects two Vector3s".to_string()))?;
        let dir = (to.0 - from.0).normalize_or_zero();
        lua.create_userdata(CFrame {
            position: from.0,
            rotation: Quat::from_rotation_arc(Vec3::NEG_Z, dir),
        }).map(LuaValue::UserData)
    })?)?;
    lua.globals().set("CFrame", cframe_class)?;

    let instance_class = lua.create_table()?;
    instance_class.set("new", lua.create_function(|lua, class_name: String| {
        use crate::common::game::bricks::components::{Brick, BrickColor, BrickPhysics, BrickShapeComponent};
        use crate::scripting::userdata::instance::Instance;

        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
        let world = unsafe { &mut *world_ref.0 };
        let id = match class_name.as_str() {
            "Part" => {
                let cmd = world.spawn((
                    Name::new(class_name),
                    Transform::default(),
                    Brick,
                    BrickShapeComponent::default(),
                    BrickPhysics::default(),
                    BrickColor::default(),
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                    lightyear::prelude::Replicate::default(),
                ));
                cmd.id()
            }
            "Folder" => {
                world.spawn((Name::new(class_name),)).id()
            }
            _ => {
                return Err(mlua::Error::RuntimeError(format!(
                    "Instance.new: unsupported class '{}' (supported: Part, Folder)",
                    class_name
                )))
            }
        };
        lua.create_userdata(Instance { entity: id }).map(LuaValue::UserData)
    })?)?;
    lua.globals().set("Instance", instance_class)?;

    let caller_info: LuaFunction = lua.load(
        "return function()
            for level = 1, 6 do
                local source, line = debug.info(level, 'sl')
                if source
                    and source ~= '[C]'
                    and source ~= '=[C]'
                    and not source:find('globals.rs', 1, true)
                    and not source:find('__vertigo', 1, true) then
                    return source, line
                end
            end
            return 'Script', nil
        end",
    ).eval()?;
    lua.globals().set("__vertigo_callerinfo", caller_info)?;

    let wait_for_child_impl: LuaFunction = lua.load(
        "return function(self, name, timeout)
            local waited = 0
            timeout = timeout or 5
            while waited < timeout do
                local child = self:FindFirstChild(name)
                if child then
                    return child
                end
                task.wait(0.1)
                waited = waited + 0.1
            end
            error('WaitForChild timed out: ' .. name, 0)
        end",
    ).eval()?;
    lua.globals().set("__vertigo_waitforchild", wait_for_child_impl)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::testing::{
        advance, entity_of, eval, global, run_script, test_vm, test_world, tick,
    };


    #[test]
    fn task_spawn_runs_functions_immediately() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, "task.spawn(function() _G.ran = true end)");
        assert!(global::<bool>(&vm, "ran"));
    }

    #[test]
    fn task_spawn_accepts_threads() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            local th = coroutine.create(function() _G.from_thread = true end)
            task.spawn(th)
        "#);
        assert!(global::<bool>(&vm, "from_thread"));
    }

    #[test]
    fn task_spawn_rejects_non_functions() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let ok: bool = eval(&vm, "return pcall(task.spawn, 123)");
        assert!(!ok, "task.spawn must reject non-function values");
    }

    #[test]
    fn task_spawn_swallows_spawned_errors() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let ok: bool = eval(&vm, "return pcall(task.spawn, function() error('boom') end)");
        assert!(ok, "errors inside spawned functions must not propagate to the caller");
    }

    #[test]
    fn task_defer_runs_on_next_tick() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, "task.defer(function() _G.deferred = true end)");
        assert!(!global::<bool>(&vm, "deferred"), "deferred must not run before the next tick");
        tick(&vm);
        assert!(global::<bool>(&vm, "deferred"));
    }

    #[test]
    fn task_delay_runs_after_elapsed_time() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, "task.delay(0.01, function() _G.delayed = true end)");
        tick(&vm);
        assert!(!global::<bool>(&vm, "delayed"));
        advance(&vm, 20, 1);
        assert!(global::<bool>(&vm, "delayed"));
    }

    #[test]
    fn task_wait_yields_until_wake_time() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            _G.before = true
            task.wait(0.01)
            _G.after = true
        "#);
        assert!(global::<bool>(&vm, "before"));
        assert!(!global::<bool>(&vm, "after"), "script must yield on task.wait");
        advance(&vm, 20, 1);
        assert!(global::<bool>(&vm, "after"));
    }

    #[test]
    fn global_wait_spawn_aliases_exist() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        assert!(eval::<bool>(&vm, "return wait == task.wait"));
        assert!(eval::<bool>(&vm, "return spawn == task.spawn"));
        assert!(eval::<bool>(&vm, "return type(delay) == 'function'"));
    }

    #[test]
    fn global_delay_forwards_arguments() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            delay(0.01, function(a, b, c)
                _G.dargs = tostring(a) .. b .. c
            end, "x", 2, "y")
        "#);
        advance(&vm, 20, 1);
        assert_eq!(global::<String>(&vm, "dargs"), "x2y");
    }


    #[test]
    fn print_and_warn_accept_any_argument_mix() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            print("a", 1, 2.5, true, nil, Vector3.new(1, 2, 3), tostring)
            warn("w", nil, false)
        "#);
    }

    #[test]
    fn print_warn_and_errors_feed_the_output_buffer() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let before = crate::scripting::output::buffer().lock().unwrap().entries.len();

        run_script(&vm, r#"
            print("buffer hello", 42)
            warn("buffer warn")
            task.spawn(function()
                error("buffer boom")
            end)
        "#);

        let buf = crate::scripting::output::buffer().lock().unwrap();
        let new: Vec<_> = buf.entries.iter().skip(before).collect();
        let describe = || {
            new.iter()
                .map(|e| format!("{:?}:{} {}", e.level, e.source, e.message))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(
            new.iter().any(|e| e.level == crate::scripting::output::OutputLevel::Info && e.message.contains("buffer hello")),
            "missing print entry:\n{}",
            describe()
        );
        assert!(
            new.iter().any(|e| e.level == crate::scripting::output::OutputLevel::Warn && e.message.contains("buffer warn")),
            "missing warn entry:\n{}",
            describe()
        );
        assert!(
            new.iter().any(|e| e.level == crate::scripting::output::OutputLevel::Error && e.message.contains("buffer boom")),
            "missing error entry:\n{}",
            describe()
        );
        let info = new.iter().find(|e| e.message.contains("buffer hello")).unwrap();
        assert!(
            info.source.contains("test"),
            "print must be attributed to the calling chunk, got: {}",
            info.source
        );
    }

    #[test]
    fn lua_value_to_string_formats_like_lua() {
        let mut world = test_world();
        let vm = test_vm(&mut world);

        assert_eq!(lua_value_to_string(&vm.lua, &LuaValue::Nil), "nil");
        assert_eq!(lua_value_to_string(&vm.lua, &LuaValue::Boolean(true)), "true");
        assert_eq!(lua_value_to_string(&vm.lua, &LuaValue::Boolean(false)), "false");
        assert_eq!(lua_value_to_string(&vm.lua, &LuaValue::Number(3.5)), "3.5");

        let vec: LuaValue = eval(&vm, "return Vector3.new(1, 2, 3)");
        assert_eq!(lua_value_to_string(&vm.lua, &vec), "1, 2, 3");

        let table: LuaValue = eval(&vm, "return { 1, 2 }");
        assert!(
            lua_value_to_string(&vm.lua, &table).starts_with("table:"),
            "tables must stringify via tostring"
        );

        let func: LuaValue = eval(&vm, "return function() end");
        assert!(
            lua_value_to_string(&vm.lua, &func).starts_with("function:"),
            "functions must stringify via tostring"
        );
    }

    #[test]
    fn error_global_raises_with_message() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let ok: bool = eval(&vm, "return pcall(function() error('custom message') end)");
        assert!(!ok);
    }

    #[test]
    fn pcall_errors_are_clean_messages() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let msg: String = eval(
            &vm,
            "local ok, err = pcall(function() error('plain message') end) return tostring(err)",
        );
        assert!(msg.contains("plain message"), "got: {msg}");
        assert!(!msg.contains("runtime error"), "got: {msg}");
        assert!(!msg.contains("stack traceback"), "got: {msg}");
    }

    #[test]
    fn error_raises_non_string_values_untouched() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let is_table: bool = eval(
            &vm,
            "local ok, err = pcall(function() error({ code = 42 }) end) return type(err) == 'table'",
        );
        assert!(is_table);
    }

    #[test]
    fn service_instances_compare_equal() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (ws, players, run): (bool, bool, bool) = eval(
            &vm,
            r#"
                return workspace == Workspace,
                       workspace == game.Workspace,
                       Players == game.Players
            "#,
        );
        assert!(ws);
        assert!(players);
        assert!(run);
    }


    #[test]
    fn vector3_construction_and_properties() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (x, y, z, mag, unit_x, unit_y, unit_z): (f64, f64, f64, f64, f64, f64, f64) = eval(
            &vm,
            "local v = Vector3.new(3, 4, 12) return v.X, v.Y, v.Z, v.Magnitude, v.Unit.X, v.Unit.Y, v.Unit.Z",
        );
        assert_eq!((x, y, z), (3.0, 4.0, 12.0));
        assert_eq!(mag, 13.0);
        assert!((unit_x - 3.0 / 13.0).abs() < 1e-5);
        assert!((unit_y - 4.0 / 13.0).abs() < 1e-5);
        assert!((unit_z - 12.0 / 13.0).abs() < 1e-5);
    }

    #[test]
    fn vector3_arithmetic_operators() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (sum_x, diff_y, scaled_x, halved_z, neg_x): (f64, f64, f64, f64, f64) = eval(
            &vm,
            r#"
                local a = Vector3.new(1, 2, 3)
                local b = Vector3.new(10, 20, 30)
                return (a + b).X, (b - a).Y, (a * 2).X, (b / 10).Z, (-a).X
            "#,
        );
        assert_eq!(sum_x, 11.0);
        assert_eq!(diff_y, 18.0);
        assert_eq!(scaled_x, 2.0);
        assert_eq!(halved_z, 3.0);
        assert_eq!(neg_x, -1.0);
    }

    #[test]
    fn vector3_dot_cross_lerp_and_equality() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (dot, cross_x, cross_y, cross_z, lerp_x, lerp_z, eq, ne, str): (
            f64, f64, f64, f64, f64, f64, bool, bool, String,
        ) = eval(
            &vm,
            r#"
                local a = Vector3.new(1, 2, 3)
                local b = Vector3.new(4, 5, 6)
                local c = a:Cross(b)
                local m = a:Lerp(b, 0.5)
                return a:Dot(b), c.X, c.Y, c.Z, m.X, m.Z,
                       a == Vector3.new(1, 2, 3), a == b, tostring(a)
            "#,
        );
        assert_eq!(dot, 32.0);
        assert_eq!((cross_x, cross_y, cross_z), (-3.0, 6.0, -3.0));
        assert_eq!((lerp_x, lerp_z), (2.5, 4.5));
        assert!(eq);
        assert!(!ne);
        assert_eq!(str, "1, 2, 3");
    }

    #[test]
    fn vector3_errors_on_wrong_operands() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let ok: bool = eval(&vm, "return pcall(function() return Vector3.new(1, 2, 3) + 5 end)");
        assert!(!ok);
    }


    #[test]
    fn color3_construction_channels_and_strings() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (r, g, b, r2, g2, b2, hex, str): (f64, f64, f64, f64, f64, f64, String, String) = eval(
            &vm,
            r#"
                local c = Color3.new(1, 0.5, 0.25)
                local d = Color3.fromRGB(255, 128, 64)
                return c.R, c.G, c.B, d.R, d.G, d.B, c:ToHex(), tostring(c)
            "#,
        );
        assert_eq!((r, g, b), (1.0, 0.5, 0.25));
        assert!((r2 - 1.0).abs() < 0.01);
        assert!((g2 - 128.0 / 255.0).abs() < 0.01);
        assert!((b2 - 64.0 / 255.0).abs() < 0.01);
        assert_eq!(hex, "#FF8040");
        assert_eq!(str, "1, 0.5, 0.25");
    }

    #[test]
    fn color3_hsv_and_hex_construction() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (r, g, b, r2, g2): (f64, f64, f64, f64, f64) = eval(
            &vm,
            r##"
                local h = Color3.fromHSV(0, 1, 1)
                local x = Color3.fromHex("#00FF80")
                return h.R, h.G, h.B, x.R, x.G
            "##,
        );
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.0).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
        assert!((r2 - 0.0).abs() < 0.01);
        assert!((g2 - 1.0).abs() < 0.01);
    }

    #[test]
    fn color3_rejects_bad_hex() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let ok: bool = eval(&vm, "return pcall(Color3.fromHex, 'not-hex')");
        assert!(!ok);
    }


    #[test]
    fn cframe_construction_forms() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (x, y, z, vx, vy, vz): (f64, f64, f64, f64, f64, f64) = eval(
            &vm,
            r#"
                local a = CFrame.new(1, 2, 3)
                local b = CFrame.new(Vector3.new(4, 5, 6))
                local id = CFrame.new()
                return a.Position.X, a.Position.Y, a.Position.Z,
                       b.Position.X, b.Position.Y, b.Position.Z,
                       id.Position.X + 0
            "#,
        );
        assert_eq!((x, y, z), (1.0, 2.0, 3.0));
        assert_eq!((vx, vy, vz), (4.0, 5.0, 6.0));
    }

    #[test]
    fn cframe_angles_and_vector_math() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (rot_x, rot_y): (f64, f64) = eval(
            &vm,
            r#"
                -- 90 degrees around +Z maps +X onto +Y (right-handed).
                local v = CFrame.angles(0, 0, math.pi / 2) * Vector3.new(1, 0, 0)
                return v.X, v.Y
            "#,
        );
        assert!(rot_x.abs() < 1e-5);
        assert!((rot_y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cframe_rotation_and_compound_math() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (px, py, pz, rx, ry, rz): (f64, f64, f64, f64, f64, f64) = eval(
            &vm,
            r#"
                local cf = CFrame.new(1, 0, 0) * CFrame.new(0, 2, 0)
                local v = CFrame.new(5, 6, 7) * CFrame.angles(0, 0.5, 0)
                local e = v:ToEulerAnglesXYZ()
                return cf.Position.X, cf.Position.Y, cf.Position.Z, e.X, e.Y, e.Z
            "#,
        );
        assert_eq!((px, py, pz), (1.0, 2.0, 0.0));
        assert!(rx.abs() < 1e-3);
        assert!((ry - 0.5).abs() < 1e-3);
        assert!(rz.abs() < 1e-3);
    }

    #[test]
    fn cframe_axes_and_lerp() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let (lx, ly, lz, ux, uy, rx, rz, mx, my, mz): (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) = eval(
            &vm,
            r#"
                local id = CFrame.new()
                local m = CFrame.new(0, 0, 0):Lerp(CFrame.new(10, 20, 30), 0.5)
                return id.LookVector.X, id.LookVector.Y, id.LookVector.Z,
                       id.UpVector.X, id.UpVector.Y,
                       id.RightVector.X, id.RightVector.Z,
                       m.Position.X, m.Position.Y, m.Position.Z
            "#,
        );
        assert!((lx - 0.0).abs() < 1e-6 && (ly - 0.0).abs() < 1e-6 && (lz + 1.0).abs() < 1e-6);
        assert!((ux - 0.0).abs() < 1e-6 && (uy - 1.0).abs() < 1e-6);
        assert!((rx - 1.0).abs() < 1e-6 && rz.abs() < 1e-6);
        assert_eq!((mx, my, mz), (5.0, 10.0, 15.0));
    }

    #[test]
    fn cframe_tostring_has_no_negative_zero() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let s: String = eval(&vm, "return tostring(CFrame.new())");
        assert!(!s.contains("-0"), "got: {s}");
        let e: String = eval(&vm, "return tostring(CFrame.angles(0, 0, 0):ToEulerAnglesXYZ())");
        assert!(!e.contains("-0"), "got: {e}");
    }

    #[test]
    fn cframe_string_and_invalid_multiplier() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let str: String = eval(&vm, "return tostring(CFrame.new(1, 2, 3))");
        assert!(str.starts_with("1, 2, 3,"), "unexpected tostring: {str}");
        let ok: bool = eval(&vm, "return pcall(function() return CFrame.new() * 5 end)");
        assert!(!ok);
    }


    #[test]
    fn instance_new_creates_parts_and_folders() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            _G.part = Instance.new("Part")
            _G.folder = Instance.new("Folder")
        "#);
        let part = entity_of(&vm, "part");
        let folder = entity_of(&vm, "folder");
        assert!(world.get::<crate::common::game::bricks::components::Brick>(part).is_some());
        assert!(world.get::<Name>(folder).is_some());
        assert!(world.get::<crate::common::game::bricks::components::Brick>(folder).is_none());
        assert!(world.get::<lightyear::prelude::Replicate>(part).is_some());
        assert!(world.get::<avian3d::prelude::RigidBody>(part).is_some());
    }

    #[test]
    fn instance_new_rejects_unsupported_classes() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let ok: bool = eval(&vm, "return pcall(Instance.new, 'MeshPart')");
        assert!(!ok);
    }
}
