pub mod server_vm;
pub mod client_vm;
pub mod scheduler;
pub mod compiler;
pub mod sandbox;

use std::sync::{Arc, Mutex};
use mlua::prelude::*;
use mlua::{LuaOptions, StdLib};
use self::scheduler::{LuaScheduler, SchedulerRef, ScriptRegistryRef, ScriptRegistry};
use crate::scripting::runtime::require::ModuleCacheRef;
use crate::scripting::vm::sandbox::{setup_sandbox, VM_MEMORY_LIMIT};

pub struct LuaVMs {
    pub lua: Lua,
    pub scheduler: Arc<Mutex<LuaScheduler>>,
    pub registry: Arc<Mutex<ScriptRegistry>>,
}

pub fn create_vm() -> LuaVMs {
    let lua = Lua::new_with(
        StdLib::ALL_SAFE & !StdLib::OS & !StdLib::DEBUG,
        LuaOptions::default(),
    )
    .expect("failed to create sandboxed Luau VM");

    lua.set_memory_limit(VM_MEMORY_LIMIT)
        .expect("failed to set Luau memory limit idk y");

    setup_sandbox(&lua);

    let scheduler = Arc::new(Mutex::new(LuaScheduler::new()));
    lua.set_app_data(SchedulerRef(scheduler.clone()));

    let registry = Arc::new(Mutex::new(ScriptRegistry {
        connections: std::collections::HashMap::new(),
    }));
    lua.set_app_data(ScriptRegistryRef(registry.clone()));

    let module_cache = Arc::new(Mutex::new(crate::scripting::runtime::require::ModuleCache {
        cached_results: std::collections::HashMap::new(),
        loading_modules: std::collections::HashSet::new(),
    }));
    lua.set_app_data(ModuleCacheRef(module_cache));

    crate::scripting::runtime::globals::setup_globals(&lua).unwrap();
    crate::scripting::runtime::require::register_require(&lua).unwrap();
    register_game_globals(&lua).unwrap();

    LuaVMs {
        lua,
        scheduler,
        registry,
    }
}

