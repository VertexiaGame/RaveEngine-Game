
use bevy::prelude::*;
use crate::scripting::ecs::ModuleScript;
use crate::scripting::testing::*;
use crate::scripting::userdata::instance::Instance;

fn example_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/lua")
}

fn read_example(name: &str) -> String {
    std::fs::read_to_string(example_dir().join(name)).unwrap()
}

#[test]
fn all_top_level_example_scripts_run_without_errors() {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(example_dir())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map_or(false, |ext| ext == "lua"))
        .collect();
    files.sort();

    assert!(!files.is_empty(), "no example scripts found in scripts/lua");

    for file in files {
        let code = std::fs::read_to_string(&file).unwrap();
        let mut world = test_world();
        world.spawn(Name::new("Workspace"));
        world.spawn((
            Name::new("Players"),
            crate::common::net::components::PlayersServiceContainer,
        ));
        world.spawn((
            Name::new("Lighting"),
            crate::common::net::components::LightingServiceContainer,
        ));
        world.insert_resource(avian3d::prelude::Gravity(Vec3::NEG_Y * 196.2));
        let vm = test_vm(&mut world);

        if let Err(e) = try_script(&vm, &code) {
            panic!("example {} failed: {e}", file.display());
        }
        advance(&vm, 20, 12);

        run_script(&vm, "_G.smoke_ok = true");
        assert!(global::<bool>(&vm, "smoke_ok"), "scheduler wedged by {}", file.display());
    }
}

#[test]
fn example_module_loads_and_is_cached() {
    let module_code = read_example("09_modules/MathModule.module.lua");
    let mut world = test_world();
    let module = world
        .spawn((Name::new("MathModule"), ModuleScript { code: module_code }))
        .id();
    let vm = test_vm(&mut world);

    vm.lua
        .globals()
        .set("mod", vm.lua.create_userdata(Instance { entity: module }).unwrap())
        .unwrap();

    run_script(&vm, r#"
        local M = require(_G.mod)
        _G.add = M.add(2, 3)
        _G.double = M.double(21)
        _G.lerp = M.lerp(0, 10, 0.5)
        _G.cached = require(_G.mod) == M
        _G.global_isolated = (module_counter == nil)
    "#);
    assert_eq!(global::<f64>(&vm, "add"), 5.0);
    assert_eq!(global::<f64>(&vm, "double"), 42.0);
    assert_eq!(global::<f64>(&vm, "lerp"), 5.0);
    assert!(global::<bool>(&vm, "cached"));
    assert!(global::<bool>(&vm, "global_isolated"));
}

#[test]
fn example_module_main_script_runs_with_script_env() {
    let module_code = read_example("09_modules/MathModule.module.lua");
    let main_code = read_example("09_modules/Main.server.lua");

    let mut world = test_world();
    let folder = world.spawn(Name::new("Main")).id();
    let module = world
        .spawn((Name::new("MathModule"), ModuleScript { code: module_code }))
        .id();
    let script = world
        .spawn((Name::new("Main"), crate::scripting::ecs::ServerScript { code: main_code.clone(), ..default() }))
        .id();
    world.entity_mut(folder).add_child(module);
    world.entity_mut(folder).add_child(script);

    let vm = test_vm(&mut world);

    let func = crate::scripting::vm::compiler::compile_code(&vm.lua, &main_code, "Main").unwrap();
    let env = vm.lua.create_table().unwrap();
    let meta = vm.lua.create_table().unwrap();
    meta.set("__index", vm.lua.globals()).unwrap();
    env.set_metatable(Some(meta)).unwrap();
    env.set("script", Instance { entity: script }).unwrap();
    func.set_environment(env).unwrap();

    let thread = vm.lua.create_thread(func).unwrap();
    thread.resume::<mlua::Value>(()).unwrap();
}
