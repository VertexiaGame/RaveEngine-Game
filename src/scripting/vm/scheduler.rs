use std::time::Instant;
use std::collections::{VecDeque, HashSet};
use std::sync::{Arc, Mutex};
use bevy::prelude::{Entity, Resource, World};
use bevy::log::*;
use mlua::prelude::*;

#[derive(Resource, Default)]
pub struct ServiceEntities {
    pub workspace: Option<Entity>,
    pub players: Option<Entity>,
    pub lighting: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct PreviousCollisions(pub HashSet<(Entity, Entity)>);

pub struct LuaTask {
    pub thread_key: mlua::RegistryKey,
    pub wake_time: Option<Instant>,
}

pub struct LuaScheduler {
    pub tasks: Vec<LuaTask>,
    pub deferred: VecDeque<mlua::RegistryKey>,
}

pub struct SchedulerRef(pub Arc<Mutex<LuaScheduler>>);

pub struct ScriptRegistry {
    pub connections: std::collections::HashMap<(Entity, &'static str), Vec<Arc<mlua::RegistryKey>>>,
}

#[derive(Clone)]
pub struct ScriptRegistryRef(pub Arc<Mutex<ScriptRegistry>>);

pub fn yielded_to_wake(yielded: LuaValue, now: Instant) -> Option<Instant> {
    match yielded {
        LuaValue::Number(n) => Some(now + std::time::Duration::from_secs_f64(n as f64)),
        LuaValue::Integer(i) => Some(now + std::time::Duration::from_secs_f64(i as f64)),
        _ => None,
    }
}

impl LuaScheduler {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            deferred: VecDeque::new(),
        }
    }

    pub fn collect_ready(&mut self, now: Instant, out: &mut Vec<mlua::RegistryKey>) {
        let mut i = 0;
        while i < self.tasks.len() {
            if self.tasks[i].wake_time.map_or(true, |wake| now >= wake) {
                out.push(self.tasks.swap_remove(i).thread_key);
            } else {
                i += 1;
            }
        }
        while let Some(key) = self.deferred.pop_front() {
            out.push(key);
        }
    }
}

pub fn run_scheduler_tick(scheduler: &Arc<Mutex<LuaScheduler>>, lua: &Lua) {
    let now = Instant::now();
    let mut ready = Vec::new();
    {
        let mut sched = scheduler.lock().unwrap();
        sched.collect_ready(now, &mut ready);
    }

    let mut requeue = Vec::new();
    for key in ready {
        match lua.registry_value::<LuaThread>(&key) {
            Ok(thread) => match thread.resume::<LuaValue>(()) {
                Ok(yielded) => {
                    if thread.status() == LuaThreadStatus::Resumable {
                        requeue.push(LuaTask {
                            thread_key: key,
                            wake_time: yielded_to_wake(yielded, now),
                        });
                    } else {
                        let _ = lua.remove_registry_value(key);
                    }
                }
                Err(e) => {
                    error!("Luau scheduler execution error: {}", e);
                    crate::scripting::output::push_error("Scheduler", e.to_string());
                    let _ = lua.remove_registry_value(key);
                }
            },
            Err(_) => {
                let _ = lua.remove_registry_value(key);
            }
        }
    }

    if !requeue.is_empty() {
        let mut sched = scheduler.lock().unwrap();
        sched.tasks.append(&mut requeue);
    }
}

