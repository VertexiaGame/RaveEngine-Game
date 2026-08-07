use bevy::prelude::*;
use crate::scripting::ecs::{ServerScript, LocalScript, ModuleScript};
use crate::scripting::vm::server_vm::{ServerScriptVM, WorldRef};
use crate::scripting::vm::client_vm::ClientScriptVM;
use crate::scripting::vm::scheduler::{LuaScheduler, LuaTask, PreviousCollisions, ServiceEntities, yielded_to_wake};
use avian3d::prelude::CollidingEntities;
use mlua::prelude::*;
use std::sync::{Arc, Mutex};

pub struct ScriptingPlugin;

impl Plugin for ScriptingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ServiceEntities>()
            .init_resource::<PreviousCollisions>()
            .register_type::<ServerScript>()
            .register_type::<LocalScript>()
            .register_type::<ModuleScript>()
            .add_systems(Update, (
                discover_and_run_server_scripts,
                discover_and_run_local_scripts,
                detect_touched_collisions,
                detect_player_added_events,
                trigger_run_service_events,
            ).chain())
            .add_systems(PostUpdate, (cache_service_entities, sweep_stale_connections));
    }
}

fn spawn_and_run_callback(
    lua: &Lua,
    scheduler: &Arc<Mutex<LuaScheduler>>,
    func: LuaFunction,
    arg: LuaValue,
) {
    if let Ok(thread) = lua.create_thread(func) {
        match thread.resume::<LuaValue>(arg) {
            Ok(yielded_val) => {
                if thread.status() == LuaThreadStatus::Resumable {
                    let wake = yielded_to_wake(yielded_val, std::time::Instant::now());
                    let mut sched = scheduler.lock().unwrap();
                    if let Ok(key) = lua.create_registry_value(thread) {
                        sched.tasks.push(LuaTask {
                            thread_key: key,
                            wake_time: wake,
                        });
                    }
                }
            }
            Err(e) => {
                error!("Luau callback runtime error: {}", e);
                crate::scripting::output::push_error("Event", e.to_string());
            }
        }
    }
}

