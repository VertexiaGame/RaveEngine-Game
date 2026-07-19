use bevy::prelude::*;
use mlua::prelude::*;
use crate::scripting::userdata::instance::RBXScriptSignal;

#[derive(Clone, Copy)]
pub struct RunService;

impl LuaUserData for RunService {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Index, |lua, _, key: String| {
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &mut *world_ref.0 };

            match key.as_str() {
                "ClassName" => Ok(LuaValue::String(lua.create_string("RunService")?)),
                "Name" => Ok(LuaValue::String(lua.create_string("RunService")?)),
                "Heartbeat" => {
                    let entity = crate::scripting::userdata::instance::find_service_entity(world, "Workspace").unwrap_or(Entity::PLACEHOLDER);
                    lua.create_userdata(RBXScriptSignal {
                        name: "Heartbeat",
                        entity,
                    }).map(LuaValue::UserData)
                }
                "Stepped" => {
                    let entity = crate::scripting::userdata::instance::find_service_entity(world, "Workspace").unwrap_or(Entity::PLACEHOLDER);
                    lua.create_userdata(RBXScriptSignal {
                        name: "Stepped",
                        entity,
                    }).map(LuaValue::UserData)
                }
                _ => {
                    if let Some(run_entity) = crate::scripting::userdata::instance::find_service_entity(world, "Workspace") {
                        let instance = crate::scripting::userdata::instance::Instance { entity: run_entity };
                        let instance_userdata = lua.create_userdata(instance)?;
                        let metatable: LuaUserDataMetatable = instance_userdata.metatable()?;
                        let index_fn: LuaFunction = metatable.get("__index")?;
                        index_fn.call::<LuaValue>((instance_userdata, key))
                    } else {
                        Ok(LuaValue::Nil)
                    }
                }
            }
        });
    }
}