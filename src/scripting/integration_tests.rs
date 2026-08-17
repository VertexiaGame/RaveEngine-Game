use bevy::prelude::*;
use mlua::prelude::*;
use crate::common::net::components::{LightingServiceContainer, Player, PlayersServiceContainer};
use crate::scripting::ecs::ServerScript;
use crate::scripting::plugin::ScriptingPlugin;
use crate::scripting::testing::*;
use crate::scripting::userdata::instance::Instance;
use crate::scripting::vm::server_vm::ServerScriptVM;

fn service_world() -> World {
    let mut world = test_world();
    world.spawn(Name::new("Workspace"));
    world.spawn((Name::new("Players"), PlayersServiceContainer));
    world.spawn((Name::new("Lighting"), LightingServiceContainer));
    world.insert_resource(avian3d::prelude::Gravity(Vec3::NEG_Y * 196.2));
    world
}

fn script_app() -> App {
    let mut app = App::new();
    app.add_plugins(ScriptingPlugin);
    app.world_mut().insert_resource(bevy::time::Time::<()>::default());
    app.world_mut().insert_resource(ServerScriptVM::new());
    app.world_mut().insert_resource(crate::scripting::vm::client_vm::ClientScriptVM::new());
    app
}

fn app_script(app: &App, code: &str) {
    let vm = app.world().resource::<ServerScriptVM>();
    run_script(vm, code);
}

fn app_global<T: FromLua>(app: &App, name: &str) -> T {
    app.world().resource::<ServerScriptVM>().lua.globals().get(name).unwrap()
}

fn app_entity(app: &App, global_name: &str) -> Entity {
    entity_of(app.world().resource::<ServerScriptVM>(), global_name)
}

#[test]
fn players_service_lists_all_players_as_instances() {
    let mut world = service_world();
    let alice = world
        .spawn((Name::new("Alice"), Player { client_id: 1, username: "Alice".to_string(), ..default() }))
        .id();
    let bob = world
        .spawn((Name::new("Bob"), Player { client_id: 2, username: "Bob".to_string(), ..default() }))
        .id();
    let vm = test_vm(&mut world);
    vm.lua.globals().set("alice", vm.lua.create_userdata(Instance { entity: alice }).unwrap()).unwrap();
    vm.lua.globals().set("bob", vm.lua.create_userdata(Instance { entity: bob }).unwrap()).unwrap();

    let (count, names, eq, class): (usize, Vec<String>, bool, String) = eval(
        &vm,
        r#"
            local ps = Players:GetPlayers()
            local names = {}
            local matches = false
            for i, p in ipairs(ps) do
                names[i] = p.Name
                if p == _G.alice or p == _G.bob then matches = true end
            end
            return #ps, names, matches, ps[1].ClassName
        "#,
    );
    assert_eq!(count, 2);
    assert!(names.contains(&"Alice".to_string()));
    assert!(names.contains(&"Bob".to_string()));
    assert!(eq);
    assert_eq!(class, "Player");
}

#[test]
fn getplayers_is_empty_without_players() {
    let mut world = service_world();
    let vm = test_vm(&mut world);
    let count: usize = eval(&vm, "return #Players:GetPlayers()");
    assert_eq!(count, 0);
}

#[test]
fn workspace_gravity_round_trips_through_lua() {
    let mut world = service_world();
    let vm = test_vm(&mut world);

    let read: f64 = eval(&vm, "return workspace.Gravity");
    assert!((read - 700.71).abs() < 0.001, "expected 700.71, got {read}");

    run_script(&vm, "workspace.Gravity = 500");
    let g = world.resource::<avian3d::prelude::Gravity>();
    assert!((g.0.y - (-140.0)).abs() < 1e-6);
}

#[test]
fn service_aliases_and_getservice_are_equivalent() {
    let mut world = service_world();
    let vm = test_vm(&mut world);

    let (ws, game_ws, players, lighting): (bool, bool, bool, bool) = eval(
        &vm,
        r#"
            return workspace == Workspace,
                   game.Workspace == workspace,
                   game:GetService("Players") == Players,
                   game:GetService("Lighting") == Lighting
        "#,
    );
    assert!(ws && game_ws && players && lighting);
}

