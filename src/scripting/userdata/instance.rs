use bevy::prelude::*;
use mlua::prelude::*;
use crate::common::game::bricks::components::{Brick, BrickColor, BrickPhysics};
use super::vector3::Vector3;
use super::color3::Color3;
use super::cframe::CFrame;
use crate::scripting::vm::scheduler::ScriptRegistryRef;
use avian3d::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Instance {
    pub entity: Entity,
}

impl LuaUserData for Instance {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaAnyUserData| {
            if let Ok(other_inst) = other.borrow::<Instance>() {
                Ok(this.entity == other_inst.entity)
            } else {
                Ok(false)
            }
        });

        methods.add_meta_method(LuaMetaMethod::ToString, |lua, this, _: ()| {
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &mut *world_ref.0 };
            let name = world.get::<Name>(this.entity).map(|n| n.as_str().to_string()).unwrap_or_else(|| "Instance".to_string());
            Ok(name)
        });

        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &mut *world_ref.0 };

            if world.get_entity(this.entity).is_err() {
                return Err(mlua::Error::RuntimeError("Instance has been destroyed".to_string()));
            }

            match key.as_str() {
                "Name" => {
                    let name = world.get::<Name>(this.entity).map(|n| n.as_str().to_string()).unwrap_or_default();
                    Ok(LuaValue::String(lua.create_string(&name)?))
                }
                "ClassName" => {
                    let name = world.get::<Name>(this.entity).map(|n| n.as_str()).unwrap_or("");
                    let is_brick = world.get::<Brick>(this.entity).is_some();
                    let is_player = world.get::<crate::common::net::components::Player>(this.entity).is_some();
                    let is_players_service = world.get::<crate::common::net::components::PlayersServiceContainer>(this.entity).is_some();
                    let is_lighting_service = world.get::<crate::common::net::components::LightingServiceContainer>(this.entity).is_some();

                    let class_name = if name == "Workspace" {
                        "Workspace"
                    } else if is_players_service {
                        "Players"
                    } else if is_lighting_service {
                        "Lighting"
                    } else if is_player {
                        "Player"
                    } else if is_brick {
                        "Part"
                    } else if world.get::<crate::scripting::ecs::ServerScript>(this.entity).is_some() {
                        "Script"
                    } else if world.get::<crate::scripting::ecs::LocalScript>(this.entity).is_some() {
                        "LocalScript"
                    } else if world.get::<crate::scripting::ecs::ModuleScript>(this.entity).is_some() {
                        "ModuleScript"
                    } else {
                        "Folder"
                    };
                    Ok(LuaValue::String(lua.create_string(class_name)?))
                }
                "Position" => {
                    let translation = world.get::<Transform>(this.entity).map(|t| t.translation).unwrap_or_default();
                    lua.create_userdata(Vector3(translation / 0.28)).map(LuaValue::UserData)
                }
                "Size" => {
                    let scale = world.get::<Transform>(this.entity).map(|t| t.scale).unwrap_or(Vec3::ONE);
                    lua.create_userdata(Vector3(scale)).map(LuaValue::UserData)
                }
                "CFrame" => {
                    let transform = world.get::<Transform>(this.entity).cloned().unwrap_or_default();
                    lua.create_userdata(CFrame {
                        position: transform.translation / 0.28,
                        rotation: transform.rotation,
                    }).map(LuaValue::UserData)
                }
                "Parent" => {
                    if let Some(child_of) = world.get::<ChildOf>(this.entity) {
                        lua.create_userdata(Instance { entity: child_of.parent() }).map(LuaValue::UserData)
                    } else {
                        Ok(LuaValue::Nil)
                    }
                }
                "Color" | "BrickColor" => {
                    let color = world.get::<BrickColor>(this.entity).map(|bc| bc.color).unwrap_or(Color::WHITE);
                    lua.create_userdata(Color3(color)).map(LuaValue::UserData)
                }
                "Anchored" => {
                    let phys = world.get::<BrickPhysics>(this.entity);
                    let anchored = phys.map_or(true, |p| !p.enabled);
                    Ok(LuaValue::Boolean(anchored))
                }
                "CanCollide" => {
                    let phys = world.get::<BrickPhysics>(this.entity);
                    let can_collide = phys.map_or(true, |p| p.player_can_collide);
                    Ok(LuaValue::Boolean(can_collide))
                }
                "Touched" => {
                    lua.create_userdata(crate::scripting::userdata::instance::RBXScriptSignal {
                        name: "Touched",
                        entity: this.entity,
                    }).map(LuaValue::UserData)
                }
                "JumpPower" => {
                    let jp = world.get::<crate::common::net::components::Player>(this.entity)
                        .map(|p| p.jump_power / 0.28)
                        .unwrap_or(50.0);
                    Ok(LuaValue::Number(jp as f64))
                }
                "Speed" => {
                    let s = world.get::<crate::common::net::components::Player>(this.entity)
                        .map(|p| p.speed / 0.28)
                        .unwrap_or(16.0);
                    Ok(LuaValue::Number(s as f64))
                }
                "Velocity" => {
                    let vel = world.get::<LinearVelocity>(this.entity)
                        .map(|v| v.0 / 0.28)
                        .unwrap_or(Vec3::ZERO);
                    lua.create_userdata(Vector3(vel)).map(LuaValue::UserData)
                }
                "Workspace" => {
                    if let Some(workspace_entity) = find_service_entity(world, "Workspace") {
                        lua.create_userdata(Instance { entity: workspace_entity }).map(LuaValue::UserData)
                    } else {
                        Ok(LuaValue::Nil)
                    }
                }
                "Players" => {
                    if let Some(players_entity) = find_service_entity(world, "Players") {
                        lua.create_userdata(Instance { entity: players_entity }).map(LuaValue::UserData)
                    } else {
                        Ok(LuaValue::Nil)
                    }
                }
                "Lighting" => {
                    if let Some(lighting_entity) = find_service_entity(world, "Lighting") {
                        lua.create_userdata(Instance { entity: lighting_entity }).map(LuaValue::UserData)
                    } else {
                        Ok(LuaValue::Nil)
                    }
                }
                "GetChildren" => {
                    let mut children_list = Vec::new();
                    let is_workspace = world.get::<Name>(this.entity).map_or(false, |n| n.as_str() == "Workspace");
                    if is_workspace {
                        if let Some(children) = world.get::<Children>(this.entity) {
                            children_list.extend(children.to_vec());
                        }
                        for archetype in world.archetypes().iter() {
                            for entity in archetype.entities() {
                                let entity = entity.id();
                                if world.get::<ChildOf>(entity).is_none() && entity != this.entity {
                                    let is_managed = world.get::<Brick>(entity).is_some()
                                        || world.get::<crate::scripting::ecs::ServerScript>(entity).is_some()
                                        || world.get::<crate::scripting::ecs::LocalScript>(entity).is_some()
                                        || world.get::<crate::scripting::ecs::ModuleScript>(entity).is_some();
                                    if is_managed {
                                        children_list.push(entity);
                                    }
                                }
                            }
                        }
                    } else {
                        if let Some(children) = world.get::<Children>(this.entity) {
                            children_list.extend(children.to_vec());
                        }
                    }

                    let table = lua.create_table()?;
                    for (i, child) in children_list.into_iter().enumerate() {
                        table.set(i + 1, Instance { entity: child })?;
                    }
                    Ok(LuaValue::Function(lua.create_function(move |_, _: LuaMultiValue| {
                        Ok(table.clone())
                    })?))
                }
                "GetParent" => {
                    let parent_opt = world.get::<ChildOf>(this.entity).map(|co| co.parent());
                    Ok(LuaValue::Function(lua.create_function(move |lua, _: LuaMultiValue| {
                        if let Some(parent) = parent_opt {
                            lua.create_userdata(Instance { entity: parent }).map(LuaValue::UserData)
                        } else {
                            Ok(LuaValue::Nil)
                        }
                    })?))
                }
                "FindFirstChild" => {
                    let mut children_list = Vec::new();
                    let is_workspace = world.get::<Name>(this.entity).map_or(false, |n| n.as_str() == "Workspace");
                    if is_workspace {
                        if let Some(children) = world.get::<Children>(this.entity) {
                            children_list.extend(children.to_vec());
                        }
                        for archetype in world.archetypes().iter() {
                            for entity in archetype.entities() {
                                let entity = entity.id();
                                if world.get::<ChildOf>(entity).is_none() && entity != this.entity {
                                    let is_managed = world.get::<Brick>(entity).is_some()
                                        || world.get::<crate::scripting::ecs::ServerScript>(entity).is_some()
                                        || world.get::<crate::scripting::ecs::LocalScript>(entity).is_some()
                                        || world.get::<crate::scripting::ecs::ModuleScript>(entity).is_some();
                                    if is_managed {
                                        children_list.push(entity);
                                    }
                                }
                            }
                        }
                    } else {
                        if let Some(children) = world.get::<Children>(this.entity) {
                            children_list.extend(children.to_vec());
                        }
                    }

                    Ok(LuaValue::Function(lua.create_function(move |lua, args: LuaMultiValue| {
                        let mut name_to_find = String::new();
                        if args.len() == 1 {
                            if let Some(LuaValue::String(s)) = args.get(0) {
                                name_to_find = s.to_str()?.to_string();
                            }
                        } else if args.len() >= 2 {
                            if let Some(LuaValue::String(s)) = args.get(1) {
                                name_to_find = s.to_str()?.to_string();
                            }
                        }
                        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
                        let world = unsafe { &*world_ref.0 };
                        for child in &children_list {
                            if let Some(child_name) = world.get::<Name>(*child) {
                                if child_name.as_str() == name_to_find {
                                    return lua.create_userdata(Instance { entity: *child }).map(LuaValue::UserData);
                                }
                            }
                        }
                        Ok(LuaValue::Nil)
                    })?))
                }
                "Clone" => {
                    let entity = this.entity;
                    Ok(LuaValue::Function(lua.create_function(move |lua, _: LuaMultiValue| {
                        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
                        let world = unsafe { &mut *world_ref.0 };
                        if world.get_entity(entity).is_err() {
                            return Err(mlua::Error::RuntimeError("Instance to clone has been destroyed".to_string()));
                        }
                        let (transform, name, shape, phys, color, layers) = {
                            let transform = world.get::<Transform>(entity).cloned().unwrap_or_default();
                            let name = world.get::<Name>(entity).cloned().unwrap_or_else(|| Name::new("Clone"));
                            let shape = world.get::<crate::common::game::bricks::components::BrickShapeComponent>(entity).cloned();
                            let phys = world.get::<BrickPhysics>(entity).cloned();
                            let color = world.get::<BrickColor>(entity).cloned();
                            let layers = world.get::<avian3d::prelude::CollisionLayers>(entity).cloned();
                            (transform, name, shape, phys, color, layers)
                        };
                        let mut new_entity = world.spawn((transform, name));
                        if let Some(s) = shape { new_entity.insert(s); }
                        if let Some(p) = phys { new_entity.insert(p); }
                        if let Some(c) = color { new_entity.insert(c); }
                        if let Some(l) = layers { new_entity.insert(l); }
                        new_entity.insert(lightyear::prelude::Replicate::default());
                        let new_id = new_entity.id();
                        lua.create_userdata(Instance { entity: new_id }).map(LuaValue::UserData)
                    })?))
                }
                "Destroy" => {
                    let entity = this.entity;
                    Ok(LuaValue::Function(lua.create_function(move |lua, _: LuaMultiValue| {
                        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
                        let world = unsafe { &mut *world_ref.0 };
                        if world.get_entity(entity).is_ok() {
                            world.entity_mut(entity).despawn();
                        }
                        Ok(())
                    })?))
                }
                _ => Ok(LuaValue::Nil),
            }
        });

        methods.add_meta_method(LuaMetaMethod::NewIndex, |lua, this, (key, value): (String, LuaValue)| {
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &mut *world_ref.0 };

            if world.get_entity(this.entity).is_err() {
                return Err(mlua::Error::RuntimeError("Instance has been destroyed".to_string()));
            }

            match key.as_str() {
                "Name" => {
                    if let LuaValue::String(s) = value {
                        let s_str = s.to_str()?.to_string();
                        world.entity_mut(this.entity).insert(Name::new(s_str));
                    }
                }
                "Position" => {
                    if let LuaValue::UserData(ud) = value {
                        if let Ok(vec) = ud.borrow::<Vector3>() {
                            if let Some(mut transform) = world.get_mut::<Transform>(this.entity) {
                                transform.translation = vec.0 * 0.28;
                            }
                        }
                    }
                }
                "Size" => {
                    if let LuaValue::UserData(ud) = value {
                        if let Ok(vec) = ud.borrow::<Vector3>() {
                            if let Some(mut transform) = world.get_mut::<Transform>(this.entity) {
                                transform.scale = vec.0;
                            }
                        }
                    }
                }
                "CFrame" => {
                    if let LuaValue::UserData(ud) = value {
                        if let Ok(cf) = ud.borrow::<CFrame>() {
                            if let Some(mut transform) = world.get_mut::<Transform>(this.entity) {
                                transform.translation = cf.position * 0.28;
                                transform.rotation = cf.rotation;
                            }
                        }
                    }
                }
                "Parent" => {
                    match value {
                        LuaValue::UserData(ud) => {
                            if let Ok(parent_inst) = ud.borrow::<Instance>() {
                                world.entity_mut(parent_inst.entity).add_child(this.entity);
                            }
                        }
                        LuaValue::Nil => {
                            world.entity_mut(this.entity).remove::<ChildOf>();
                        }
                        _ => {}
                    }
                }
                "Color" | "BrickColor" => {
                    if let LuaValue::UserData(ud) = value {
                        if let Ok(col) = ud.borrow::<Color3>() {
                            if let Some(mut bc) = world.get_mut::<BrickColor>(this.entity) {
                                bc.color = col.0;
                            } else {
                                world.entity_mut(this.entity).insert(BrickColor { color: col.0 });
                            }
                        }
                    }
                }
                "Anchored" => {
                    if let LuaValue::Boolean(b) = value {
                        if let Some(mut phys) = world.get_mut::<BrickPhysics>(this.entity) {
                            phys.enabled = !b;
                        } else {
                            world.entity_mut(this.entity).insert(BrickPhysics {
                                enabled: !b,
                                ..default()
                            });
                        }
                        let is_enabled = world.get::<BrickPhysics>(this.entity).map_or(true, |p| p.enabled);
                        if is_enabled {
                            world.entity_mut(this.entity).insert(RigidBody::Dynamic);
                        } else {
                            world.entity_mut(this.entity).insert(RigidBody::Static);
                        }
                    }
                }
                "CanCollide" => {
                    if let LuaValue::Boolean(b) = value {
                        if let Some(mut phys) = world.get_mut::<BrickPhysics>(this.entity) {
                            phys.player_can_collide = b;
                        } else {
                            world.entity_mut(this.entity).insert(BrickPhysics {
                                player_can_collide: b,
                                ..default()
                            });
                        }
                        let player_can_collide = world.get::<BrickPhysics>(this.entity).map_or(true, |p| p.player_can_collide);
                        let layers = if player_can_collide {
                            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                        } else {
                            CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                        };
                        world.entity_mut(this.entity).insert(layers);
                    }
                }
                "JumpPower" => {
                    let opt_val = match value {
                        LuaValue::Number(n) => Some(n),
                        LuaValue::Integer(i) => Some(i as f64),
                        _ => None,
                    };
                    if let Some(val) = opt_val {
                        if let Some(mut player) = world.get_mut::<crate::common::net::components::Player>(this.entity) {
                            player.jump_power = val as f32 * 0.28;
                        }
                    }
                }
                "Speed" => {
                    let opt_val = match value {
                        LuaValue::Number(n) => Some(n),
                        LuaValue::Integer(i) => Some(i as f64),
                        _ => None,
                    };
                    if let Some(val) = opt_val {
                        if let Some(mut player) = world.get_mut::<crate::common::net::components::Player>(this.entity) {
                            player.speed = val as f32 * 0.28;
                        }
                    }
                }
                "Velocity" => {
                    if let LuaValue::UserData(ud) = value {
                        if let Ok(vec) = ud.borrow::<Vector3>() {
                            if let Some(mut vel) = world.get_mut::<LinearVelocity>(this.entity) {
                                vel.0 = vec.0 * 0.28;
                            } else {
                                world.entity_mut(this.entity).insert(LinearVelocity(vec.0 * 0.28));
                            }
                        }
                    }
                }
                "Gravity" => {
                    let opt_val = match value {
                        LuaValue::Number(n) => Some(n),
                        LuaValue::Integer(i) => Some(i as f64),
                        _ => None,
                    };
                    if let Some(val) = opt_val {
                        if let Some(mut g) = world.get_resource_mut::<avian3d::prelude::Gravity>() {
                            g.0.y = -val as f32 * 0.28;
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        });
    }
}

pub fn find_service_entity(world: &World, service_name: &str) -> Option<Entity> {
    if let Some(cache) = world.get_resource::<crate::scripting::vm::scheduler::ServiceEntities>() {
        let cached = match service_name {
            "Workspace" => cache.workspace,
            "Players" => cache.players,
            "Lighting" => cache.lighting,
            _ => None,
        };
        if cached.is_some() {
            return cached;
        }
    }
    for archetype in world.archetypes().iter() {
        for entity in archetype.entities() {
            let entity = entity.id();
            if let Some(name) = world.get::<Name>(entity) {
                if name.as_str() == "Workspace" && service_name == "Workspace" {
                    return Some(entity);
                }
                if name.as_str() == "Players" && service_name == "Players" {
                    return Some(entity);
                }
                if name.as_str() == "Lighting" && service_name == "Lighting" {
                    return Some(entity);
                }
            }
        }
    }
    None
}

pub struct RBXScriptSignal {
    pub name: &'static str,
    pub entity: Entity,
}

impl LuaUserData for RBXScriptSignal {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("Connect", |lua, this, callback: LuaFunction| {
            let registry_ref = lua.app_data_ref::<ScriptRegistryRef>().unwrap();
            let mut registry = registry_ref.0.lock().unwrap();
            let key = std::sync::Arc::new(lua.create_registry_value(callback)?);
            registry.connections.entry((this.entity, this.name))
                .or_default()
                .push(key.clone());

            let conn_table = lua.create_table()?;
            let key_clone = key.clone();
            let entity = this.entity;
            let name = this.name;
            let registry_ref_clone = (*registry_ref).clone();
            conn_table.set("Disconnect", lua.create_function(move |_, _: ()| {
                let mut registry = registry_ref_clone.0.lock().unwrap();
                if let Some(conns) = registry.connections.get_mut(&(entity, name)) {
                    conns.retain(|k| k != &key_clone);
                }
                Ok(())
            })?)?;
            Ok(conn_table)
        });
    }
}