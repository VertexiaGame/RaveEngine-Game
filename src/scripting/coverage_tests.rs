use crate::common::net::components::{LightingServiceContainer, PlayersServiceContainer};
use crate::scripting::ecs::{LocalScript, ModuleScript, ServerScript};
use crate::scripting::plugin::ScriptingPlugin;
use crate::scripting::testing::{advance, entity_of, eval, global, run_script, test_vm, test_world, tick, try_script};
use crate::scripting::userdata::instance::Instance;
use crate::scripting::vm::client_vm::ClientScriptVM;
use crate::scripting::vm::server_vm::ServerScriptVM;
use bevy::prelude::*;
use mlua::prelude::*;

fn full_world() -> World {
    let mut world = test_world();
    world.spawn(Name::new("Workspace"));
    world.spawn((Name::new("Players"), PlayersServiceContainer));
    world.spawn((Name::new("Lighting"), LightingServiceContainer));
    world.spawn(Name::new("AssetService"));
    world.insert_resource(avian3d::prelude::Gravity(Vec3::NEG_Y * 196.2));
    world.insert_resource(crate::client::sky::LightingConfig::default());
    world
}

fn script_app() -> App {
    let mut app = App::new();
    app.add_plugins(ScriptingPlugin);
    app.world_mut()
        .insert_resource(bevy::time::Time::<()>::default());
    app.world_mut().insert_resource(ServerScriptVM::new());
    app.world_mut().insert_resource(ClientScriptVM::new());
    app
}

fn server_global<T: FromLua>(app: &App, name: &str) -> T {
    app.world()
        .resource::<ServerScriptVM>()
        .lua
        .globals()
        .get(name)
        .unwrap()
}

fn client_global<T: FromLua>(app: &App, name: &str) -> T {
    app.world()
        .resource::<ClientScriptVM>()
        .lua
        .globals()
        .get(name)
        .unwrap()
}

fn expose(vm: &ServerScriptVM, name: &str, entity: Entity) {
    vm.lua
        .globals()
        .set(name, vm.lua.create_userdata(Instance { entity }).unwrap())
        .unwrap();
}

#[test]
fn instance_new_supports_all_classes() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        _G.part = Instance.new("Part")
        _G.folder = Instance.new("Folder")
        _G.image = Instance.new("Image")
        _G.mesh = Instance.new("Mesh")
        _G.texture = Instance.new("Texture")
        _G.sound = Instance.new("Sound")
        "#,
    );
    let (p, f, i, m, t, s): (String, String, String, String, String, String) = eval(
        &vm,
        r#"return _G.part.ClassName, _G.folder.ClassName, _G.image.ClassName, _G.mesh.ClassName, _G.texture.ClassName, _G.sound.ClassName"#,
    );
    assert_eq!(p, "Part");
    assert_eq!(f, "Folder");
    assert_eq!(i, "Image");
    assert_eq!(m, "Mesh");
    assert_eq!(t, "Texture");
    assert_eq!(s, "Sound");
    assert!(world.get::<crate::common::game::bricks::components::Brick>(entity_of(&vm, "part")).is_some());
    assert!(world.get::<crate::common::game::assets::components::Image>(entity_of(&vm, "image")).is_some());
    assert!(world.get::<crate::common::game::assets::components::Mesh>(entity_of(&vm, "mesh")).is_some());
    assert!(world.get::<crate::common::game::assets::components::Texture>(entity_of(&vm, "texture")).is_some());
    assert!(world.get::<crate::common::game::assets::components::Sound>(entity_of(&vm, "sound")).is_some());
}

#[test]
fn brickcolor_alias_matches_color() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        local p = Instance.new("Part")
        p.BrickColor = Color3.fromRGB(10, 20, 30)
        _G.via_color = p.Color
        _G.via_brick = p.BrickColor
        p.Color = Color3.fromRGB(200, 100, 50)
        _G.after = p.BrickColor
        "#,
    );
    let (r1, r2, r3): (f64, f64, f64) = eval(&vm, "return _G.via_color.R, _G.via_brick.R, _G.after.R");
    assert!((r1 - 10.0 / 255.0).abs() < 0.01);
    assert!((r2 - 10.0 / 255.0).abs() < 0.01);
    assert!((r3 - 200.0 / 255.0).abs() < 0.01);
}

