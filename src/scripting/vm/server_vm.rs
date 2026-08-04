use bevy::prelude::*;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};
use super::scheduler::{LuaScheduler, SchedulerRef, ScriptRegistryRef, ScriptRegistry};
use crate::scripting::runtime::require::ModuleCacheRef;

#[derive(Resource)]
pub struct ServerScriptVM {
    pub lua: Lua,
    pub scheduler: Arc<Mutex<LuaScheduler>>,
    pub registry: Arc<Mutex<ScriptRegistry>>,
}

pub struct WorldRef(pub *mut World);
unsafe impl Send for WorldRef {}
unsafe impl Sync for WorldRef {}

impl ServerScriptVM {
    pub fn new() -> Self {
        let lua = Lua::new();

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

        lua.globals().set("workspace", crate::scripting::services::workspace::WorkspaceService).unwrap();
        lua.globals().set("Workspace", crate::scripting::services::workspace::WorkspaceService).unwrap();
        lua.globals().set("Players", crate::scripting::services::players::PlayersService).unwrap();
        lua.globals().set("RunService", crate::scripting::services::run_service::RunService).unwrap();

        let game_table = lua.create_table().unwrap();
        game_table.set("Workspace", crate::scripting::services::workspace::WorkspaceService).unwrap();
        game_table.set("workspace", crate::scripting::services::workspace::WorkspaceService).unwrap();
        game_table.set("Players", crate::scripting::services::players::PlayersService).unwrap();
        game_table.set("RunService", crate::scripting::services::run_service::RunService).unwrap();
        game_table.set("GetService", lua.create_function(|_, service_name: String| {
            Ok(service_name)
        }).unwrap()).unwrap();
        lua.globals().set("game", game_table).unwrap();

        Self {
            lua,
            scheduler,
            registry,
        }
    }
}