#[test]
fn script_environment_exposes_script_instance() {
    let mut world = service_world();
    let folder = world.spawn(Name::new("Folder")).id();
    let script = world
        .spawn((Name::new("MyScript"), ServerScript::default()))
        .id();
    world.entity_mut(folder).add_child(script);

    let vm = test_vm(&mut world);
    let env = vm.lua.create_table().unwrap();
    let meta = vm.lua.create_table().unwrap();
    meta.set("__index", vm.lua.globals()).unwrap();
    env.set_metatable(Some(meta)).unwrap();
    env.set("script", Instance { entity: script }).unwrap();

    let func = crate::scripting::vm::compiler::compile_code(
        &vm.lua,
        r#"
            _G.script_name = script.Name
            _G.script_class = script.ClassName
            _G.script_parent = script.Parent.Name
            _G.script_tostring = tostring(script)
        "#,
        "EnvTest",
    )
    .unwrap();
    func.set_environment(env).unwrap();
    func.call::<()>(()).unwrap();

    assert_eq!(global::<String>(&vm, "script_name"), "MyScript");
    assert_eq!(global::<String>(&vm, "script_class"), "Script");
    assert_eq!(global::<String>(&vm, "script_parent"), "Folder");
    assert_eq!(global::<String>(&vm, "script_tostring"), "MyScript");
}

#[test]
fn workspace_children_include_parts_scripts_and_folders() {
    let mut world = service_world();
    world.spawn((Name::new("Baseplate"), crate::common::game::bricks::components::Brick));
    world.spawn((Name::new("Loop"), ServerScript::default()));
    world.spawn((Name::new("Init"), crate::scripting::ecs::ModuleScript::default()));
    world.spawn(Name::new("IgnoredBystander"));

    let vm = test_vm(&mut world);
    let (children, descendants, classes): (usize, usize, Vec<String>) = eval(
        &vm,
        r#"
            local children = workspace:GetChildren()
            local classes = {}
            for i, c in ipairs(children) do classes[i] = c.ClassName end
            return #children, #workspace:GetDescendants(), classes
        "#,
    );
    assert_eq!(children, 3);
    assert_eq!(descendants, 3);
    assert!(classes.contains(&"Part".to_string()));
    assert!(classes.contains(&"Script".to_string()));
    assert!(classes.contains(&"ModuleScript".to_string()));
}