#[test]
fn player_gravity_round_trips() {
    let mut world = full_world();
    let player = world
        .spawn((
            Name::new("P"),
            crate::common::net::components::Player {
                client_id: 1,
                gravity: 100.0,
                ..default()
            },
        ))
        .id();
    let vm = test_vm(&mut world);
    expose(&vm, "p", player);
    let before: f64 = eval(&vm, "return _G.p.Gravity");
    assert!((before - 100.0 / 0.28).abs() < 0.5);
    run_script(&vm, "_G.p.Gravity = 60");
    let stored = world.get::<crate::common::net::components::Player>(player).unwrap();
    assert!((stored.gravity - 60.0 * 0.28).abs() < 1e-4);
    let after: f64 = eval(&vm, "return _G.p.Gravity");
    assert_eq!(after, 60.0);
}

#[test]
fn speedresponse_and_speedmode_aliases() {
    let mut world = full_world();
    let player = world
        .spawn((
            Name::new("P"),
            crate::common::net::components::Player::default(),
        ))
        .id();
    let vm = test_vm(&mut world);
    expose(&vm, "p", player);
    run_script(&vm, "_G.p.SpeedResponse = 'Exponential'");
    let via_mode: String = eval(&vm, "return _G.p.SpeedMode");
    assert_eq!(via_mode, "Exponential");
    run_script(&vm, "_G.p.SpeedMode = 'Linear'");
    let via_response: String = eval(&vm, "return _G.p.SpeedResponse");
    assert_eq!(via_response, "Linear");
    run_script(&vm, "_G.p.SpeedResponse = 'nonsense'");
    let kept: String = eval(&vm, "return _G.p.SpeedResponse");
    assert_eq!(kept, "Linear");
}

#[test]
fn getparent_method_matches_parent_property() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        _G.folder = Instance.new("Folder")
        _G.child = Instance.new("Part")
        _G.child.Parent = _G.folder
        "#,
    );
    let (by_prop, by_method, same): (String, String, bool) = eval(
        &vm,
        r#"return _G.child.Parent.Name, _G.child:GetParent().Name, _G.child:GetParent() == _G.child.Parent"#,
    );
    assert_eq!(by_prop, "Folder");
    assert_eq!(by_method, "Folder");
    assert!(same);
    run_script(&vm, "_G.child.Parent = nil");
    let is_nil: bool = eval(&vm, "return _G.child:GetParent() == nil and _G.child.Parent == nil");
    assert!(is_nil);
}

#[test]
fn missing_services_resolve_to_nil() {
    let mut world = test_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.p = Instance.new('Part')");
    let (ws, pl, lg, asvc): (bool, bool, bool, bool) = eval(
        &vm,
        "return _G.p.Workspace == nil, _G.p.Players == nil, _G.p.Lighting == nil, _G.p.AssetService == nil",
    );
    assert!(ws && pl && lg && asvc);
}

#[test]
fn findfirstchild_errors_without_name() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.f = Instance.new('Folder')");
    let ok: bool = eval(&vm, "return pcall(function() return _G.f:FindFirstChild(123) end)");
    assert!(!ok);
    let ok2: bool = eval(&vm, "return pcall(function() return _G.f:FindFirstChildOfClass(42) end)");
    assert!(!ok2);
    let ok3: bool = eval(&vm, "return pcall(function() return _G.f:IsA() end)");
    assert!(!ok3);
}

#[test]
fn findfirstchildofclass_recursive_and_missing() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        _G.root = Instance.new("Folder")
        _G.sub = Instance.new("Folder")
        _G.sub.Name = "Sub"
        _G.sub.Parent = _G.root
        _G.deep = Instance.new("Part")
        _G.deep.Name = "Deep"
        _G.deep.Parent = _G.sub
        "#,
    );
    let (direct_nil, recursive_found, missing_nil): (bool, bool, bool) = eval(
        &vm,
        r#"
        local r = _G.root
        return r:FindFirstChildOfClass("Part") == nil,
               r:FindFirstChildOfClass("Part", true) ~= nil,
               r:FindFirstChildOfClass("Nope", true) == nil
        "#,
    );
    assert!(direct_nil);
    assert!(recursive_found);
    assert!(missing_nil);
}

#[test]
fn cframe_lookat_points_along_negative_z() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let (lx, ly, lz): (f64, f64, f64) = eval(
        &vm,
        r#"
        local cf = CFrame.lookAt(Vector3.new(0, 0, 0), Vector3.new(0, 0, -5))
        return cf.LookVector.X, cf.LookVector.Y, cf.LookVector.Z
        "#,
    );
    assert!(lx.abs() < 1e-4 && ly.abs() < 1e-4 && (lz + 1.0).abs() < 1e-4);
    let ok: bool = eval(&vm, "return pcall(CFrame.lookAt, Vector3.new(0,0,0), 5)");
    assert!(!ok);
}

#[test]
fn cframe_new_rejects_bad_args() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let ok: bool = eval(&vm, "return pcall(CFrame.new, 'bad')");
    assert!(!ok);
    let ok2: bool = eval(&vm, "return pcall(function() return CFrame.new(1, 2) end)");
    assert!(!ok2);
}

