use bevy::prelude::*;
use mlua::prelude::*;

#[derive(Clone, Copy)]
pub struct PlayersService;

impl LuaUserData for PlayersService {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Eq, |_, _, other: LuaAnyUserData| {
            Ok(other.is::<PlayersService>())
        });

        methods.add_meta_method(LuaMetaMethod::Index, |lua, _, key: mlua::LuaString| {
            let world_ref = lua
                .app_data_ref::<crate::scripting::vm::server_vm::WorldRef>()
                .unwrap();
            let world = unsafe { &*world_ref.0 };

            match key.to_str()?.as_ref() {
                "ClassName" => Ok(LuaValue::String(lua.create_string("Players")?)),
                "Name" => Ok(LuaValue::String(lua.create_string("Players")?)),
                "PlayerAdded" => {
                    let entity =
                        crate::scripting::userdata::instance::find_service_entity(world, "Players")
                            .unwrap_or(Entity::PLACEHOLDER);
                    lua.create_userdata(crate::scripting::userdata::instance::RBXScriptSignal {
                        name: "PlayerAdded",
                        entity,
                    })
                    .map(LuaValue::UserData)
                }
                "GetPlayers" => Ok(LuaValue::Function(lua.create_function(
                    move |lua, _: LuaMultiValue| {
                        let world_ref = lua
                            .app_data_ref::<crate::scripting::vm::server_vm::WorldRef>()
                            .unwrap();
                        let world = unsafe { &*world_ref.0 };
                        let table = lua.create_table()?;
                        let mut i = 1;
                        for archetype in world.archetypes().iter() {
                            for entity in archetype.entities() {
                                let entity = entity.id();
                                if world
                                    .get::<crate::common::net::components::Player>(entity)
                                    .is_some()
                                {
                                    table.set(
                                        i,
                                        crate::scripting::userdata::instance::Instance { entity },
                                    )?;
                                    i += 1;
                                }
                            }
                        }
                        Ok(table)
                    },
                )?)),
                _ => {
                    if let Some(players_entity) =
                        crate::scripting::userdata::instance::find_service_entity(world, "Players")
                    {
                        let instance = crate::scripting::userdata::instance::Instance {
                            entity: players_entity,
                        };
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