#[test]
fn part_velocity_reads_zero_without_physics() {
    let mut world = service_world();
    let vm = test_vm(&mut world);
    run_script(&vm, r#"
        _G.p = Instance.new("Part")
        local v = _G.p.Velocity
        _G.vx, _G.vy, _G.vz = v.X, v.Y, v.Z
    "#);
    let (x, y, z): (f64, f64, f64) = (global(&vm, "vx"), global(&vm, "vy"), global(&vm, "vz"));
    assert_eq!((x, y, z), (0.0, 0.0, 0.0));
}

#[test]
fn yielding_heartbeat_callback_does_not_accumulate_threads() {
    let mut app = script_app();
    app.world_mut().spawn(Name::new("Workspace"));
    app.world_mut().spawn((
        Name::new("Connector"),
        ServerScript {
            code: r#"
                RunService.Heartbeat:Connect(function(dt)
                    while true do
                        _G.hb = (_G.hb or 0) + 1
                        task.wait(0.02)
                    end
                end)
            "#.to_string(),
            ..default()
        },
    ));

    app.update();
    assert_eq!(app_global::<i32>(&app, "hb"), 1);

    for _ in 0..3 {
        app.update();
    }
    let pending = {
        let vm = app.world().resource::<ServerScriptVM>();
        let sched = vm.scheduler.lock().unwrap();
        sched.tasks.len() + sched.deferred.len()
    };
    assert_eq!(pending, 1, "yielding callback must not spawn duplicate threads");

    std::thread::sleep(std::time::Duration::from_millis(30));
    app.update();
    assert_eq!(app_global::<i32>(&app, "hb"), 2);
    let pending = {
        let vm = app.world().resource::<ServerScriptVM>();
        let sched = vm.scheduler.lock().unwrap();
        sched.tasks.len() + sched.deferred.len()
    };
    assert_eq!(pending, 1, "callback must stay queued, not duplicate");

    std::thread::sleep(std::time::Duration::from_millis(30));
    app.update();
    assert_eq!(app_global::<i32>(&app, "hb"), 3);
    let pending = {
        let vm = app.world().resource::<ServerScriptVM>();
        let sched = vm.scheduler.lock().unwrap();
        sched.tasks.len() + sched.deferred.len()
    };
    assert_eq!(pending, 1);
}

#[test]
fn heartbeat_and_stepped_fire_with_delta_and_disconnect_stops_one() {
    let mut app = script_app();
    app.world_mut().spawn(Name::new("Workspace"));
    app.world_mut().spawn((
        Name::new("Connector"),
        ServerScript {
            code: r#"
                local hb = RunService.Heartbeat:Connect(function(dt)
                    _G.hb = (_G.hb or 0) + 1
                    _G.last_dt = dt
                end)
                local st = RunService.Stepped:Connect(function(dt)
                    _G.st = (_G.st or 0) + 1
                end)
                _G.hb_conn = hb
                _G.st_conn = st
            "#.to_string(),
            ..default()
        },
    ));

    app.update();
    assert_eq!(app_global::<i32>(&app, "hb"), 1);
    assert_eq!(app_global::<i32>(&app, "st"), 1);
    assert!(app_global::<f64>(&app, "last_dt") >= 0.0);

    app_script(&app, "_G.st_conn:Disconnect()");
    app.update();
    assert_eq!(app_global::<i32>(&app, "hb"), 2);
    assert_eq!(app_global::<i32>(&app, "st"), 1, "disconnected callback must not fire");
}

#[test]
fn player_added_fires_once_per_joining_player() {
    let mut app = script_app();
    app.world_mut().spawn((Name::new("Players"), PlayersServiceContainer));
    app.world_mut().spawn((
        Name::new("Connector"),
        ServerScript {
            code: r#"
                Players.PlayerAdded:Connect(function(p)
                    _G.added = (_G.added or 0) + 1
                    _G.last = p.Name
                end)
            "#.to_string(),
            ..default()
        },
    ));
    app.update();

    app.world_mut().spawn((Name::new("A"), Player { client_id: 1, ..default() }));
    app.world_mut().spawn((Name::new("B"), Player { client_id: 2, ..default() }));
    app.update();
    assert_eq!(app_global::<i32>(&app, "added"), 2);
    assert_eq!(app_global::<String>(&app, "last"), "B");

    app.update();
    assert_eq!(app_global::<i32>(&app, "added"), 2);
}

#[test]
fn touched_disconnect_prevents_future_events() {
    let mut app = script_app();
    app.world_mut().spawn((
        Name::new("Sensor"),
        ServerScript {
            code: r#"
                _G.p = Instance.new("Part")
                _G.conn = _G.p.Touched:Connect(function()
                    _G.count = (_G.count or 0) + 1
                end)
            "#.to_string(),
            ..default()
        },
    ));
    app.update();
    let part = app_entity(&app, "p");

    app.world_mut().entity_mut(part)
        .insert(avian3d::prelude::CollidingEntities(bevy::ecs::entity::EntityHashSet::from([Entity::PLACEHOLDER])));
    app.update();
    assert_eq!(app_global::<i32>(&app, "count"), 1);

    app_script(&app, "_G.conn:Disconnect()");
    app.world_mut().entity_mut(part).remove::<avian3d::prelude::CollidingEntities>();
    app.world_mut().entity_mut(part)
        .insert(avian3d::prelude::CollidingEntities(bevy::ecs::entity::EntityHashSet::from([Entity::PLACEHOLDER])));
    app.update();
    assert_eq!(app_global::<i32>(&app, "count"), 1, "disconnected signal must not fire");
}

#[test]
fn error_in_spawned_task_reaches_output_buffer() {
    crate::scripting::output::clear_entries();
    let mut world = service_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "task.spawn(function() error('boom in task') end)");

    let buffer = crate::scripting::output::buffer();
    let entries = buffer.lock().unwrap();
    let found = entries.entries.iter().any(|e| {
        e.level == crate::scripting::output::OutputLevel::Error
            && e.message.contains("boom in task")
    });
    assert!(found, "expected an Error entry for the failed task");
}