#[test]
fn cframe_lerp_rejects_bad_args() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let ok: bool = eval(&vm, "return pcall(function() return CFrame.new().Lerp(5, 0.5) end)");
    assert!(!ok);
    let ok2: bool = eval(&vm, "return pcall(function() return CFrame.new():Lerp(CFrame.new()) end)");
    assert!(!ok2);
}

#[test]
fn cframe_multiply_vector_and_invalid() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let (x, y, z): (f64, f64, f64) = eval(
        &vm,
        "return (CFrame.new(1, 2, 3) * Vector3.new(4, 5, 6)).X, (CFrame.new(1, 2, 3) * Vector3.new(4, 5, 6)).Y, (CFrame.new(1, 2, 3) * Vector3.new(4, 5, 6)).Z",
    );
    assert_eq!((x, y, z), (5.0, 7.0, 9.0));
    let ok: bool = eval(&vm, "return pcall(function() return CFrame.new() * 'x' end)");
    assert!(!ok);
}

#[test]
fn vector3_lowercase_and_zero_unit() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let (x, y, z, mag, ux, uy, uz): (f64, f64, f64, f64, f64, f64, f64) = eval(
        &vm,
        "local v = Vector3.new(1, 2, 3) local z = Vector3.new(0, 0, 0) return v.x, v.y, v.z, z.Magnitude, z.Unit.X, z.Unit.Y, z.Unit.Z",
    );
    assert_eq!((x, y, z), (1.0, 2.0, 3.0));
    assert_eq!(mag, 0.0);
    assert_eq!((ux, uy, uz), (0.0, 0.0, 0.0));
}

#[test]
fn vector3_dot_cross_lerp_reject_bad_args() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let ok: bool = eval(&vm, "return pcall(function() return Vector3.new(1,2,3).Dot(5) end)");
    assert!(!ok);
    let ok2: bool = eval(&vm, "return pcall(function() return Vector3.new(1,2,3).Cross(nil) end)");
    assert!(!ok2);
    let ok3: bool = eval(&vm, "return pcall(function() return Vector3.new(1,2,3).Lerp(Vector3.new(0,0,0)) end)");
    assert!(!ok3);
}

#[test]
fn vector3_scalar_ops_reject_vectors() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let ok: bool = eval(&vm, "return pcall(function() return Vector3.new(1,2,3) * Vector3.new(1,1,1) end)");
    assert!(!ok);
    let ok2: bool = eval(&vm, "return pcall(function() return Vector3.new(1,2,3) - 'x' end)");
    assert!(!ok2);
    let eq_other: bool = eval(&vm, "return Vector3.new(1,2,3) == 5");
    assert!(!eq_other);
}

#[test]
fn color3_lowercase_and_fromhex_variants() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let (r, g, b, r2, hex): (f64, f64, f64, f64, String) = eval(
        &vm,
        r##"
        local c = Color3.new(0.5, 0.25, 0.125)
        local d = Color3.fromHex("ff0000")
        return c.r, c.g, c.b, d.R, d:ToHex()
        "##,
    );
    assert!((r - 0.5).abs() < 1e-5 && (g - 0.25).abs() < 1e-5);
    assert!((b - 0.125).abs() < 1e-5);
    assert!((r2 - 1.0).abs() < 0.01);
    assert_eq!(hex, "#FF0000");
}

#[test]
fn color3_fromhsv_grayscale_and_wrap() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let (r, g, b, r2, g2, b2): (f64, f64, f64, f64, f64, f64) = eval(
        &vm,
        r#"
        local gray = Color3.fromHSV(123, 0, 0.5)
        local wrapped = Color3.fromHSV(360, 1, 1)
        return gray.R, gray.G, gray.B, wrapped.R, wrapped.G, wrapped.B
        "#,
    );
    assert!((r - 0.5).abs() < 0.01 && (g - 0.5).abs() < 0.01 && (b - 0.5).abs() < 0.01);
    assert!((r2 - 1.0).abs() < 0.01 && g2.abs() < 0.01 && b2.abs() < 0.01);
}

#[test]
fn color3_equality_with_other_types() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let (eq, ne, ne2): (bool, bool, bool) = eval(
        &vm,
        "return Color3.new(1,0,0) == Color3.fromRGB(255,0,0), Color3.new(1,0,0) == Color3.new(0,1,0), Color3.new(1,0,0) == 5",
    );
    assert!(eq);
    assert!(!ne);
    assert!(!ne2);
}