fn start_script(lua: &Lua, scheduler: &Arc<Mutex<LuaScheduler>>, entity: Entity, code: String, kind: &str) {
    let chunk_name = format!("{kind}[{entity}]");
    match crate::scripting::vm::compiler::compile_code(lua, &code, &chunk_name) {
        Ok(func) => {
            let script_env = match lua.create_table() {
                Ok(t) => t,
                Err(_) => return,
            };
            let meta = match lua.create_table() {
                Ok(m) => m,
                Err(_) => return,
            };
            let globals = lua.globals();
            let _ = meta.set("__index", globals);
            let _ = script_env.set_metatable(Some(meta));

            let script_instance = crate::scripting::userdata::instance::Instance { entity };
            let _ = script_env.set("script", script_instance);

            let _ = func.set_environment(script_env);

            crate::scripting::output::record_script_start();

            match lua.create_thread(func) {
                Ok(thread) => {
                    match thread.resume::<LuaValue>(()) {
                        Ok(yielded_val) => {
                            if thread.status() == LuaThreadStatus::Resumable {
                                let wake = yielded_to_wake(yielded_val, std::time::Instant::now());
                                let mut sched = scheduler.lock().unwrap();
                                if let Ok(key) = lua.create_registry_value(thread) {
                                    sched.tasks.push(LuaTask {
                                        thread_key: key,
                                        wake_time: wake,
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            error!("Luau {kind} runtime error: {}", e);
                            crate::scripting::output::push_error(&chunk_name, e.to_string());
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create thread for {kind}: {}", e);
                    crate::scripting::output::push_error(&chunk_name, format!("Failed to create thread: {}", e));
                }
            }
        }
        Err(e) => {
            error!("Luau {kind} compile error: {}", e);
            crate::scripting::output::push_error(&chunk_name, format!("Compile error: {}", e));
        }
    }
}

pub fn discover_and_run_server_scripts(world: &mut World) {
    let mut scripts_to_run = Vec::new();
    let mut query = world.query::<(Entity, &mut ServerScript)>();
    for (entity, mut script) in query.iter_mut(world) {
        if script.code != script.running_code {
            script.started = false;
        }
        if !script.started && script.enabled {
            script.running_code = script.code.clone();
            script.started = true;
            scripts_to_run.push((entity, script.code.clone()));
        }
    }

    if scripts_to_run.is_empty() {
        return;
    }

    if let Some(server_vm) = world.remove_resource::<ServerScriptVM>() {
        server_vm.lua.set_app_data(WorldRef(world as *mut World));

        for (entity, code) in scripts_to_run {
            start_script(&server_vm.lua, &server_vm.scheduler, entity, code, "ServerScript");
        }

        world.insert_resource(server_vm);
    }
}

pub fn discover_and_run_local_scripts(world: &mut World) {
    let mut scripts_to_run = Vec::new();
    let mut query = world.query::<(Entity, &mut LocalScript)>();
    for (entity, mut script) in query.iter_mut(world) {
        if script.code != script.running_code {
            script.started = false;
        }
        if !script.started && script.enabled {
            script.running_code = script.code.clone();
            script.started = true;
            scripts_to_run.push((entity, script.code.clone()));
        }
    }

    if scripts_to_run.is_empty() {
        return;
    }

    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        client_vm.lua.set_app_data(WorldRef(world as *mut World));

        for (entity, code) in scripts_to_run {
            start_script(&client_vm.lua, &client_vm.scheduler, entity, code, "LocalScript");
        }

        world.insert_resource(client_vm);
    }
}

fn collect_signal_callbacks(
    lua: &Lua,
    registry: &crate::scripting::vm::scheduler::ScriptRegistry,
    key: (bevy::prelude::Entity, &'static str),
    make_arg: impl Fn(&Lua) -> mlua::Result<LuaValue>,
    out: &mut Vec<(LuaFunction, LuaValue)>,
) {
    if let Some(keys) = registry.connections.get(&key) {
        for registry_key in keys {
            if let Ok(func) = lua.registry_value::<LuaFunction>(&**registry_key) {
                if let Ok(arg) = make_arg(lua) {
                    out.push((func, arg));
                }
            }
        }
    }
}

pub fn detect_touched_collisions(
    world: &mut World,
    mut current: Local<std::collections::HashSet<(Entity, Entity)>>,
) {
    current.clear();
    let mut query = world.query::<(Entity, &CollidingEntities)>();
    for (entity, colliding) in query.iter(world) {
        for other in colliding.iter() {
            let pair = if entity < *other { (entity, *other) } else { (*other, entity) };
            current.insert(pair);
        }
    }

    let began: Vec<(Entity, Entity)> = {
        let mut history = world
            .get_resource_mut::<PreviousCollisions>()
            .expect("PreviousCollisions resource must exist");
        let began = current.difference(&history.0).copied().collect();
        std::mem::swap(&mut history.0, &mut current);
        began
    };

    if began.is_empty() {
        return;
    }

    if let Some(server_vm) = world.remove_resource::<ServerScriptVM>() {
        server_vm.lua.set_app_data(WorldRef(world as *mut World));

        let mut callbacks = Vec::new();
        {
            let registry = server_vm.registry.lock().unwrap();
            for &(a, b) in &began {
                collect_signal_callbacks(&server_vm.lua, &registry, (a, "Touched"), |lua| {
                    lua.create_userdata(crate::scripting::userdata::instance::Instance { entity: b })
                        .map(LuaValue::UserData)
                }, &mut callbacks);
                collect_signal_callbacks(&server_vm.lua, &registry, (b, "Touched"), |lua| {
                    lua.create_userdata(crate::scripting::userdata::instance::Instance { entity: a })
                        .map(LuaValue::UserData)
                }, &mut callbacks);
            }
        }

        for (func, arg) in callbacks {
            spawn_and_run_callback(&server_vm.lua, &server_vm.scheduler, func, arg);
        }

        world.insert_resource(server_vm);
    }

    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        client_vm.lua.set_app_data(WorldRef(world as *mut World));

        let mut callbacks = Vec::new();
        {
            let registry = client_vm.registry.lock().unwrap();
            for &(a, b) in &began {
                collect_signal_callbacks(&client_vm.lua, &registry, (a, "Touched"), |lua| {
                    lua.create_userdata(crate::scripting::userdata::instance::Instance { entity: b })
                        .map(LuaValue::UserData)
                }, &mut callbacks);
                collect_signal_callbacks(&client_vm.lua, &registry, (b, "Touched"), |lua| {
                    lua.create_userdata(crate::scripting::userdata::instance::Instance { entity: a })
                        .map(LuaValue::UserData)
                }, &mut callbacks);
            }
        }

        for (func, arg) in callbacks {
            spawn_and_run_callback(&client_vm.lua, &client_vm.scheduler, func, arg);
        }

        world.insert_resource(client_vm);
    }
}

pub fn detect_player_added_events(
    world: &mut World,
    mut last_players: Local<std::collections::HashSet<Entity>>,
) {
    let mut joined = Vec::new();
    {
        let mut query = world.query::<(Entity, &crate::common::net::components::Player)>();
        for (entity, _) in query.iter(world) {
            if !last_players.contains(&entity) {
                joined.push(entity);
            }
        }
        last_players.clear();
        for (entity, _) in query.iter(world) {
            last_players.insert(entity);
        }
    }

    if joined.is_empty() {
        return;
    }

    if let Some(server_vm) = world.remove_resource::<ServerScriptVM>() {
        server_vm.lua.set_app_data(WorldRef(world as *mut World));

        let mut callbacks = Vec::new();
        {
            let registry = server_vm.registry.lock().unwrap();
            let players_entity = crate::scripting::userdata::instance::find_service_entity(world, "Players")
                .unwrap_or(Entity::PLACEHOLDER);
            if let Some(keys) = registry.connections.get(&(players_entity, "PlayerAdded")) {
                for key in keys {
                    if let Ok(func) = server_vm.lua.registry_value::<LuaFunction>(&**key) {
                        for &entity in &joined {
                            if let Ok(inst) = server_vm.lua.create_userdata(
                                crate::scripting::userdata::instance::Instance { entity },
                            ) {
                                callbacks.push((func.clone(), LuaValue::UserData(inst)));
                            }
                        }
                    }
                }
            }
        }

        for (func, arg) in callbacks {
            spawn_and_run_callback(&server_vm.lua, &server_vm.scheduler, func, arg);
        }

        world.insert_resource(server_vm);
    }

    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        client_vm.lua.set_app_data(WorldRef(world as *mut World));

        let mut callbacks = Vec::new();
        {
            let registry = client_vm.registry.lock().unwrap();
            let players_entity = crate::scripting::userdata::instance::find_service_entity(world, "Players")
                .unwrap_or(Entity::PLACEHOLDER);
            if let Some(keys) = registry.connections.get(&(players_entity, "PlayerAdded")) {
                for key in keys {
                    if let Ok(func) = client_vm.lua.registry_value::<LuaFunction>(&**key) {
                        for &entity in &joined {
                            if let Ok(inst) = client_vm.lua.create_userdata(
                                crate::scripting::userdata::instance::Instance { entity },
                            ) {
                                callbacks.push((func.clone(), LuaValue::UserData(inst)));
                            }
                        }
                    }
                }
            }
        }

        for (func, arg) in callbacks {
            spawn_and_run_callback(&client_vm.lua, &client_vm.scheduler, func, arg);
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
        server_vm.lua.set_app_data(WorldRef(world as *mut World));

        let mut callbacks = Vec::new();
        {
            let registry = server_vm.registry.lock().unwrap();
            let workspace_entity = crate::scripting::userdata::instance::find_service_entity(world, "Workspace")
                .unwrap_or(Entity::PLACEHOLDER);

            let drain = |event: &'static str, out: &mut Vec<(LuaFunction, LuaValue)>| {
                collect_signal_callbacks(&server_vm.lua, &registry, (workspace_entity, event), |_| {
                    Ok(LuaValue::Number(delta_secs as f64))
                }, out);
            };
            drain("Heartbeat", &mut callbacks);
            drain("Stepped", &mut callbacks);
        }

        for (func, arg) in callbacks {
            spawn_and_run_callback(&server_vm.lua, &server_vm.scheduler, func, arg);
        }

        crate::scripting::vm::scheduler::run_scheduler_tick(&server_vm.scheduler, &server_vm.lua);

        world.insert_resource(server_vm);
    }

    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        client_vm.lua.set_app_data(WorldRef(world as *mut World));

        let mut callbacks = Vec::new();
        {
            let registry = client_vm.registry.lock().unwrap();
            let workspace_entity = crate::scripting::userdata::instance::find_service_entity(world, "Workspace")
                .unwrap_or(Entity::PLACEHOLDER);

            let drain = |event: &'static str, out: &mut Vec<(LuaFunction, LuaValue)>| {
                collect_signal_callbacks(&client_vm.lua, &registry, (workspace_entity, event), |_| {
                    Ok(LuaValue::Number(delta_secs as f64))
                }, out);
            };
            drain("Heartbeat", &mut callbacks);
            drain("Stepped", &mut callbacks);
        }

        for (func, arg) in callbacks {
            spawn_and_run_callback(&client_vm.lua, &client_vm.scheduler, func, arg);
        }

        crate::scripting::vm::scheduler::run_scheduler_tick(&client_vm.scheduler, &client_vm.lua);

        world.insert_resource(client_vm);
    }
}

fn cache_service_entities(
    mut cache: ResMut<ServiceEntities>,
    query: Query<(Entity, &Name)>,
) {
    if cache.workspace.is_some_and(|entity| {
        !matches!(query.get(entity), Ok((_, name)) if name.as_str() == "Workspace")
    }) {
        cache.workspace = None;
    }
    if cache.players.is_some_and(|entity| {
        !matches!(query.get(entity), Ok((_, name)) if name.as_str() == "Players")
    }) {
        cache.players = None;
    }
    if cache.lighting.is_some_and(|entity| {
        !matches!(query.get(entity), Ok((_, name)) if name.as_str() == "Lighting")
    }) {
        cache.lighting = None;
    }

    if cache.workspace.is_some() && cache.players.is_some() && cache.lighting.is_some() {
        return;
    }

    for (entity, name) in &query {
        match name.as_str() {
            "Workspace" if cache.workspace.is_none() => cache.workspace = Some(entity),
            "Players" if cache.players.is_none() => cache.players = Some(entity),
            "Lighting" if cache.lighting.is_none() => cache.lighting = Some(entity),
            _ => {}
        }
    }
}

fn sweep_stale_connections(world: &mut World) {
    if let Some(server_vm) = world.remove_resource::<ServerScriptVM>() {
        crate::scripting::vm::scheduler::sweep_stale_connections(&server_vm.lua, &server_vm.registry, world);
        world.insert_resource(server_vm);
    }
    if let Some(client_vm) = world.remove_resource::<ClientScriptVM>() {
        crate::scripting::vm::scheduler::sweep_stale_connections(&client_vm.lua, &client_vm.registry, world);
        world.insert_resource(client_vm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::vm::scheduler::ServiceEntities;
    use crate::common::net::components::{Player, PlayersServiceContainer};

    fn cache_app() -> App {
        let mut app = App::new();
        app.init_resource::<ServiceEntities>()
            .add_systems(Update, cache_service_entities);
        app
    }

    #[test]
    fn discovers_service_entities() {
        let mut app = cache_app();
        let workspace = app.world_mut().spawn(Name::new("Workspace")).id();
        let players = app.world_mut().spawn(Name::new("Players")).id();
        let lighting = app.world_mut().spawn(Name::new("Lighting")).id();

        app.update();

        let cache = app.world().resource::<ServiceEntities>();
        assert_eq!(cache.workspace, Some(workspace));
        assert_eq!(cache.players, Some(players));
        assert_eq!(cache.lighting, Some(lighting));
    }

    #[test]
    fn refreshes_renamed_and_recreated_services() {
        let mut app = cache_app();
        let old_workspace = app.world_mut().spawn(Name::new("Workspace")).id();
        let old_players = app.world_mut().spawn(Name::new("Players")).id();
        app.world_mut().spawn(Name::new("Lighting"));
        app.update();

        app.world_mut().entity_mut(old_workspace).insert(Name::new("Renamed"));
        app.world_mut().despawn(old_players);
        let workspace = app.world_mut().spawn(Name::new("Workspace")).id();
        let players = app.world_mut().spawn(Name::new("Players")).id();
        app.update();

        let cache = app.world().resource::<ServiceEntities>();
        assert_eq!(cache.workspace, Some(workspace));
        assert_eq!(cache.players, Some(players));
    }


    fn script_app() -> App {
        let mut app = App::new();
        app.add_plugins(ScriptingPlugin);
        app.world_mut().insert_resource(bevy::time::Time::<()>::default());
        app.world_mut().insert_resource(ServerScriptVM::new());
        app.world_mut().insert_resource(ClientScriptVM::new());
        app
    }

    fn server_global<T: FromLua>(app: &App, name: &str) -> T {
        app.world().resource::<ServerScriptVM>().lua.globals().get(name).unwrap()
    }

    fn client_global<T: FromLua>(app: &App, name: &str) -> T {
        app.world().resource::<ClientScriptVM>().lua.globals().get(name).unwrap()
    }

    #[test]
    fn server_scripts_discover_run_and_restart_on_edit() {
        let mut app = script_app();
        let entity = app
            .world_mut()
            .spawn((
                Name::new("Counter"),
                ServerScript {
                    code: "_G.runs = (_G.runs or 0) + 1".to_string(),
                    ..default()
                },
            ))
            .id();

        app.update();
        assert_eq!(server_global::<i32>(&app, "runs"), 1);

        app.world_mut().entity_mut(entity).insert(ServerScript {
            code: "_G.runs = (_G.runs or 0) + 2".to_string(),
            ..default()
        });
        app.update();
        assert_eq!(server_global::<i32>(&app, "runs"), 3);

        app.update();
        assert_eq!(server_global::<i32>(&app, "runs"), 3);
    }

    #[test]
    fn disabled_scripts_do_not_run() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("Off"),
            ServerScript {
                code: "_G.should_not_run = true".to_string(),
                enabled: false,
                ..default()
            },
        ));

        app.update();
        let value = server_global::<LuaValue>(&app, "should_not_run");
        assert_eq!(value, LuaValue::Nil);
    }

    #[test]
    fn local_scripts_run_on_the_client_vm() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("L"),
            LocalScript {
                code: "_G.client_ran = true".to_string(),
                ..default()
            },
            lightyear::prelude::Replicate::default(),
        ));

        app.update();
        assert!(client_global::<bool>(&app, "client_ran"));
        let value = server_global::<LuaValue>(&app, "client_ran");
        assert_eq!(value, LuaValue::Nil);
    }

    #[test]
    fn scripts_error_without_crashing_the_engine() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("Bad"),
            ServerScript {
                code: "error('script exploded')".to_string(),
                ..default()
            },
        ));
        app.update();
        app.world_mut().spawn((
            Name::new("Good"),
            ServerScript {
                code: "_G.good_ran = true".to_string(),
                ..default()
            },
        ));
        app.update();
        assert!(server_global::<bool>(&app, "good_ran"));
    }

    #[test]
    fn player_added_fires_for_joining_players() {
        let mut app = script_app();
        app.world_mut().spawn((Name::new("Players"), PlayersServiceContainer));
        app.world_mut().spawn((
            Name::new("Connector"),
            ServerScript {
                code: r#"
                    Players.PlayerAdded:Connect(function(p)
                        _G.added = (_G.added or 0) + 1
                        _G.last_added = p.Name
                    end)
                "#.to_string(),
                ..default()
            },
        ));

        app.update();
        assert_eq!(server_global::<LuaValue>(&app, "added"), LuaValue::Nil);

        app.world_mut().spawn((
            Name::new("Bob"),
            Player {
                client_id: 1,
                ..default()
            },
        ));
        app.update();
        assert_eq!(server_global::<i32>(&app, "added"), 1);
        assert_eq!(server_global::<String>(&app, "last_added"), "Bob");

        app.update();
        assert_eq!(server_global::<i32>(&app, "added"), 1);
    }

    #[test]
    fn touched_fires_once_per_contact() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("Sensor"),
            ServerScript {
                code: r#"
                    _G.p = Instance.new("Part")
                    _G.count = 0
                    _G.p.Touched:Connect(function()
                        _G.count = _G.count + 1
                    end)
                "#.to_string(),
                ..default()
            },
        ));
        app.update();

        let part = {
            let ud: mlua::AnyUserData = server_global(&app, "p");
            ud.borrow::<crate::scripting::userdata::instance::Instance>().unwrap().entity
        };
        let other = app.world_mut().spawn(Name::new("Other")).id();

        app.world_mut().entity_mut(part)
            .insert(CollidingEntities(bevy::ecs::entity::EntityHashSet::from([other])));
        app.update();
        assert_eq!(server_global::<i32>(&app, "count"), 1);

        app.update();
        assert_eq!(server_global::<i32>(&app, "count"), 1);

        app.world_mut().entity_mut(part).remove::<CollidingEntities>();
        app.update();

        app.world_mut().entity_mut(part)
            .insert(CollidingEntities(bevy::ecs::entity::EntityHashSet::from([other])));
        app.update();
        assert_eq!(server_global::<i32>(&app, "count"), 2);
    }

    #[test]
    fn run_service_events_fire_every_frame() {
        let mut app = script_app();
        app.world_mut().spawn(Name::new("Workspace"));
        app.world_mut().spawn((
            Name::new("Connector"),
            ServerScript {
                code: r#"
                    RunService.Heartbeat:Connect(function(dt)
                        _G.hb = (_G.hb or 0) + 1
                        _G.last_dt = dt
                    end)
                    RunService.Stepped:Connect(function(dt)
                        _G.stepped = (_G.stepped or 0) + 1
                    end)
                "#.to_string(),
                ..default()
            },
        ));

        app.update();
        assert_eq!(server_global::<i32>(&app, "hb"), 1);
        assert_eq!(server_global::<i32>(&app, "stepped"), 1);
        let dt: f64 = server_global(&app, "last_dt");
        assert!(dt >= 0.0, "Heartbeat must pass the frame delta");

        app.update();
        assert_eq!(server_global::<i32>(&app, "hb"), 2);
    }

    #[test]
    fn stale_connections_swept_end_to_end() {
        let mut app = script_app();
        app.world_mut().spawn((
            Name::new("Sensor"),
            ServerScript {
                code: r#"
                    _G.p = Instance.new("Part")
                    _G.p.Touched:Connect(function() end)
                "#.to_string(),
                ..default()
            },
        ));
        app.update();

        let part = {
            let ud: mlua::AnyUserData = server_global(&app, "p");
            ud.borrow::<crate::scripting::userdata::instance::Instance>().unwrap().entity
        };
        assert!(app.world().resource::<ServerScriptVM>()
            .registry.lock().unwrap()
            .connections.contains_key(&(part, "Touched")));

        app.world_mut().entity_mut(part).despawn();
        app.update();
        assert!(!app.world().resource::<ServerScriptVM>()
            .registry.lock().unwrap()
            .connections.contains_key(&(part, "Touched")));
    }
}



