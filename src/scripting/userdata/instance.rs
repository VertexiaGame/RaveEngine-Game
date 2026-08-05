use bevy::prelude::*;
use mlua::prelude::*;
use crate::common::game::bricks::components::{Brick, BrickColor, BrickPhysics, BrickShapeComponent};
use super::vector3::Vector3;
use super::color3::Color3;
use super::cframe::CFrame;
use crate::scripting::ecs::{ServerScript, LocalScript, ModuleScript};
use crate::scripting::vm::scheduler::ScriptRegistryRef;
use avian3d::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Instance {
    pub entity: Entity,
}

fn to_studs(meters: f32) -> f32 {
    let studs = meters / 0.28;
    (studs * 10000.0).round() / 10000.0
}

pub fn class_name_of(world: &World, entity: Entity) -> &'static str {
    let name = world.get::<Name>(entity).map(|n| n.as_str()).unwrap_or("");
    if name == "Workspace" {
        return "Workspace";
    }
    if world.get::<crate::common::net::components::PlayersServiceContainer>(entity).is_some() {
        return "Players";
    }
    if world.get::<crate::common::net::components::LightingServiceContainer>(entity).is_some() {
        return "Lighting";
    }
    if world.get::<crate::common::net::components::Player>(entity).is_some() {
        return "Player";
    }
    if world.get::<Brick>(entity).is_some() {
        return "Part";
    }
    if world.get::<ServerScript>(entity).is_some() {
        return "Script";
    }
    if world.get::<LocalScript>(entity).is_some() {
        return "LocalScript";
    }
    if world.get::<ModuleScript>(entity).is_some() {
        return "ModuleScript";
    }
    "Folder"
}

fn is_workspace(world: &World, entity: Entity) -> bool {
    world.get::<Name>(entity).map_or(false, |n| n.as_str() == "Workspace")
}

fn is_managed(world: &World, entity: Entity) -> bool {
    world.get::<Brick>(entity).is_some()
        || world.get::<ServerScript>(entity).is_some()
        || world.get::<LocalScript>(entity).is_some()
        || world.get::<ModuleScript>(entity).is_some()
}

fn direct_children(world: &World, entity: Entity) -> Vec<Entity> {
    let mut children = Vec::new();
    if is_workspace(world, entity) {
        if let Some(children_comp) = world.get::<Children>(entity) {
            children.extend(children_comp.to_vec());
        }
        for archetype in world.archetypes().iter() {
            for location in archetype.entities() {
                let child = location.id();
                if child == entity {
                    continue;
                }
                if world.get::<ChildOf>(child).is_some() {
                    continue;
                }
                if is_managed(world, child) {
                    children.push(child);
                }
            }
        }
    } else if let Some(children_comp) = world.get::<Children>(entity) {
        children.extend(children_comp.to_vec());
    }
    children
}

fn descendants(world: &World, entity: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack = direct_children(world, entity);
    while let Some(child) = stack.pop() {
        out.push(child);
        if let Some(children_comp) = world.get::<Children>(child) {
            stack.extend(children_comp.to_vec());
        }
    }
    out
}

fn last_string_arg(args: &LuaMultiValue) -> Option<String> {
    args.iter().rev().find_map(|v| match v {
        LuaValue::String(s) => Some(s.to_string_lossy()),
        _ => None,
    })
}