#[test]
fn lighting_all_numeric_props_round_trip() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        Lighting.SunAngularRadius = 0.1
        Lighting.MoonAngularRadius = 0.2
        Lighting.PlanetRadius = 100000
        Lighting.CloudBottomHeight = 1500
        Lighting.CloudTopHeight = 3000
        Lighting.CloudBaseEdgeSoftness = 0.5
        Lighting.CloudBottomSoftness = 1.5
        Lighting.CloudBaseScale = 3.0
        Lighting.CloudDetailScale = 50.0
        Lighting.CloudShadowStepSize = 20.0
        Lighting.CloudShadowStepMultiply = 2.0
        Lighting.CloudForwardScatteringG = 0.5
        Lighting.CloudBackwardScatteringG = -0.5
        Lighting.CloudScatteringLerp = 1.5
        Lighting.CloudMinTransmittance = 0.5
        Lighting.CloudReprojectionStrength = 0.5
        "#,
    );
    let cfg = world.resource::<crate::client::sky::LightingConfig>();
    assert!((cfg.sun_angular_radius - 0.1).abs() < 1e-5);
    assert!((cfg.moon_angular_radius - 0.2).abs() < 1e-5);
    assert_eq!(cfg.planet_radius, 100000.0);
    assert_eq!(cfg.cloud_bottom_height, 1500.0);
    assert_eq!(cfg.cloud_top_height, 3000.0);
    assert_eq!(cfg.cloud_base_edge_softness, 0.5);
    assert_eq!(cfg.cloud_bottom_softness, 1.5);
    assert_eq!(cfg.cloud_base_scale, 3.0);
    assert_eq!(cfg.cloud_detail_scale, 50.0);
    assert_eq!(cfg.cloud_shadow_step_size, 20.0);
    assert_eq!(cfg.cloud_shadow_step_multiply, 2.0);
    assert_eq!(cfg.cloud_forward_scattering_g, 0.5);
    assert_eq!(cfg.cloud_backward_scattering_g, -0.5);
    assert_eq!(cfg.cloud_scattering_lerp, 1.5);
    assert_eq!(cfg.cloud_min_transmittance, 0.5);
    assert_eq!(cfg.cloud_reprojection_strength, 0.5);
    let (sun, moon, cov): (f64, f64, f64) = eval(&vm, "return Lighting.SunAngularRadius, Lighting.MoonAngularRadius, Lighting.CloudCoverage");
    assert!((sun - 0.1).abs() < 1e-5);
    assert!((moon - 0.2).abs() < 1e-5);
    assert!((cov - 0.48).abs() < 1e-5);
}

#[test]
fn lighting_clamps_out_of_range() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        Lighting.Latitude = 200
        Lighting.SunAngularRadius = 5
        Lighting.MoonAngularRadius = -1
        Lighting.StarDensity = 5
        Lighting.CloudCoverage = 5
        Lighting.CloudDensity = 0
        Lighting.CloudRaymarchSteps = 500
        Lighting.CloudShadowSteps = 500
        Lighting.CloudRenderScale = 5
        "#,
    );
    let cfg = world.resource::<crate::client::sky::LightingConfig>();
    assert_eq!(cfg.latitude, 90.0);
    assert_eq!(cfg.sun_angular_radius, 0.5);
    assert_eq!(cfg.moon_angular_radius, 0.001);
    assert_eq!(cfg.star_density, 1.0);
    assert_eq!(cfg.cloud_coverage, 1.0);
    assert_eq!(cfg.cloud_density, 0.001);
    assert_eq!(cfg.cloud_raymarch_steps, 100);
    assert_eq!(cfg.cloud_shadow_steps, 50);
    assert_eq!(cfg.cloud_render_scale, 1.0);
}

#[test]
fn lighting_timeofday_invalid_keeps_value() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let before: String = eval(&vm, "return Lighting.TimeOfDay");
    run_script(&vm, "Lighting.TimeOfDay = 'not a time'");
    let after: String = eval(&vm, "return Lighting.TimeOfDay");
    assert_eq!(before, after);
    run_script(&vm, "Lighting.TimeOfDay = '25:00:00'");
    let still: String = eval(&vm, "return Lighting.TimeOfDay");
    assert_eq!(still, after);
    run_script(&vm, "Lighting.ClockTime = 'bad'");
    let clock: f64 = eval(&vm, "return Lighting.ClockTime");
    assert_eq!(clock, 14.5);
}

#[test]
fn lighting_clocktime_wraps() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "Lighting.ClockTime = 26");
    assert!((world.resource::<crate::client::sky::LightingConfig>().time_of_day - 2.0).abs() < 1e-4);
    let fmt: String = eval(&vm, "return Lighting.TimeOfDay");
    assert_eq!(fmt, "02:00:00");
}

