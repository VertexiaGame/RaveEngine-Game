use std::time::Instant;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use bevy::prelude::{Entity, Resource};
use bevy::log::*;
use mlua::prelude::*;

#[derive(Resource, Default)]
pub struct ServiceEntities {
    pub workspace: Option<Entity>,
    pub players: Option<Entity>,
    pub lighting: Option<Entity>,
}

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
    pub connections: std::collections::HashMap<(Entity, String), Vec<std::sync::Arc<mlua::RegistryKey>>>,
}

#[derive(Clone)]
pub struct ScriptRegistryRef(pub Arc<Mutex<ScriptRegistry>>);

impl LuaScheduler {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            deferred: VecDeque::new(),
        }
    }

    pub fn run_tick(&mut self, lua: &mlua::Lua) {
        let now = Instant::now();

        let old_tasks = std::mem::take(&mut self.tasks);
        let mut tasks_to_run = Vec::new();
        for task in old_tasks {
            if let Some(wake) = task.wake_time {
                if now >= wake {
                    tasks_to_run.push(task.thread_key);
                } else {
                    self.tasks.push(task);
                }
            } else {
                tasks_to_run.push(task.thread_key);
            }
        }

        for thread_key in tasks_to_run {
            let thread_opt: Result<LuaThread, _> = lua.registry_value(&thread_key);
            if let Ok(thread) = thread_opt {
                match thread.resume::<Option<f32>>(()) {
                    Ok(yielded_val) => {
                        if thread.status() == LuaThreadStatus::Resumable {
                            let wake = yielded_val.map(|sec| now + std::time::Duration::from_secs_f64(sec as f64));
                            self.tasks.push(LuaTask {
                                thread_key,
                                wake_time: wake,
                            });
                        } else {
                            let _ = lua.remove_registry_value(thread_key);
                        }
                    }
                    Err(e) => {
                        error!("Luau scheduler execution error: {}", e);
                        let _ = lua.remove_registry_value(thread_key);
                    }
                }
            } else {
                let _ = lua.remove_registry_value(thread_key);
            }
        }

        while let Some(thread_key) = self.deferred.pop_front() {
            let thread_opt: Result<LuaThread, _> = lua.registry_value(&thread_key);
            if let Ok(thread) = thread_opt {
                match thread.resume::<Option<f32>>(()) {
                    Ok(yielded_val) => {
                        if thread.status() == LuaThreadStatus::Resumable {
                            let wake = yielded_val.map(|sec| now + std::time::Duration::from_secs_f64(sec as f64));
                            self.tasks.push(LuaTask {
                                thread_key,
                                wake_time: wake,
                            });
                        } else {
                            let _ = lua.remove_registry_value(thread_key);
                        }
                    }
                    Err(e) => {
                        error!("Luau scheduler execution error: {}", e);
                        let _ = lua.remove_registry_value(thread_key);
                    }
                }
            } else {
                let _ = lua.remove_registry_value(thread_key);
            }
        }
    }
}