#[test]
fn print_and_warn_route_to_output_buffer_with_levels() {
    crate::scripting::output::clear_entries();
    let mut world = service_world();
    let vm = test_vm(&mut world);
    run_script(&vm, "print('hello', 42)\nwarn('careful now')");

    let buffer = crate::scripting::output::buffer();
    let entries = buffer.lock().unwrap();
    let has_info = entries.entries.iter().any(|e| {
        e.level == crate::scripting::output::OutputLevel::Info && e.message.contains("hello")
    });
    let has_warn = entries.entries.iter().any(|e| {
        e.level == crate::scripting::output::OutputLevel::Warn && e.message.contains("careful now")
    });
    assert!(has_info, "print must produce an Info entry");
    assert!(has_warn, "warn must produce a Warn entry");
}

#[test]
fn lighting_reads_return_defaults_without_config_resource() {
    let mut world = service_world();
    let vm = test_vm(&mut world);
    let (clock, latitude, clouds, write_ok, class): (f64, f64, bool, bool, String) = eval(
        &vm,
        r#"
            return Lighting.ClockTime, Lighting.Latitude, Lighting.VolumetricClouds,
                   pcall(function() Lighting.ClockTime = 8 end),
                   Lighting.ClassName
        "#,
    );
    assert_eq!(clock, 14.5);
    assert_eq!(latitude, 45.0);
    assert!(clouds);
    assert!(write_ok);
    assert_eq!(class, "Lighting");
}

#[test]
fn require_caches_across_script_invocations_in_one_vm() {
    let mut world = service_world();
    let module = world
        .spawn((Name::new("M"), crate::scripting::ecs::ModuleScript {
            code: "return { value = 7 }".to_string(),
        }))
        .id();
    let vm = test_vm(&mut world);
    vm.lua.globals().set("mod", vm.lua.create_userdata(Instance { entity: module }).unwrap()).unwrap();

    run_script(&vm, "_G.first = require(_G.mod)");
    run_script(&vm, "_G.second = require(_G.mod)");
    let same: bool = eval(&vm, "return _G.first == _G.second");
    assert!(same);
    let value: i32 = eval(&vm, "return _G.first.value");
    assert_eq!(value, 7);
}

#[test]
fn task_composition_runs_spawn_wait_and_delay() {
    let mut world = service_world();
    let vm = test_vm(&mut world);
    run_script(&vm, r#"
        _G.ticks = 0
        task.spawn(function()
            for _ = 1, 3 do
                _G.ticks = _G.ticks + 1
                task.wait(0.01)
            end
        end)
        task.delay(0.02, function() _G.delayed = true end)
    "#);
    assert_eq!(global::<i32>(&vm, "ticks"), 1);
    advance(&vm, 15, 6);
    assert_eq!(global::<i32>(&vm, "ticks"), 3);
    assert!(global::<bool>(&vm, "delayed"));
}

#[test]
fn destroy_removes_instance_from_workspace_children() {
    let mut world = service_world();
    let vm = test_vm(&mut world);
    run_script(&vm, r#"
        _G.p = Instance.new("Part")
        _G.before = #workspace:GetChildren()
        _G.p:Destroy()
        _G.after = #workspace:GetChildren()
    "#);
    let before: usize = global(&vm, "before");
    let after: usize = global(&vm, "after");
    assert_eq!(after, before - 1);
}

#[test]
fn world_reflection_finds_players_by_class_across_services() {
    let mut world = test_world();
    let workspace = world.spawn(Name::new("Workspace")).id();
    let alice = world
        .spawn((Name::new("Alice"), Player { client_id: 1, ..default() }))
        .id();
    world.entity_mut(workspace).add_child(alice);
    let vm = test_vm(&mut world);

    let found: bool = eval(&vm, r#"
        local ps = workspace:GetDescendants()
        for _, p in ipairs(ps) do
            if p.ClassName == "Player" and p.Name == "Alice" then return true end
        end
        return false
    "#);
    assert!(found);
}