#[test]
fn assetservice_getmesh_creates_mesh() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.mesh = game:GetService('AssetService'):GetMesh(99)");
    let class: String = eval(&vm, "return _G.mesh.ClassName");
    assert_eq!(class, "Mesh");
    let id: String = eval(&vm, "return _G.mesh.ID");
    assert_eq!(id, "mesh/99");
    let entity = entity_of(&vm, "mesh");
    assert_eq!(
        world
            .get::<crate::common::game::assets::components::Mesh>(entity)
            .unwrap()
            .asset_id,
        99
    );
}

#[test]
fn assetservice_getsound_creates_sound() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.sound = game:GetService('AssetService'):GetSound(55)");
    let class: String = eval(&vm, "return _G.sound.ClassName");
    assert_eq!(class, "Sound");
    let id: String = eval(&vm, "return _G.sound.ID");
    assert_eq!(id, "sound/55");
    let entity = entity_of(&vm, "sound");
    let sound = world
        .get::<crate::common::game::assets::components::Sound>(entity)
        .unwrap();
    assert_eq!(sound.asset_id, 55);
    assert!(!sound.playing, "GetSound must not auto-play");
}

#[test]
fn assetservice_class_and_getservice() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let (a, b, c, d): (String, String, bool, bool) = eval(
        &vm,
        r#"return game.AssetService.ClassName, game:GetService("AssetService").ClassName, game.AssetService == game:GetService("AssetService"), AssetService == game.AssetService"#,
    );
    assert_eq!(a, "AssetService");
    assert_eq!(b, "AssetService");
    assert!(c && d);
}

#[test]
fn service_equality_all() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let (ws, pl, rs, lg, asvc): (bool, bool, bool, bool, bool) = eval(
        &vm,
        "return Workspace == Workspace, Players == Players, RunService == RunService, Lighting == Lighting, AssetService == AssetService",
    );
    assert!(ws && pl && rs && lg && asvc);
    let cross: bool = eval(&vm, "return Workspace == Players");
    assert!(!cross);
}

#[test]
fn getservice_lowercase_workspace_and_errors() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let lower: String = eval(&vm, "return game:GetService('workspace').ClassName");
    assert_eq!(lower, "Workspace");
    let ok: bool = eval(&vm, "return pcall(game.GetService, 123)");
    assert!(!ok);
}

#[test]
fn spawn_budget_blocks_when_exhausted() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    vm.lua
        .app_data_ref::<crate::scripting::vm::sandbox::SpawnBudget>()
        .unwrap()
        .per_tick
        .set(0);
    let ok: bool = eval(&vm, "return pcall(Instance.new, 'Part')");
    assert!(!ok);
    crate::scripting::vm::sandbox::reset_tick_budgets(&vm.lua);
    let ok2: bool = eval(&vm, "return pcall(Instance.new, 'Part')");
    assert!(ok2);
}

#[test]
fn connections_limit_blocks_extra() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.p = Instance.new('Part')");
    let part = entity_of(&vm, "p");
    {
        let mut reg = vm.registry.lock().unwrap();
        let mut vec = Vec::new();
        for _ in 0..crate::scripting::vm::sandbox::MAX_CONNECTIONS_PER_ENTITY {
            vec.push(std::sync::Arc::new(vm.lua.create_registry_value(vm.lua.create_function(|_, _: ()| Ok(())).unwrap()).unwrap()));
        }
        reg.connections.insert((part, "Touched"), vec);
    }
    let ok: bool = eval(&vm, "return pcall(function() _G.p.Touched:Connect(function() end) end)");
    assert!(!ok);
}

#[test]
fn pending_tasks_limit_blocks_spawn() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    {
        let sched_ref = vm.lua.app_data_ref::<crate::scripting::vm::scheduler::SchedulerRef>().unwrap();
        let mut sched = sched_ref.0.lock().unwrap();
        for _ in 0..crate::scripting::vm::sandbox::MAX_PENDING_TASKS {
            let f: LuaFunction = vm.lua.load("return 1").into_function().unwrap();
            let th = vm.lua.create_thread(f).unwrap();
            let key = vm.lua.create_registry_value(th).unwrap();
            sched.tasks.push(crate::scripting::vm::scheduler::LuaTask {
                thread_key: key,
                wake_time: Some(std::time::Instant::now() + std::time::Duration::from_secs(60)),
                callback_key: None,
                source: "fill".to_string(),
            });
        }
    }
    let ok: bool = eval(&vm, "return pcall(task.spawn, function() end)");
    assert!(!ok);
}