fn register_game_globals(lua: &Lua) -> mlua::Result<()> {
    use crate::scripting::services::{asset_service::AssetService, lighting::LightingService, players::PlayersService, run_service::RunService, workspace::WorkspaceService};

    lua.globals().set("workspace", WorkspaceService)?;
    lua.globals().set("Workspace", WorkspaceService)?;
    lua.globals().set("Players", PlayersService)?;
    lua.globals().set("RunService", RunService)?;
    lua.globals().set("Lighting", LightingService)?;
    lua.globals().set("AssetService", AssetService)?;

    let game_table = lua.create_table()?;
    game_table.set("Workspace", WorkspaceService)?;
    game_table.set("workspace", WorkspaceService)?;
    game_table.set("Players", PlayersService)?;
    game_table.set("RunService", RunService)?;
    game_table.set("Lighting", LightingService)?;
    game_table.set("AssetService", AssetService)?;

    game_table.set("GetService", lua.create_function(|lua, args: LuaMultiValue| {
        let name = args
            .iter()
            .find_map(|v| match v {
                LuaValue::String(s) => Some(s.to_string_lossy()),
                _ => None,
            })
            .ok_or_else(|| mlua::Error::RuntimeError("GetService expects a service name".to_string()))?;
        match name.as_str() {
            "Workspace" | "workspace" => {
                lua.create_userdata(WorkspaceService).map(LuaValue::UserData)
            }
            "Players" => lua.create_userdata(PlayersService).map(LuaValue::UserData),
            "RunService" => lua.create_userdata(RunService).map(LuaValue::UserData),
            "Lighting" => lua.create_userdata(LightingService).map(LuaValue::UserData),
            "AssetService" => lua.create_userdata(AssetService).map(LuaValue::UserData),
            _ => Err(mlua::Error::RuntimeError(format!("Unknown service '{}'", name))),
        }
    })?)?;

    lua.globals().set("game", game_table)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::testing::{eval, test_vm, test_world};

    #[test]
    fn create_vm_registers_runtime_globals() {
        let mut world = test_world();
        let vm = test_vm(&mut world);

        for name in ["game", "workspace", "Workspace", "Players", "RunService", "Lighting", "AssetService", "task",
                     "wait", "spawn", "delay", "require", "print", "warn", "error",
                     "Vector3", "Color3", "CFrame", "Instance"] {
            assert_ne!(
                vm.lua.globals().get::<LuaValue>(name).unwrap(),
                LuaValue::Nil,
                "missing global {name}"
            );
        }
    }

    #[test]
    fn game_table_exposes_services() {
        let mut world = test_world();
        let vm = test_vm(&mut world);

        let workspace_class: String = eval(&vm, "return game.Workspace.ClassName");
        assert_eq!(workspace_class, "Workspace");
        let players_class: String = eval(&vm, "return game.Players.ClassName");
        assert_eq!(players_class, "Players");
        let run_class: String = eval(&vm, "return game.RunService.ClassName");
        assert_eq!(run_class, "RunService");
        let lighting_class: String = eval(&vm, "return game.Lighting.ClassName");
        assert_eq!(lighting_class, "Lighting");
        let asset_class: String = eval(&vm, "return game.AssetService.ClassName");
        assert_eq!(asset_class, "AssetService");
    }

    #[test]
    fn get_service_supports_dot_and_colon_and_errors_on_unknown() {
        let mut world = test_world();
        let vm = test_vm(&mut world);

        let via_dot: String = eval(&vm, "return game.GetService('Workspace').ClassName");
        assert_eq!(via_dot, "Workspace");
        let via_colon: String = eval(&vm, "return game:GetService('RunService').ClassName");
        assert_eq!(via_colon, "RunService");
        let unknown_ok: bool = eval(&vm, "return pcall(game.GetService, 'Nope')");
        assert!(!unknown_ok);
    }

    #[test]
    fn get_service_without_name_errors() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let ok: bool = eval(&vm, "return pcall(function() game:GetService() end)");
        assert!(!ok);
    }

    #[test]
    fn server_and_client_vms_are_equivalent() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let client = crate::scripting::vm::client_vm::ClientScriptVM::new();

        for lua in [&vm.lua, &client.lua] {
            let has_task = lua.globals().get::<LuaValue>("task").unwrap() != LuaValue::Nil;
            let has_game = lua.globals().get::<LuaValue>("game").unwrap() != LuaValue::Nil;
            assert!(has_task && has_game);
        }
    }

    #[test]
    fn sandboxed_vm_denies_os_and_debug_libraries() {
        let mut world = test_world();
        let vm = test_vm(&mut world);

        for lib in ["os", "debug"] {
            let present: bool = eval(&vm, &format!("return {lib} ~= nil"));
            assert!(!present, "library `{lib}` must not be exposed to scripts");
        }
    }

    #[test]
    fn sandboxed_vm_aborts_infinite_loop_and_stays_usable() {
        let mut world = test_world();
        let vm = test_vm(&mut world);

        let result: mlua::Result<LuaValue> = vm.lua.load("while true do end").eval();
        let err = result.expect_err("infinite loop must be aborted by the instruction budget");
        assert!(
            err.to_string().contains("instruction budget"),
            "unexpected error: {err}"
        );

        crate::scripting::vm::sandbox::reset_tick_budgets(&vm.lua);
        let still_works: i64 = eval(&vm, "return 2 + 2");
        assert_eq!(still_works, 4);
    }

    #[test]
    fn oversized_scripts_are_rejected_at_compile_time() {
        let mut world = test_world();
        let vm = test_vm(&mut world);

        let huge = "x".repeat(crate::scripting::vm::sandbox::MAX_SCRIPT_CODE_BYTES + 1);
        let result = crate::scripting::vm::compiler::compile_code(&vm.lua, &huge, "oversized");
        assert!(result.is_err(), "oversized script must be rejected"); //woah there
    }

    #[test]
    fn print_budget_limits_output_per_tick() {
        let mut world = test_world();
        let vm = test_vm(&mut world);

        crate::scripting::vm::sandbox::reset_tick_budgets(&vm.lua);
        let max = crate::scripting::vm::sandbox::MAX_PRINT_PER_TICK;
        let mut accepted = 0u32;
        for _ in 0..(max + 100) {
            if crate::scripting::vm::sandbox::try_print(&vm.lua) {
                accepted += 1;
            }
        }
        assert_eq!(accepted, max);
    }
}


