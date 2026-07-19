use bevy::prelude::*;
use crate::scripting::ecs::{ServerScript, LocalScript, ModuleScript};
use crate::scripting::vm::server_vm::ServerScriptVM;
use crate::scripting::vm::client_vm::ClientScriptVM;
use avian3d::prelude::CollidingEntities;
use mlua::prelude::*;

pub struct ScriptingPlugin;

impl Plugin for ScriptingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::scripting::vm::scheduler::ServiceEntities>()
            .register_type::<ServerScript>()
            .register_type::<LocalScript>()
            .register_type::<ModuleScript>()
            .add_systems(Update, (
                discover_and_run_server_scripts,
                discover_and_run_local_scripts,
                detect_touched_collisions,
                detect_player_added_events,
                trigger_run_service_events,
                server_scheduler_system,
                client_scheduler_system,
            ).chain())
            .add_systems(PostUpdate, cache_service_entities
                .run_if(|cache: Option<Res<crate::scripting::vm::scheduler::ServiceEntities>>| {
                    cache.map_or(true, |c| c.workspace.is_none() || c.players.is_none() || c.lighting.is_none())
                })
            );
    }
}

fn spawn_and_run_callback(
    lua: &Lua,
    scheduler: &std::sync::Arc<std::sync::Mutex<crate::scripting::vm::scheduler::LuaScheduler>>,
    func: LuaFunction,
    arg: LuaValue,
) {
    if let Ok(thread) = lua.create_thread(func) {
        match thread.resume::<Option<f32>>(arg) {
            Ok(yielded_val) => {
                if thread.status() == LuaThreadStatus::Resumable {
                    let wake = yielded_val.map(|sec| std::time::Instant::now() + std::time::Duration::from_secs_f64(sec as f64));
                    let mut sched = scheduler.lock().unwrap();
                    if let Ok(key) = lua.create_registry_value(thread) {
                        sched.tasks.push(crate::scripting::vm::scheduler::LuaTask {
                            thread_key: key,
                            wake_time: wake,
                        });
                    }
                }
            }
            Err(e) => {
                error!("Luau callback runtime error: {}", e);
            }
        }
    }
}

pub fn server_scheduler_system(world: &mut World) {
    if let Some(server_vm) = world.remove_resource::<ServerScriptVM>() {
        server_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));
        {
            let mut scheduler = server_vm.scheduler.lock().unwrap();
            scheduler.run_tick(&server_vm.lua);
        }
        world.insert_resource(server_vm);
    }
}

pub fn client_scheduler_system(world: &mut World) {
    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        client_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));
        {
            let mut scheduler = client_vm.scheduler.lock().unwrap();
            scheduler.run_tick(&client_vm.lua);
        }
        world.insert_resource(client_vm);
    }
}