#[test]
fn modulescript_does_not_auto_run() {
    let mut app = script_app();
    app.world_mut().spawn((
        Name::new("M"),
        ModuleScript {
            code: "_G.module_ran = true return 1".to_string(),
        },
    ));
    app.update();
    let v: LuaValue = server_global(&app, "module_ran");
    assert_eq!(v, LuaValue::Nil);
}

#[test]
fn localscript_restarts_on_edit() {
    let mut app = script_app();
    let entity = app
        .world_mut()
        .spawn((
            Name::new("L"),
            LocalScript {
                code: "_G.lruns = (_G.lruns or 0) + 1".to_string(),
                ..default()
            },
        ))
        .id();
    app.update();
    assert_eq!(client_global::<i32>(&app, "lruns"), 1);
    app.world_mut().entity_mut(entity).insert(LocalScript {
        code: "_G.lruns = (_G.lruns or 0) + 5".to_string(),
        ..default()
    });
    app.update();
    assert_eq!(client_global::<i32>(&app, "lruns"), 6);
    app.update();
    assert_eq!(client_global::<i32>(&app, "lruns"), 6);
}

#[test]
fn disabled_localscript_does_not_run() {
    let mut app = script_app();
    app.world_mut().spawn((
        Name::new("Off"),
        LocalScript {
            code: "_G.local_should_not_run = true".to_string(),
            enabled: false,
            ..default()
        },
    ));
    app.update();
    let v: LuaValue = client_global(&app, "local_should_not_run");
    assert_eq!(v, LuaValue::Nil);
}

#[test]
fn client_heartbeat_fires() {
    let mut app = script_app();
    app.world_mut().spawn(Name::new("Workspace"));
    app.world_mut().spawn((
        Name::new("C"),
        LocalScript {
            code: "RunService.Heartbeat:Connect(function(dt) _G.chb = (_G.chb or 0) + 1 _G.cdt = dt end)".to_string(),
            ..default()
        },
    ));
    app.update();
    assert_eq!(client_global::<i32>(&app, "chb"), 1);
    assert!(client_global::<f64>(&app, "cdt") >= 0.0);
    app.update();
    assert_eq!(client_global::<i32>(&app, "chb"), 2);
}

#[test]
fn client_playeradded_fires() {
    let mut app = script_app();
    app.world_mut()
        .spawn((Name::new("Players"), PlayersServiceContainer));
    app.world_mut().spawn((
        Name::new("C"),
        LocalScript {
            code: "Players.PlayerAdded:Connect(function(p) _G.cadded = (_G.cadded or 0) + 1 _G.cname = p.Name end)".to_string(),
            ..default()
        },
    ));
    app.update();
    app.world_mut().spawn((
        Name::new("Zed"),
        crate::common::net::components::Player {
            client_id: 9,
            ..default()
        },
    ));
    app.update();
    assert_eq!(client_global::<i32>(&app, "cadded"), 1);
    assert_eq!(client_global::<String>(&app, "cname"), "Zed");
}

#[test]
fn client_touched_fires() {
    let mut app = script_app();
    app.world_mut().spawn((
        Name::new("S"),
        LocalScript {
            code: "_G.cp = Instance.new('Part') _G.ccount = 0 _G.cp.Touched:Connect(function(o) _G.ccount = _G.ccount + 1 _G.other_name = o.Name end)".to_string(),
            ..default()
        },
    ));
    app.update();
    let part: Entity = {
        let ud: LuaAnyUserData = client_global(&app, "cp");
        ud.borrow::<Instance>().unwrap().entity
    };
    let other = app.world_mut().spawn(Name::new("Other")).id();
    app.world_mut()
        .entity_mut(part)
        .insert(avian3d::prelude::CollidingEntities(
            bevy::ecs::entity::EntityHashSet::from([other]),
        ));
    app.update();
    assert_eq!(client_global::<i32>(&app, "ccount"), 1);
    assert_eq!(client_global::<String>(&app, "other_name"), "Other");
}

#[test]
fn clone_copies_local_and_module_code() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let local = world
        .spawn((
            Name::new("L"),
            LocalScript {
                code: "print('local')".to_string(),
                ..default()
            },
        ))
        .id();
    let module = world
        .spawn((
            Name::new("M"),
            ModuleScript {
                code: "return 5".to_string(),
            },
        ))
        .id();
    expose(&vm, "l", local);
    expose(&vm, "m", module);
    run_script(&vm, "_G.lc = _G.l:Clone() _G.mc = _G.m:Clone()");
    let lc: LuaAnyUserData = global(&vm, "lc");
    let mc: LuaAnyUserData = global(&vm, "mc");
    let lc_entity = lc.borrow::<Instance>().unwrap().entity;
    let mc_entity = mc.borrow::<Instance>().unwrap().entity;
    assert_eq!(world.get::<LocalScript>(lc_entity).unwrap().code, "print('local')");
    assert_eq!(world.get::<ModuleScript>(mc_entity).unwrap().code, "return 5");
    assert!(!world.get::<LocalScript>(lc_entity).unwrap().started);
}