fn last_boolean_arg(args: &LuaMultiValue) -> Option<bool> {
    args.iter().rev().find_map(|v| match v {
        LuaValue::Boolean(b) => Some(*b),
        _ => None,
    })
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
            let world = unsafe { &*world_ref.0 };
            let name = world.get::<Name>(this.entity).map(|n| n.as_str().to_string()).unwrap_or_else(|| "Instance".to_string());
            Ok(name)
        });

        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &mut *world_ref.0 };

            if world.get_entity(this.entity).is_err() && key != "Destroy" {
                return Err(mlua::Error::RuntimeError("Instance has been destroyed".to_string()));
            }

            match key.as_str() {
                "Name" => {
                    let name = world.get::<Name>(this.entity).map(|n| n.as_str().to_string()).unwrap_or_default();
                    Ok(LuaValue::String(lua.create_string(&name)?))
                }
                "ClassName" => {
                    Ok(LuaValue::String(lua.create_string(class_name_of(world, this.entity))?))
                }
                "Position" => {
                    let translation = world.get::<Transform>(this.entity).map(|t| t.translation).unwrap_or_default();
                    lua.create_userdata(Vector3(Vec3::new(to_studs(translation.x), to_studs(translation.y), to_studs(translation.z)))).map(LuaValue::UserData)
                }
                "Size" => {
                    let scale = world.get::<Transform>(this.entity).map(|t| t.scale).unwrap_or(Vec3::ONE);
                    lua.create_userdata(Vector3(scale)).map(LuaValue::UserData)
                }
                "CFrame" => {
                    let transform = world.get::<Transform>(this.entity).cloned().unwrap_or_default();
                    lua.create_userdata(CFrame {
                        position: Vec3::new(to_studs(transform.translation.x), to_studs(transform.translation.y), to_studs(transform.translation.z)),
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
                    lua.create_userdata(RBXScriptSignal {
                        name: "Touched",
                        entity: this.entity,
                    }).map(LuaValue::UserData)
                }
                "JumpPower" => {
                    let jp = world.get::<crate::common::net::components::Player>(this.entity)
                        .map(|p| to_studs(p.jump_power))
                        .unwrap_or(50.0);
                    Ok(LuaValue::Number(jp as f64))
                }
                "Speed" => {
                    let s = world.get::<crate::common::net::components::Player>(this.entity)
                        .map(|p| to_studs(p.speed))
                        .unwrap_or(16.0);
                    Ok(LuaValue::Number(s as f64))
                }
                "Velocity" => {
                    let vel = world.get::<LinearVelocity>(this.entity)
                        .map(|v| Vec3::new(to_studs(v.0.x), to_studs(v.0.y), to_studs(v.0.z)))
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
                    let entity = this.entity;
                    Ok(LuaValue::Function(lua.create_function(move |lua, _: LuaMultiValue| {
                        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
                        let world = unsafe { &*world_ref.0 };
                        let children = direct_children(world, entity);
                        let table = lua.create_table()?;
                        for (i, child) in children.into_iter().enumerate() {
                            table.set(i + 1, Instance { entity: child })?;
                        }
                        Ok(LuaValue::Table(table))
                    })?))
                }
                "GetDescendants" => {
                    let entity = this.entity;
                    Ok(LuaValue::Function(lua.create_function(move |lua, _: LuaMultiValue| {
                        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
                        let world = unsafe { &*world_ref.0 };
                        let all = descendants(world, entity);
                        let table = lua.create_table()?;
                        for (i, child) in all.into_iter().enumerate() {
                            table.set(i + 1, Instance { entity: child })?;
                        }
                        Ok(LuaValue::Table(table))
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
                "IsA" => {
                    let entity = this.entity;
                    Ok(LuaValue::Function(lua.create_function(move |lua, args: LuaMultiValue| {
                        let want = last_string_arg(&args)
                            .ok_or_else(|| mlua::Error::RuntimeError("IsA expects a class name".to_string()))?;
                        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
                        let world = unsafe { &*world_ref.0 };
                        let class = class_name_of(world, entity);
                        let is = want == "Instance" || class == want || (want == "BasePart" && class == "Part");
                        Ok(LuaValue::Boolean(is))
                    })?))
                }
                "FindFirstChild" => {
                    let entity = this.entity;
                    Ok(LuaValue::Function(lua.create_function(move |lua, args: LuaMultiValue| {
                        let name_to_find = last_string_arg(&args)
                            .ok_or_else(|| mlua::Error::RuntimeError("FindFirstChild expects a name".to_string()))?;
                        let recursive = last_boolean_arg(&args).unwrap_or(false);
                        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
                        let world = unsafe { &*world_ref.0 };
                        let search: Vec<Entity> = if recursive {
                            descendants(world, entity)
                        } else {
                            direct_children(world, entity)
                        };
                        for child in search {
                            if world.get::<Name>(child).is_some_and(|n| n.as_str() == name_to_find) {
                                return lua.create_userdata(Instance { entity: child }).map(LuaValue::UserData);
                            }
                        }
                        Ok(LuaValue::Nil)
                    })?))
                }
                "FindFirstChildOfClass" => {
                    let entity = this.entity;
                    Ok(LuaValue::Function(lua.create_function(move |lua, args: LuaMultiValue| {
                        let want = last_string_arg(&args)
                            .ok_or_else(|| mlua::Error::RuntimeError("FindFirstChildOfClass expects a class name".to_string()))?;
                        let recursive = last_boolean_arg(&args).unwrap_or(false);
                        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
                        let world = unsafe { &*world_ref.0 };
                        let search: Vec<Entity> = if recursive {
                            descendants(world, entity)
                        } else {
                            direct_children(world, entity)
                        };
                        for child in search {
                            if class_name_of(world, child) == want {
                                return lua.create_userdata(Instance { entity: child }).map(LuaValue::UserData);
                            }
                        }
                        Ok(LuaValue::Nil)
                    })?))
                }
                "WaitForChild" => {
                    Ok(LuaValue::Function(
                        lua.globals().get::<LuaFunction>("__vertigo_waitforchild")?,
                    ))
                }
                "Clone" => {
                    let entity = this.entity;
                    Ok(LuaValue::Function(lua.create_function(move |lua, _: LuaMultiValue| {
                        let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
                        let world = unsafe { &mut *world_ref.0 };
                        if world.get_entity(entity).is_err() {
                            return Err(mlua::Error::RuntimeError("Instance to clone has been destroyed".to_string()));
                        }
                        let (transform, name, shape, phys, color, layers, is_brick, parent, server_code, local_code, module_code) = {
                            let transform = world.get::<Transform>(entity).cloned().unwrap_or_default();
                            let name = world.get::<Name>(entity).cloned().unwrap_or_else(|| Name::new("Clone"));
                            let shape = world.get::<BrickShapeComponent>(entity).cloned();
                            let phys = world.get::<BrickPhysics>(entity).cloned();
                            let color = world.get::<BrickColor>(entity).cloned();
                            let layers = world.get::<CollisionLayers>(entity).cloned();
                            let is_brick = world.get::<Brick>(entity).is_some();
                            let parent = world.get::<ChildOf>(entity)
                                .map(|co| co.parent())
                                .filter(|parent| world.get_entity(*parent).is_ok());
                            let server_code = world.get::<ServerScript>(entity).map(|s| s.code.clone());
                            let local_code = world.get::<LocalScript>(entity).map(|s| s.code.clone());
                            let module_code = world.get::<ModuleScript>(entity).map(|s| s.code.clone());
                            (transform, name, shape, phys, color, layers, is_brick, parent, server_code, local_code, module_code)
                        };
                        let mut new_entity = world.spawn((transform, name));
                        if is_brick { new_entity.insert(Brick); }
                        if let Some(s) = shape { new_entity.insert(s); }
                        if let Some(p) = phys { new_entity.insert(p); }
                        if let Some(c) = color { new_entity.insert(c); }
                        if let Some(l) = layers { new_entity.insert(l); }
                        if let Some(code) = server_code {
                            new_entity.insert(ServerScript { code, ..default() });
                        }
                        if let Some(code) = local_code {
                            new_entity.insert(LocalScript { code, ..default() });
                        }
                        if let Some(code) = module_code {
                            new_entity.insert(ModuleScript { code });
                        }
                        new_entity.insert(lightyear::prelude::Replicate::default());
                        let new_id = new_entity.id();
                        drop(new_entity);
                        if let Some(parent) = parent {
                            world.entity_mut(parent).add_child(new_id);
                        }
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
                                if world.get_entity(parent_inst.entity).is_ok() {
                                    world.entity_mut(parent_inst.entity).add_child(this.entity);
                                } else {
                                    return Err(mlua::Error::RuntimeError(
                                        "Parent instance has been destroyed".to_string(),
                                    ));
                                }
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
        if let Some(entity) = cached {
            if world.get::<Name>(entity).is_some_and(|name| name.as_str() == service_name) {
                return Some(entity);
            }
        }
    }
    for archetype in world.archetypes().iter() {
        for location in archetype.entities() {
            let entity = location.id();
            if class_name_of(world, entity) == service_name {
                return Some(entity);
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
            let key = Arc::new(lua.create_registry_value(callback)?);
            registry.connections.entry((this.entity, this.name))
                .or_default()
                .push(key.clone());

            let conn_table = lua.create_table()?;
            let entity = this.entity;
            let name = this.name;
            let registry_ref_clone = (*registry_ref).clone();
            let mut owned_key = Some(key);
            conn_table.set("Disconnect", lua.create_function_mut(move |lua, _: ()| {
                let mut registry = registry_ref_clone.0.lock().unwrap();
                let mut to_remove = None;
                if let Some(conns) = registry.connections.get_mut(&(entity, name)) {
                    conns.retain(|k| !Arc::ptr_eq(k, owned_key.as_ref().unwrap()));
                    if conns.is_empty() {
                        registry.connections.remove(&(entity, name));
                    }
                }
                if let Some(k) = owned_key.take() {
                    match Arc::try_unwrap(k) {
                        Ok(orphaned) => to_remove = Some(orphaned),
                        Err(arc) => owned_key = Some(arc),
                    }
                }
                drop(registry);
                if let Some(orphaned) = to_remove {
                    let _ = lua.remove_registry_value(orphaned);
                }
                Ok(())
            })?)?;
            Ok(conn_table)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::testing::{
        advance, entity_of, eval, global, run_script, spawn_brick, test_vm, test_world,
    };
    use crate::common::net::components::{LightingServiceContainer, Player, PlayersServiceContainer};

    fn expose(vm: &crate::scripting::vm::server_vm::ServerScriptVM, name: &str, entity: Entity) {
        vm.lua
            .globals()
            .set(name, vm.lua.create_userdata(Instance { entity }).unwrap())
            .unwrap();
    }

    fn count_connections(vm: &crate::scripting::vm::server_vm::ServerScriptVM, entity: Entity, name: &'static str) -> usize {
        vm.registry
            .lock()
            .unwrap()
            .connections
            .get(&(entity, name))
            .map_or(0, |conns| conns.len())
    }


    #[test]
    fn class_names_cover_all_known_entity_kinds() {
        let mut world = test_world();
        let vm = test_vm(&mut world);

        let workspace = world.spawn(Name::new("Workspace")).id();
        let players = world.spawn(PlayersServiceContainer).id();
        let lighting = world.spawn(LightingServiceContainer).id();
        let player = world.spawn(Player::default()).id();
        let brick = spawn_brick(&mut world, "Brick");
        let server_script = world.spawn((Name::new("S"), ServerScript::default())).id();
        let local_script = world.spawn((Name::new("L"), LocalScript::default())).id();
        let module = world.spawn((Name::new("M"), ModuleScript::default())).id();
        let folder = world.spawn(Name::new("Folder")).id();

        let cases = [
            (workspace, "Workspace"),
            (players, "Players"),
            (lighting, "Lighting"),
            (player, "Player"),
            (brick, "Part"),
            (server_script, "Script"),
            (local_script, "LocalScript"),
            (module, "ModuleScript"),
            (folder, "Folder"),
        ];
        for (i, (entity, expected)) in cases.into_iter().enumerate() {
            expose(&vm, "e", entity);
            let class: String = eval(&vm, "return e.ClassName");
            assert_eq!(class, expected, "entity #{i}");
        }
    }

    #[test]
    fn index_unknown_keys_return_nil() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let entity = spawn_brick(&mut world, "Brick");
        expose(&vm, "e", entity);
        let is_nil: bool = eval(&vm, "return e.NotARealProperty == nil");
        assert!(is_nil);
    }

    #[test]
    fn instance_equality_and_tostring() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let entity = spawn_brick(&mut world, "MyBrick");
        expose(&vm, "e", entity);
        expose(&vm, "f", entity);
        let other = spawn_brick(&mut world, "Other");
        expose(&vm, "g", other);

        let (eq, ne, str): (bool, bool, String) = eval(&vm, "return e == f, e == g, tostring(e)");
        assert!(eq);
        assert!(!ne);
        assert_eq!(str, "MyBrick");
    }


    #[test]
    fn part_properties_round_trip_through_lua() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            local p = Instance.new("Part")
            p.Name = "Hello"
            p.CFrame = CFrame.new(5, 6, 7) * CFrame.angles(0, math.pi / 2, 0)
            p.Position = Vector3.new(10, 20, 30)
            p.Size = Vector3.new(2, 3, 4)
            p.Color = Color3.fromRGB(255, 0, 0)
            p.Anchored = false
            p.CanCollide = false
            _G.part = p
        "#);
        let part = entity_of(&vm, "part");

        let (name, class, px, py, pz, sx, sz, look_x, look_z, r, g, b, anchored, can_collide): (
            String, String, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, bool, bool,
        ) = eval(
            &vm,
            r#"
                local p = _G.part
                local cf = p.CFrame
                return p.Name, p.ClassName,
                       p.Position.X, p.Position.Y, p.Position.Z,
                       p.Size.X, p.Size.Z,
                       cf.LookVector.X, cf.LookVector.Z,
                       p.Color.R, p.Color.G, p.Color.B,
                       p.Anchored, p.CanCollide
            "#,
        );
        assert_eq!(name, "Hello");
        assert_eq!(class, "Part");
        assert_eq!((px, py, pz), (10.0, 20.0, 30.0));
        assert!((sx - 2.0).abs() < 1e-4 && (sz - 4.0).abs() < 1e-4);
        assert!(look_x.abs() > 0.999, "look_x: {look_x}");
        assert!(look_z.abs() < 1e-3);
        assert!((r - 1.0).abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);
        assert!(!anchored);
        assert!(!can_collide);

        let phys = world.get::<BrickPhysics>(part).unwrap();
        assert!(phys.enabled, "Anchored=false must enable physics");
        assert!(!phys.player_can_collide);
        assert_eq!(
            world.get::<RigidBody>(part).unwrap(),
            &RigidBody::Dynamic,
            "unanchored parts must become dynamic"
        );
    }

    #[test]
    fn anchoring_a_part_makes_it_static() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            local p = Instance.new("Part")
            p.Anchored = true
            _G.part = p
        "#);
        let part = entity_of(&vm, "part");
        assert!(world.get::<BrickPhysics>(part).unwrap().enabled == false);
        assert_eq!(world.get::<RigidBody>(part).unwrap(), &RigidBody::Static);
        let anchored: bool = eval(&vm, "return _G.part.Anchored");
        assert!(anchored);
    }

    #[test]
    fn velocity_and_player_stats_round_trip() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let player = world
            .spawn((Player {
                client_id: 1,
                speed: 100.0,
                jump_power: 40.0,
                username: "Tester".to_string(),
            },))
            .id();
        expose(&vm, "p", player);

        run_script(&vm, r#"
            local p = _G.p
            p.Velocity = Vector3.new(1, 2, 3)
            p.JumpPower = 60
            p.Speed = 120
        "#);

        let (vx, vz, jp, spd): (f64, f64, f64, f64) = eval(
            &vm,
            "return _G.p.Velocity.X, _G.p.Velocity.Z, _G.p.JumpPower, _G.p.Speed",
        );
        assert_eq!((vx, vz), (1.0, 3.0));
        assert_eq!(jp, 60.0);
        assert_eq!(spd, 120.0);

        let stored = world.get::<LinearVelocity>(player).unwrap();
        assert!((stored.0.x - 0.28).abs() < 1e-6);
        let player_comp = world.get::<Player>(player).unwrap();
        assert!((player_comp.jump_power - 16.8).abs() < 1e-6);
        assert!((player_comp.speed - 33.6).abs() < 1e-6);
    }


    #[test]
    fn parent_manipulation() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            _G.folder = Instance.new("Folder")
            _G.child = Instance.new("Folder")
            _G.child.Parent = _G.folder
        "#);
        let folder = entity_of(&vm, "folder");
        let child = entity_of(&vm, "child");
        assert_eq!(world.get::<ChildOf>(child).unwrap().parent(), folder);

        run_script(&vm, "_G.child.Parent = nil");
        assert!(world.get::<ChildOf>(child).is_none());

        let gone = world.spawn(Name::new("Gone")).id();
        world.entity_mut(gone).despawn();
        expose(&vm, "gone", gone);
        let ok: bool = eval(&vm, "return pcall(function() _G.child.Parent = _G.gone end)");
        assert!(!ok);
    }

    #[test]
    fn workspace_getchildren_reports_unparented_managed_entities() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        world.spawn(Name::new("Workspace"));
        let brick = spawn_brick(&mut world, "LooseBrick");
        world.spawn(Name::new("Ignored"));
        expose(&vm, "brick", brick);

        let found: bool = eval(
            &vm,
            r#"
                local children = workspace:GetChildren()
                for _, child in ipairs(children) do
                    if child == _G.brick then return true end
                end
                return false
            "#,
        );
        assert!(found);
    }

    #[test]
    fn children_descendants_and_findfirstchild() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            local folder = Instance.new("Folder")
            folder.Name = "F"
            local part = Instance.new("Part")
            part.Name = "Target"
            part.Parent = folder
            local script = Instance.new("Folder")
            script.Name = "Target"
            script.Parent = folder
            local sub = Instance.new("Folder")
            sub.Name = "Sub"
            sub.Parent = folder
            local deep = Instance.new("Part")
            deep.Name = "Deep"
            deep.Parent = sub
            _G.folder = folder
        "#);
        let folder = entity_of(&vm, "folder");

        let (child_count, has_deep, by_name, missing, by_class): (usize, bool, String, bool, String) = eval(
            &vm,
            r#"
                local f = _G.folder
                local children = f:GetChildren()
                local descendants = f:GetDescendants()
                local found = f:FindFirstChild("Target")
                local deep = f:FindFirstChild("Deep", true)
                return #children, deep ~= nil,
                       found.Name, f:FindFirstChild("Nope") == nil,
                       f:FindFirstChildOfClass("Part").Name
            "#,
        );
        assert_eq!(child_count, 3);
        assert!(has_deep);
        assert_eq!(by_name, "Target");
        assert!(missing);
        assert_eq!(by_class, "Target");

        let desc_count: usize = eval(&vm, "return #_G.folder:GetDescendants()");
        assert_eq!(desc_count, 4);
        assert!(world.get::<Children>(folder).is_some());
    }

    #[test]
    fn is_a_matches_class_and_bases() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, "_G.p = Instance.new('Part')");
        let (p, inst, base, player): (bool, bool, bool, bool) = eval(
            &vm,
            r#"
                local p = _G.p
                return p:IsA("Part"), p:IsA("Instance"), p:IsA("BasePart"), p:IsA("Player")
            "#,
        );
        assert!(p && inst && base);
        assert!(!player);
    }

    #[test]
    fn wait_for_child_finds_immediate_child() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            local folder = Instance.new("Folder")
            local part = Instance.new("Part")
            part.Name = "Ready"
            part.Parent = folder
            _G.found = folder:WaitForChild("Ready").Name
        "#);
        assert_eq!(global::<String>(&vm, "found"), "Ready");
    }

    #[test]
    fn wait_for_child_waits_for_late_child() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            _G.folder = Instance.new("Folder")
            task.spawn(function()
                task.wait(0.05)
                local p = Instance.new("Part")
                p.Name = "Late"
                p.Parent = _G.folder
            end)
            task.spawn(function()
                local child = _G.folder:WaitForChild("Late", 2)
                _G.found = child and child.Name or "none"
            end)
        "#);
        advance(&vm, 20, 10);
        assert_eq!(global::<String>(&vm, "found"), "Late");
    }

    #[test]
    fn wait_for_child_times_out() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            _G.folder = Instance.new("Folder")
            task.spawn(function()
                local ok, err = pcall(function()
                    _G.folder:WaitForChild("Never", 0.2)
                end)
                _G.timed_out = not ok and tostring(err):find("timed out") ~= nil
            end)
        "#);
        advance(&vm, 25, 12);
        assert!(global::<bool>(&vm, "timed_out"));
    }


    #[test]
    fn clone_copies_class_properties_and_parent() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            local folder = Instance.new("Folder")
            folder.Name = "ParentFolder"
            local p = Instance.new("Part")
            p.Name = "Src"
            p.Position = Vector3.new(10, 20, 30)
            p.Color = Color3.fromRGB(255, 0, 0)
            p.Parent = folder
            _G.p = p
        "#);
        let original = entity_of(&vm, "p");

        let (name, class, px, py, pz, parent_name, is_same): (String, String, f64, f64, f64, String, bool) = eval(
            &vm,
            r#"
                local c = _G.p:Clone()
                return c.Name, c.ClassName,
                       c.Position.X, c.Position.Y, c.Position.Z,
                       c.Parent.Name, c == _G.p
            "#,
        );
        assert_eq!(name, "Src");
        assert_eq!(class, "Part");
        assert_eq!((px, py, pz), (10.0, 20.0, 30.0));
        assert_eq!(parent_name, "ParentFolder");
        assert!(!is_same);

        let cloned_entity = eval::<mlua::AnyUserData>(&vm, "return _G.p:Clone()");
        let cloned_entity = cloned_entity.borrow::<Instance>().unwrap().entity;
        assert_ne!(cloned_entity, original);
        assert!(world.get::<Brick>(cloned_entity).is_some());
        assert_eq!(
            world.get::<ChildOf>(cloned_entity).unwrap().parent(),
            world.get::<ChildOf>(original).unwrap().parent()
        );
    }

    #[test]
    fn clone_copies_script_code() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        let script = world
            .spawn((
                Name::new("S"),
                ServerScript {
                    code: "print('hello')".to_string(),
                    ..default()
                },
            ))
            .id();
        expose(&vm, "s", script);

        let cloned = eval::<mlua::AnyUserData>(&vm, "return _G.s:Clone()");
        let cloned = cloned.borrow::<Instance>().unwrap().entity;
        let cloned_code = world.get::<ServerScript>(cloned).unwrap().code.clone();
        assert_eq!(cloned_code, "print('hello')");
        assert!(!world.get::<ServerScript>(cloned).unwrap().started);
    }

    #[test]
    fn destroy_removes_the_entity_and_future_access_errors() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            _G.p = Instance.new("Part")
            _G.p:Destroy()
        "#);
        let part = entity_of(&vm, "p");
        assert!(world.get_entity(part).is_err(), "Destroy must despawn the entity");

        let (index_ok, newindex_ok): (bool, bool) = eval(
            &vm,
            r#"
                local p = _G.p
                return pcall(function() return p.Name end),
                       pcall(function() p.Name = "x" end)
            "#,
        );
        assert!(!index_ok);
        assert!(!newindex_ok);
    }

    #[test]
    fn destroy_is_idempotent() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            _G.p = Instance.new("Part")
            _G.p:Destroy()
            local ok = pcall(function() _G.p:Destroy() end)
            _G.double_destroy_ok = ok
        "#);
        assert!(global::<bool>(&vm, "double_destroy_ok"));
    }


    #[test]
    fn instance_resolves_service_instances() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        world.spawn(Name::new("Workspace"));
        world.spawn(PlayersServiceContainer);
        world.spawn(LightingServiceContainer);
        run_script(&vm, "_G.p = Instance.new('Part')");

        let (ws, pl, lg): (String, String, String) = eval(
            &vm,
            r#"
                return _G.p.Workspace.ClassName, _G.p.Players.ClassName, _G.p.Lighting.ClassName
            "#,
        );
        assert_eq!(ws, "Workspace");
        assert_eq!(pl, "Players");
        assert_eq!(lg, "Lighting");
    }

    #[test]
    fn touched_signal_connect_and_disconnect() {
        let mut world = test_world();
        let vm = test_vm(&mut world);
        run_script(&vm, r#"
            _G.p = Instance.new("Part")
            _G.conn = _G.p.Touched:Connect(function() end)
            _G.conn2 = _G.p.Touched:Connect(function() end)
        "#);
        let part = entity_of(&vm, "p");
        assert_eq!(count_connections(&vm, part, "Touched"), 2);

        run_script(&vm, "_G.conn:Disconnect()");
        assert_eq!(count_connections(&vm, part, "Touched"), 1);

        run_script(&vm, "_G.conn2:Disconnect()");
        assert_eq!(count_connections(&vm, part, "Touched"), 0);
        assert!(
            !vm.registry.lock().unwrap().connections.contains_key(&(part, "Touched")),
            "empty connection lists must be removed"
        );
    }

    #[test]
    fn find_service_entity_uses_cache_then_fallback() {
        let mut world = test_world();
        let entity = world.spawn(Name::new("Workspace")).id();

        assert_eq!(find_service_entity(&world, "Workspace"), Some(entity));
        assert_eq!(find_service_entity(&world, "Players"), None);

        world.resource_mut::<crate::scripting::vm::scheduler::ServiceEntities>().workspace = Some(entity);
        assert_eq!(find_service_entity(&world, "Workspace"), Some(entity));

        world.resource_mut::<crate::scripting::vm::scheduler::ServiceEntities>().workspace = Some(Entity::PLACEHOLDER);
        assert_eq!(find_service_entity(&world, "Workspace"), Some(entity));
    }
}
