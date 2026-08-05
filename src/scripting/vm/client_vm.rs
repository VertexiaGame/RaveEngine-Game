use bevy::prelude::*;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};
use super::scheduler::{LuaScheduler, ScriptRegistry};

#[derive(Resource)]
pub struct ClientScriptVM {
    pub lua: Lua,
    pub scheduler: Arc<Mutex<LuaScheduler>>,
    pub registry: Arc<Mutex<ScriptRegistry>>,
}

impl ClientScriptVM {
    pub fn new() -> Self {
        let vm = crate::scripting::vm::create_vm();
        Self {
            lua: vm.lua,
            scheduler: vm.scheduler,
            registry: vm.registry,
        }
    }
}