#[test]
fn clone_of_destroyed_errors() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.p = Instance.new('Part') _G.p:Destroy()");
    let ok: bool = eval(&vm, "return pcall(function() return _G.p:Clone() end)");
    assert!(!ok);
}

#[test]
fn instance_eq_false_for_other_types() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.p = Instance.new('Part')");
    let (eq_vec, eq_num, eq_nil): (bool, bool, bool) = eval(
        &vm,
        "return _G.p == Vector3.new(0,0,0), _G.p == 5, _G.p == nil",
    );
    assert!(!eq_vec && !eq_num && !eq_nil);
}

#[test]
fn wait_without_args_resumes_next_tick() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        _G.wbefore = true
        task.wait()
        _G.wafter = true
        "#,
    );
    assert!(global::<bool>(&vm, "wbefore"));
    tick(&vm);
    assert!(global::<bool>(&vm, "wafter"));
}

#[test]
fn task_delay_zero_runs_next_tick() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "task.delay(0, function() _G.zran = true end)");
    tick(&vm);
    assert!(global::<bool>(&vm, "zran"));
}

#[test]
fn scheduler_has_work_reflects_queues() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let sched_ref = vm.lua.app_data_ref::<crate::scripting::vm::scheduler::SchedulerRef>().unwrap().0.clone();
    assert!(!crate::scripting::vm::scheduler::scheduler_has_work(&sched_ref));
    run_script(&vm, "task.defer(function() end)");
    assert!(crate::scripting::vm::scheduler::scheduler_has_work(&sched_ref));
    tick(&vm);
    assert!(!crate::scripting::vm::scheduler::scheduler_has_work(&sched_ref));
}

#[test]
fn output_push_error_splits_traceback() {
    let (msg, tb) = crate::scripting::output::split_error("oops\nstack traceback:\n\tline1");
    assert_eq!(msg, "oops");
    assert!(tb.unwrap().starts_with("stack traceback:"));
    crate::scripting::output::clear_entries();
    crate::scripting::output::push_error("Src", "plain failure".to_string());
    let buf = crate::scripting::output::buffer();
    let entries = buf.lock().unwrap();
    let found = entries.entries.iter().any(|e| e.level == crate::scripting::output::OutputLevel::Error && e.message.contains("plain failure"));
    assert!(found);
}

#[test]
fn game_workspace_aliases() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let (a, b, c, d): (bool, bool, String, String) = eval(
        &vm,
        "return workspace == Workspace, game.Workspace == Workspace, game.Workspace.ClassName, game:GetService('Workspace').Name",
    );
    assert!(a && b);
    assert_eq!(c, "Workspace");
    assert_eq!(d, "Workspace");
}

#[test]
fn instance_tostring_folder() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.f = Instance.new('Folder') _G.f.Name = 'MyFolder'");
    let s: String = eval(&vm, "return tostring(_G.f)");
    assert_eq!(s, "MyFolder");
}

#[test]
fn clone_does_not_copy_children() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        _G.parent = Instance.new("Folder")
        _G.parent.Name = "P"
        _G.kid = Instance.new("Part")
        _G.kid.Name = "Kid"
        _G.kid.Parent = _G.parent
        _G.copy = _G.parent:Clone()
        "#,
    );
    let count: usize = eval(&vm, "return #_G.copy:GetChildren()");
    assert_eq!(count, 0);
    let name: String = eval(&vm, "return _G.copy.Name");
    assert_eq!(name, "P");
}

#[test]
fn image_id_numeric_and_prefixed() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.img = Instance.new('Image') _G.img.ID = 5");
    let id: String = eval(&vm, "return _G.img.ID");
    assert_eq!(id, "image/5");
    run_script(&vm, "_G.img.ID = 'image/9'");
    let id2: String = eval(&vm, "return _G.img.ID");
    assert_eq!(id2, "image/9");
}

#[test]
fn texture_id_mesh_vs_image() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "_G.t = Instance.new('Texture') _G.t.ID = 'mesh/7'");
    let id: String = eval(&vm, "return _G.t.ID");
    assert_eq!(id, "mesh/7");
    run_script(&vm, "_G.t.ID = 11");
    let id2: String = eval(&vm, "return _G.t.ID");
    assert_eq!(id2, "image/11");
    let ok: bool = eval(&vm, "return pcall(function() _G.t.ID = 'bogus' end)");
    assert!(!ok);
}

