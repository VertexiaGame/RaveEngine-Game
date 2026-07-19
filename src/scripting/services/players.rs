use bevy::prelude::*;
use mlua::prelude::*;

#[derive(Clone, Copy)]
pub struct PlayersService;

impl LuaUserData for PlayersService {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Index, |lua, _, key: String| {
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &mut *world_ref.0 };

            match key.as_str() {
                "ClassName" => Ok(LuaValue::String(lua.create_string("Players")?)),
                "Name" => Ok(LuaValue::String(lua.create_string("Players")?)),
                "PlayerAdded" => {
                    let entity = crate::scripting::userdata::instance::find_service_entity(world, "Players").unwrap_or(Entity::PLACEHOLDER);
                    lua.create_userdata(crate::scripting::userdata::instance::RBXScriptSignal {
                        name: "PlayerAdded",
                        entity,
                    }).map(LuaValue::UserData)
                }
                "GetPlayers" => {
                    let mut list = Vec::new();
                    for archetype in world.archetypes().iter() {
                        for entity in archetype.entities() {
                            let entity = entity.id();
                            if world.get::<crate::common::net::components::Player>(entity).is_some() {
                                list.push(entity);
                            }
                        }
                    }
                    let table = lua.create_table()?;
                    for (i, p) in list.into_iter().enumerate() {
                        table.set(i + 1, crate::scripting::userdata::instance::Instance { entity: p })?;
                    }
                    Ok(LuaValue::Function(lua.create_function(move |_, _: LuaMultiValue| {
                        Ok(table.clone())
                    })?))
                }
                _ => {
                    if let Some(players_entity) = crate::scripting::userdata::instance::find_service_entity(world, "Players") {
                        let instance = crate::scripting::userdata::instance::Instance { entity: players_entity };
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