pub fn sweep_stale_connections(lua: &Lua, registry: &Arc<Mutex<ScriptRegistry>>, world: &World) {
    let mut dead_keys = Vec::new();
    {
        let mut registry = registry.lock().unwrap();
        let stale: Vec<(Entity, &'static str)> = registry
            .connections
            .keys()
            .filter(|(entity, _)| world.get_entity(*entity).is_err())
            .copied()
            .collect();
        for key in stale {
            if let Some(conns) = registry.connections.remove(&key) {
                for arc in conns {
                    if let Ok(key) = Arc::try_unwrap(arc) {
                        dead_keys.push(key);
                    }
                }
            }
        }
    }
    for key in dead_keys {
        let _ = lua.remove_registry_value(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use std::time::Duration;

    fn task(key: mlua::RegistryKey, wake: Option<Instant>) -> LuaTask {
        LuaTask { thread_key: key, wake_time: wake }
    }

    fn vm() -> Lua {
        Lua::new()
    }

    #[test]
    fn yielded_to_wake_handles_numeric_and_other_values() {
        let lua = vm();
        let now = Instant::now();

        let wake = yielded_to_wake(LuaValue::Number(0.5), now);
        assert!(wake.is_some());
        assert_eq!(wake.unwrap().duration_since(now), Duration::from_millis(500));

        let wake = yielded_to_wake(LuaValue::Integer(3), now);
        assert!(wake.is_some());
        assert_eq!(wake.unwrap().duration_since(now), Duration::from_secs(3));

        assert!(yielded_to_wake(LuaValue::Nil, now).is_none());
        assert!(yielded_to_wake(LuaValue::String(lua.create_string("later").unwrap()), now).is_none());
    }

    #[test]
    fn collect_ready_moves_due_and_deferred_but_keeps_future_tasks() {
        let lua = vm();
        let due_key = lua.create_registry_value(42).unwrap();
        let past_key = lua.create_registry_value("past").unwrap();
        let future_key = lua.create_registry_value("future").unwrap();
        let deferred_key = lua.create_registry_value("deferred").unwrap();

        let now = Instant::now();
        let mut sched = LuaScheduler {
            tasks: vec![
                task(due_key, None),
                task(past_key, Some(now - Duration::from_secs(1))),
                task(future_key, Some(now + Duration::from_secs(60))),
            ],
            deferred: VecDeque::from([deferred_key]),
        };

        let mut out = Vec::new();
        sched.collect_ready(now, &mut out);

        assert_eq!(out.len(), 3);
        assert_eq!(sched.tasks.len(), 1);
        assert!(sched.deferred.is_empty());
    }

    #[test]
    fn run_scheduler_tick_completes_threads_and_frees_keys() {
        let lua = vm();
        let func: LuaFunction = lua.load("_G.completed = true").into_function().unwrap();
        let thread = lua.create_thread(func).unwrap();
        let key = lua.create_registry_value(thread).unwrap();

        let scheduler = Arc::new(Mutex::new(LuaScheduler {
            tasks: vec![task(key, None)],
            deferred: VecDeque::new(),
        }));

        run_scheduler_tick(&scheduler, &lua);

        assert!(lua.globals().get::<bool>("completed").unwrap());
        assert!(scheduler.lock().unwrap().tasks.is_empty());
    }

    #[test]
    fn run_scheduler_tick_requeues_yielding_threads_with_wake() {
        let lua = vm();
        let func: LuaFunction = lua
            .load("return coroutine.yield(0.01)")
            .into_function()
            .unwrap();
        let thread = lua.create_thread(func).unwrap();
        let key = lua.create_registry_value(thread).unwrap();

        let scheduler = Arc::new(Mutex::new(LuaScheduler {
            tasks: vec![task(key, None)],
            deferred: VecDeque::new(),
        }));

        run_scheduler_tick(&scheduler, &lua);

        {
            let sched = scheduler.lock().unwrap();
            assert_eq!(sched.tasks.len(), 1);
            let requeued = &sched.tasks[0];
            assert!(requeued.wake_time.is_some());
            assert!(requeued.wake_time.unwrap() > Instant::now());
        }

        std::thread::sleep(Duration::from_millis(20));
        run_scheduler_tick(&scheduler, &lua);
        assert!(scheduler.lock().unwrap().tasks.is_empty());
    }

    #[test]
    fn run_scheduler_tick_drops_threads_that_error_and_continues() {
        let lua = vm();
        let bad: LuaFunction = lua.load("error('boom')").into_function().unwrap();
        let bad_thread = lua.create_thread(bad).unwrap();
        let bad_key = lua.create_registry_value(bad_thread).unwrap();

        let good: LuaFunction = lua.load("ok_ran = true").into_function().unwrap();
        let good_thread = lua.create_thread(good).unwrap();
        let good_key = lua.create_registry_value(good_thread).unwrap();

        let scheduler = Arc::new(Mutex::new(LuaScheduler {
            tasks: vec![task(bad_key, None), task(good_key, None)],
            deferred: VecDeque::new(),
        }));

        run_scheduler_tick(&scheduler, &lua);

        assert!(lua.globals().get::<bool>("ok_ran").unwrap());
        assert!(scheduler.lock().unwrap().tasks.is_empty());
    }

    #[test]
    fn deferred_threads_run_on_next_tick() {
        let lua = vm();
        let func: LuaFunction = lua.load("_G.deferred_ran = true").into_function().unwrap();
        let thread = lua.create_thread(func).unwrap();
        let key = lua.create_registry_value(thread).unwrap();

        let scheduler = Arc::new(Mutex::new(LuaScheduler {
            tasks: Vec::new(),
            deferred: VecDeque::from([key]),
        }));

        run_scheduler_tick(&scheduler, &lua);
        assert!(lua.globals().get::<bool>("deferred_ran").unwrap());
        assert!(scheduler.lock().unwrap().deferred.is_empty());
    }

    #[test]
    fn sweep_removes_only_stale_connections() {
        let mut world = World::new();
        let alive = world.spawn(Name::new("Alive")).id();
        let dead = world.spawn(Name::new("Dead")).id();
        world.entity_mut(dead).despawn();

        let lua = vm();
        let registry = Arc::new(Mutex::new(ScriptRegistry {
            connections: std::collections::HashMap::new(),
        }));
        let alive_key = Arc::new(lua.create_registry_value("alive-cb").unwrap());
        let dead_key = Arc::new(lua.create_registry_value("dead-cb").unwrap());
        {
            let mut reg = registry.lock().unwrap();
            reg.connections.insert((alive, "Touched"), vec![alive_key.clone()]);
            reg.connections.insert((dead, "Touched"), vec![dead_key]);
        }

        sweep_stale_connections(&lua, &registry, &world);

        let reg = registry.lock().unwrap();
        assert!(reg.connections.contains_key(&(alive, "Touched")));
        assert!(!reg.connections.contains_key(&(dead, "Touched")));
    }

    #[test]
    fn sweep_keeps_connections_still_referenced_by_connection_tables() {
        let mut world = World::new();
        let entity = world.spawn(Name::new("E")).id();
        let lua = vm();
        let registry = Arc::new(Mutex::new(ScriptRegistry {
            connections: std::collections::HashMap::new(),
        }));

        let key = Arc::new(lua.create_registry_value("cb").unwrap());
        let table_handle = key.clone();
        {
            let mut reg = registry.lock().unwrap();
            reg.connections.insert((entity, "Touched"), vec![key]);
        }

        world.entity_mut(entity).despawn();
        sweep_stale_connections(&lua, &registry, &world);

        assert!(registry.lock().unwrap().connections.is_empty());
        let _ = table_handle;
        assert!(lua.registry_value::<String>(&table_handle).is_ok());
    }
}