#[test]
fn anchored_and_cancollide_default_true_for_plain_entities() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let folder = world.spawn(Name::new("F")).id();
    expose(&vm, "f", folder);
    let (anchored, collide): (bool, bool) = eval(&vm, "return _G.f.Anchored, _G.f.CanCollide");
    assert!(anchored && collide);
}

#[test]
fn velocity_defaults_zero_for_plain_entities() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let folder = world.spawn(Name::new("F")).id();
    expose(&vm, "f", folder);
    let (x, y, z): (f64, f64, f64) = eval(&vm, "return _G.f.Velocity.X, _G.f.Velocity.Y, _G.f.Velocity.Z");
    assert_eq!((x, y, z), (0.0, 0.0, 0.0));
}

#[test]
fn require_missing_module_parent_errors() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let ok: bool = eval(&vm, "return pcall(require, workspace)");
    assert!(!ok);
}

#[test]
fn task_spawn_defer_preserve_caller_budget() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        _G.order = {}
        task.defer(function() table.insert(_G.order, 'deferred') end)
        task.spawn(function() table.insert(_G.order, 'spawned') end)
        "#,
    );
    assert_eq!(global::<Vec<String>>(&vm, "order"), vec!["spawned".to_string()]);
    tick(&vm);
    assert_eq!(global::<Vec<String>>(&vm, "order"), vec!["spawned".to_string(), "deferred".to_string()]);
}

#[test]
fn touched_passes_other_part_instance() {
    let mut app = script_app();
    app.world_mut().spawn((
        Name::new("S"),
        ServerScript {
            code: "_G.a = Instance.new('Part') _G.a.Name = 'A' _G.b = Instance.new('Part') _G.b.Name = 'B' _G.got = '' _G.a.Touched:Connect(function(o) _G.got = o.Name end)".to_string(),
            ..default()
        },
    ));
    app.update();
    let (a, b): (Entity, Entity) = {
        let va: LuaAnyUserData = server_global(&app, "a");
        let vb: LuaAnyUserData = server_global(&app, "b");
        (
            va.borrow::<Instance>().unwrap().entity,
            vb.borrow::<Instance>().unwrap().entity,
        )
    };
    app.world_mut().entity_mut(a).insert(avian3d::prelude::CollidingEntities(
        bevy::ecs::entity::EntityHashSet::from([b]),
    ));
    app.update();
    assert_eq!(server_global::<String>(&app, "got"), "B");
}

#[test]
fn heartbeat_disconnect_stops_only_that_callback() {
    let mut app = script_app();
    app.world_mut().spawn(Name::new("Workspace"));
    app.world_mut().spawn((
        Name::new("C"),
        ServerScript {
            code: "_G.n1 = 0 _G.n2 = 0 local c1 = RunService.Heartbeat:Connect(function() _G.n1 = _G.n1 + 1 end) local c2 = RunService.Heartbeat:Connect(function() _G.n2 = _G.n2 + 1 end) _G.c1 = c1".to_string(),
            ..default()
        },
    ));
    app.update();
    assert_eq!(server_global::<i32>(&app, "n1"), 1);
    assert_eq!(server_global::<i32>(&app, "n2"), 1);
    let vm = app.world().resource::<ServerScriptVM>();
    run_script(vm, "_G.c1:Disconnect()");
    app.update();
    assert_eq!(server_global::<i32>(&app, "n1"), 1);
    assert_eq!(server_global::<i32>(&app, "n2"), 2);
}

#[test]
fn error_values_preserved_and_messages_clean() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    let is_table: bool = eval(&vm, "local ok, err = pcall(function() error({x = 1}) end) return type(err) == 'table'");
    assert!(is_table);
    let msg: String = eval(&vm, "local ok, err = pcall(function() error('clean boom') end) return tostring(err)");
    assert!(msg.contains("clean boom"));
    assert!(!msg.contains("stack traceback"));
}

#[test]
fn try_script_invalid_syntax_errors() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    assert!(try_script(&vm, "this is not lua").is_err());
    assert!(try_script(&vm, &"x".repeat(crate::scripting::vm::sandbox::MAX_SCRIPT_CODE_BYTES + 1)).is_err());
}

#[test]
fn advance_runs_delayed_and_waiting_tasks() {
    let mut world = full_world();
    let vm = test_vm(&mut world);
    run_script(
        &vm,
        r#"
        _G.n = 0
        task.spawn(function() task.wait(0.01) _G.n = _G.n + 1 end)
        task.delay(0.01, function() _G.n = _G.n + 10 end)
        "#,
    );
    assert_eq!(global::<i32>(&vm, "n"), 0);
    advance(&vm, 20, 2);
    assert_eq!(global::<i32>(&vm, "n"), 11);
}
