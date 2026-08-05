use bevy::prelude::*;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};
use super::scheduler::{LuaScheduler, ScriptRegistry};

pub struct WorldRef(pub *mut World);
unsafe impl Send for WorldRef {}
unsafe impl Sync for WorldRef {}

#[derive(Resource)]
pub struct ServerScriptVM {
    pub lua: Lua,
    pub scheduler: Arc<Mutex<LuaScheduler>>,
    pub registry: Arc<Mutex<ScriptRegistry>>,
}

impl ServerScriptVM {
    pub fn new() -> Self {
        let vm = crate::scripting::vm::create_vm();
        Self {
            lua: vm.lua,
            scheduler: vm.scheduler,
            registry: vm.registry,
        }
    }
}