pub fn discover_and_run_server_scripts(world: &mut World) {
    let mut scripts_to_run = Vec::new();
    let mut query = world.query::<(Entity, &mut ServerScript)>();
    for (entity, mut script) in query.iter_mut(world) {
        if !script.started && script.enabled {
            scripts_to_run.push((entity, script.code.clone()));
            script.started = true;
        }
    }

    if scripts_to_run.is_empty() {
        return;
    }

    if let Some(server_vm) = world.remove_resource::<ServerScriptVM>() {
        server_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));

        for (entity, code) in scripts_to_run {
            match crate::scripting::vm::compiler::compile_code(&server_vm.lua, &code, "ServerScript") {
                Ok(func) => {
                    let script_env = match server_vm.lua.create_table() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let meta = match server_vm.lua.create_table() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let globals = server_vm.lua.globals();
                    let _ = meta.set("__index", globals);
                    let _ = script_env.set_metatable(Some(meta));

                    let script_instance = crate::scripting::userdata::instance::Instance { entity };
                    let _ = script_env.set("script", script_instance);

                    let _ = func.set_environment(script_env);

                    match server_vm.lua.create_thread(func) {
                        Ok(thread) => {
                            match thread.resume::<Option<f32>>(()) {
                                Ok(yielded_val) => {
                                    if thread.status() == LuaThreadStatus::Resumable {
                                        let wake = yielded_val.map(|sec| std::time::Instant::now() + std::time::Duration::from_secs_f64(sec as f64));
                                        let mut scheduler = server_vm.scheduler.lock().unwrap();
                                        if let Ok(key) = server_vm.lua.create_registry_value(thread) {
                                            scheduler.tasks.push(crate::scripting::vm::scheduler::LuaTask {
                                                thread_key: key,
                                                wake_time: wake,
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Luau script runtime error: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to create thread for ServerScript: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Luau script compile error: {}", e);
                }
            }
        }

        world.insert_resource(server_vm);
    }
}

pub fn discover_and_run_local_scripts(world: &mut World) {
    let mut scripts_to_run = Vec::new();
    let mut query = world.query::<(Entity, &mut LocalScript)>();
    for (entity, mut script) in query.iter_mut(world) {
        if !script.started && script.enabled {
            scripts_to_run.push((entity, script.code.clone()));
            script.started = true;
        }
    }

    if scripts_to_run.is_empty() {
        return;
    }

    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        client_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));

        for (entity, code) in scripts_to_run {
            match crate::scripting::vm::compiler::compile_code(&client_vm.lua, &code, "LocalScript") {
                Ok(func) => {
                    let script_env = match client_vm.lua.create_table() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let meta = match client_vm.lua.create_table() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let globals = client_vm.lua.globals();
                    let _ = meta.set("__index", globals);
                    let _ = script_env.set_metatable(Some(meta));

                    let script_instance = crate::scripting::userdata::instance::Instance { entity };
                    let _ = script_env.set("script", script_instance);

                    let _ = func.set_environment(script_env);

                    match client_vm.lua.create_thread(func) {
                        Ok(thread) => {
                            match thread.resume::<Option<f32>>(()) {
                                Ok(yielded_val) => {
                                    if thread.status() == LuaThreadStatus::Resumable {
                                        let wake = yielded_val.map(|sec| std::time::Instant::now() + std::time::Duration::from_secs_f64(sec as f64));
                                        let mut scheduler = client_vm.scheduler.lock().unwrap();
                                        if let Ok(key) = client_vm.lua.create_registry_value(thread) {
                                            scheduler.tasks.push(crate::scripting::vm::scheduler::LuaTask {
                                                thread_key: key,
                                                wake_time: wake,
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Luau local script runtime error: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to create thread for LocalScript: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Luau local script compile error: {}", e);
                }
            }
        }

        world.insert_resource(client_vm);
    }
}

pub fn detect_touched_collisions(world: &mut World) {
    let mut collisions = Vec::new();
    let mut query = world.query::<(Entity, &CollidingEntities)>();
    for (entity, colliding) in query.iter(world) {
        for other in colliding.iter() {
            collisions.push((entity, *other));
        }
    }

    if collisions.is_empty() {
        return;
    }

    if let Some(server_vm) = world.remove_resource::<ServerScriptVM>() {
        server_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));

        {
            let registry = server_vm.registry.lock().unwrap();
            for &(entity, other) in &collisions {
                if let Some(keys) = registry.connections.get(&(entity, "Touched".to_string())) {
                    for key in keys {
                        if let Ok(func) = server_vm.lua.registry_value::<LuaFunction>(&**key) {
                            if let Ok(other_inst) = server_vm.lua.create_userdata(crate::scripting::userdata::instance::Instance { entity: other }) {
                                spawn_and_run_callback(&server_vm.lua, &server_vm.scheduler, func, LuaValue::UserData(other_inst));
                            }
                        }
                    }
                }
                if let Some(keys) = registry.connections.get(&(other, "Touched".to_string())) {
                    for key in keys {
                        if let Ok(func) = server_vm.lua.registry_value::<LuaFunction>(&**key) {
                            if let Ok(entity_inst) = server_vm.lua.create_userdata(crate::scripting::userdata::instance::Instance { entity }) {
                                spawn_and_run_callback(&server_vm.lua, &server_vm.scheduler, func, LuaValue::UserData(entity_inst));
                            }
                        }
                    }
                }
            }
        }

        world.insert_resource(server_vm);
    }

    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        client_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));

        {
            let registry = client_vm.registry.lock().unwrap();
            for &(entity, other) in &collisions {
                if let Some(keys) = registry.connections.get(&(entity, "Touched".to_string())) {
                    for key in keys {
                        if let Ok(func) = client_vm.lua.registry_value::<LuaFunction>(&**key) {
                            if let Ok(other_inst) = client_vm.lua.create_userdata(crate::scripting::userdata::instance::Instance { entity: other }) {
                                spawn_and_run_callback(&client_vm.lua, &client_vm.scheduler, func, LuaValue::UserData(other_inst));
                            }
                        }
                    }
                }
                if let Some(keys) = registry.connections.get(&(other, "Touched".to_string())) {
                    for key in keys {
                        if let Ok(func) = client_vm.lua.registry_value::<LuaFunction>(&**key) {
                            if let Ok(entity_inst) = client_vm.lua.create_userdata(crate::scripting::userdata::instance::Instance { entity }) {
                                spawn_and_run_callback(&client_vm.lua, &client_vm.scheduler, func, LuaValue::UserData(entity_inst));
                            }
                        }
                    }
                }
            }
        }

        world.insert_resource(client_vm);
    }
}

pub fn detect_player_added_events(
    world: &mut World,
    mut last_players: Local<std::collections::HashSet<Entity>>,
) {
    let mut current_players = std::collections::HashSet::new();
    let mut query = world.query::<(Entity, &crate::common::net::components::Player)>();
    for (entity, _) in query.iter(world) {
        current_players.insert(entity);
    }

    let joined: Vec<Entity> = current_players.difference(&last_players).cloned().collect();
    *last_players = current_players;

    if joined.is_empty() {
        return;
    }

    if let Some(server_vm) = world.remove_resource::<ServerScriptVM>() {
        server_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));

        {
            let registry = server_vm.registry.lock().unwrap();
            let players_entity = crate::scripting::userdata::instance::find_service_entity(world, "Workspace").unwrap_or(Entity::PLACEHOLDER);
            if let Some(keys) = registry.connections.get(&(players_entity, "PlayerAdded".to_string())) {
                for key in keys {
                    if let Ok(func) = server_vm.lua.registry_value::<LuaFunction>(&**key) {
                        for &entity in &joined {
                            if let Ok(inst) = server_vm.lua.create_userdata(crate::scripting::userdata::instance::Instance { entity }) {
                                spawn_and_run_callback(&server_vm.lua, &server_vm.scheduler, func.clone(), LuaValue::UserData(inst));
                            }
                        }
                    }
                }
            }
        }

        world.insert_resource(server_vm);
    }

    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        client_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));

        {
            let registry = client_vm.registry.lock().unwrap();
            let players_entity = crate::scripting::userdata::instance::find_service_entity(world, "Workspace").unwrap_or(Entity::PLACEHOLDER);
            if let Some(keys) = registry.connections.get(&(players_entity, "PlayerAdded".to_string())) {
                for key in keys {
                    if let Ok(func) = client_vm.lua.registry_value::<LuaFunction>(&**key) {
                        for &entity in &joined {
                            if let Ok(inst) = client_vm.lua.create_userdata(crate::scripting::userdata::instance::Instance { entity }) {
                                spawn_and_run_callback(&client_vm.lua, &client_vm.scheduler, func.clone(), LuaValue::UserData(inst));
                            }
                        }
                    }
                }
            }
        }

        world.insert_resource(client_vm);
    }
}

pub fn trigger_run_service_events(world: &mut World) {
    let delta_secs = {
        let time = world.resource::<Time>();
        time.delta_secs()
    };

    if let Some(server_vm) = world.remove_resource::<ServerScriptVM>() {
        server_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));

        {
            let registry = server_vm.registry.lock().unwrap();
            let workspace_entity = crate::scripting::userdata::instance::find_service_entity(world, "Workspace").unwrap_or(Entity::PLACEHOLDER);
            
            if let Some(keys) = registry.connections.get(&(workspace_entity, "Heartbeat".to_string())) {
                for key in keys {
                    if let Ok(func) = server_vm.lua.registry_value::<LuaFunction>(&**key) {
                        spawn_and_run_callback(&server_vm.lua, &server_vm.scheduler, func, LuaValue::Number(delta_secs as f64));
                    }
                }
            }

            if let Some(keys) = registry.connections.get(&(workspace_entity, "Stepped".to_string())) {
                for key in keys {
                    if let Ok(func) = server_vm.lua.registry_value::<LuaFunction>(&**key) {
                        spawn_and_run_callback(&server_vm.lua, &server_vm.scheduler, func, LuaValue::Number(delta_secs as f64));
                    }
                }
            }
        }

        world.insert_resource(server_vm);
    }

    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        client_vm.lua.set_app_data(crate::scripting::vm::server_vm::WorldRef(world as *mut World));

        {
            let registry = client_vm.registry.lock().unwrap();
            let workspace_entity = crate::scripting::userdata::instance::find_service_entity(world, "Workspace").unwrap_or(Entity::PLACEHOLDER);
            
            if let Some(keys) = registry.connections.get(&(workspace_entity, "Heartbeat".to_string())) {
                for key in keys {
                    if let Ok(func) = client_vm.lua.registry_value::<LuaFunction>(&**key) {
                        spawn_and_run_callback(&client_vm.lua, &client_vm.scheduler, func, LuaValue::Number(delta_secs as f64));
                    }
                }
            }

            if let Some(keys) = registry.connections.get(&(workspace_entity, "Stepped".to_string())) {
                for key in keys {
                    if let Ok(func) = client_vm.lua.registry_value::<LuaFunction>(&**key) {
                        spawn_and_run_callback(&client_vm.lua, &client_vm.scheduler, func, LuaValue::Number(delta_secs as f64));
                    }
                }
            }
        }

        world.insert_resource(client_vm);
    }
}

fn cache_service_entities(
    mut cache: ResMut<crate::scripting::vm::scheduler::ServiceEntities>,
    query: Query<(Entity, &Name)>,
) {
    for (entity, name) in &query {
        match name.as_str() {
            "Workspace" => cache.workspace = Some(entity),
            "Players" => cache.players = Some(entity),
            "Lighting" => cache.lighting = Some(entity),
            _ => {}
        }
    }
}