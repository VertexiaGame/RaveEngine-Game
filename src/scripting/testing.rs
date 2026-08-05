
use bevy::prelude::*;
use mlua::prelude::*;
use crate::scripting::vm::scheduler::{LuaTask, yielded_to_wake};
use crate::scripting::vm::server_vm::{ServerScriptVM, WorldRef};
use std::time::{Duration, Instant};

pub fn test_world() -> World {
    let mut world = World::new();
    world.init_resource::<crate::scripting::vm::scheduler::ServiceEntities>();
    world
}

pub fn test_vm(world: &mut World) -> ServerScriptVM {
    let vm = ServerScriptVM::new();
    vm.lua.set_app_data(WorldRef(world as *mut World));
    vm
}

pub fn run_script(vm: &ServerScriptVM, code: &str) {
    try_script(vm, code).unwrap();
}

pub fn try_script(vm: &ServerScriptVM, code: &str) -> mlua::Result<()> {
    let func = crate::scripting::vm::compiler::compile_code(&vm.lua, code, "test")?;
    let thread = vm.lua.create_thread(func)?;
    match thread.resume::<LuaValue>(()) {
        Ok(yielded) => {
            if thread.status() == LuaThreadStatus::Resumable {
                let key = vm.lua.create_registry_value(thread)?;
                vm.scheduler.lock().unwrap().tasks.push(LuaTask {
                    thread_key: key,
                    wake_time: yielded_to_wake(yielded, Instant::now()),
                });
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn eval<T: FromLuaMulti>(vm: &ServerScriptVM, code: &str) -> T {
    vm.lua.load(code).eval().unwrap()
}

pub fn tick(vm: &ServerScriptVM) {
    crate::scripting::vm::scheduler::run_scheduler_tick(&vm.scheduler, &vm.lua);
}

pub fn advance(vm: &ServerScriptVM, ms: u64, ticks: usize) {
    for _ in 0..ticks {
        std::thread::sleep(Duration::from_millis(ms));
        tick(vm);
    }
}

pub fn global<T: FromLua>(vm: &ServerScriptVM, name: &str) -> T {
    vm.lua.globals().get(name).unwrap()
}

pub fn entity_of(vm: &ServerScriptVM, global_name: &str) -> Entity {
    let ud: mlua::AnyUserData = global(vm, global_name);
    let inst = ud.borrow::<crate::scripting::userdata::instance::Instance>().unwrap();
    inst.entity
}

pub fn spawn_brick(world: &mut World, name: &str) -> Entity {
    use crate::common::game::bricks::components::{Brick, BrickColor, BrickPhysics};
    world
        .spawn((
            Name::new(name.to_string()),
            Transform::default(),
            Brick,
            BrickPhysics::default(),
            BrickColor::default(),
        ))
        .id()
}
