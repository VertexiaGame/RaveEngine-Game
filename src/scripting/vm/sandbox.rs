use std::cell::{Cell, RefCell};
use mlua::prelude::*;
use mlua::VmState;

pub const VM_MEMORY_LIMIT: usize = 512 * 1024 * 1024;
pub const SCRIPT_INSTRUCTION_BUDGET: u64 = 1_000_000;
pub const MAX_SCRIPT_CODE_BYTES: usize = 64 * 1024;
pub const MAX_PENDING_TASKS: usize = 10_000;
pub const MAX_CONNECTIONS_PER_ENTITY: usize = 64;
pub const MAX_ENTITY_SPAWNS_PER_TICK: u32 = 2_000;
pub const MAX_ENTITY_SPAWNS_TOTAL: u64 = 100_000;
pub const MAX_PRINT_PER_TICK: u32 = 2_000;

pub struct ExecutionBudget(pub Cell<u64>);

pub struct SpawnBudget {
    pub per_tick: Cell<u32>,
    pub total: Cell<u64>,
}

pub struct PrintBudget(pub Cell<u32>);

pub struct CallerFrame(pub RefCell<Option<(String, Option<u32>)>>);

pub fn reset_tick_budgets(lua: &Lua) {
    if let Some(budget) = lua.app_data_ref::<ExecutionBudget>() {
        budget.0.set(SCRIPT_INSTRUCTION_BUDGET);
    }
    if let Some(budget) = lua.app_data_ref::<SpawnBudget>() {
        budget.per_tick.set(MAX_ENTITY_SPAWNS_PER_TICK);
    }
    if let Some(budget) = lua.app_data_ref::<PrintBudget>() {
        budget.0.set(MAX_PRINT_PER_TICK);
    }
}

pub fn set_caller_frame(lua: &Lua, source: String, line: Option<u32>) {
    if let Some(frame) = lua.app_data_ref::<CallerFrame>() {
        *frame.0.borrow_mut() = Some((source, line));
    }
}

pub fn current_caller_frame(lua: &Lua) -> (String, Option<u32>) {
    match lua.app_data_ref::<CallerFrame>() {
        Some(frame) => frame.0.borrow().clone().unwrap_or_else(|| ("Script".to_string(), None)),
        None => ("Script".to_string(), None),
    }
}

pub fn try_spawn_entity(lua: &Lua) -> Result<(), mlua::Error> {
    let budget = lua
        .app_data_ref::<SpawnBudget>()
        .ok_or_else(|| mlua::Error::RuntimeError("spawn budget unavailable".to_string()))?;
    if budget.per_tick.get() == 0 {
        return Err(mlua::Error::RuntimeError(
            "entity spawn limit exceeded for this tick".to_string(),
        ));
    }
    if budget.total.get() >= MAX_ENTITY_SPAWNS_TOTAL {
        return Err(mlua::Error::RuntimeError(
            "entity spawn limit exceeded for this session".to_string(),
        ));
    }
    budget.per_tick.set(budget.per_tick.get() - 1);
    budget.total.set(budget.total.get() + 1);
    Ok(())
}

pub fn try_print(lua: &Lua) -> bool {
    match lua.app_data_ref::<PrintBudget>() {
        Some(budget) if budget.0.get() > 0 => {
            budget.0.set(budget.0.get() - 1);
            true
        }
        _ => false,
    }
}

pub fn setup_sandbox(lua: &Lua) {
    lua.set_app_data(ExecutionBudget(Cell::new(SCRIPT_INSTRUCTION_BUDGET)));
    lua.set_app_data(SpawnBudget {
        per_tick: Cell::new(MAX_ENTITY_SPAWNS_PER_TICK),
        total: Cell::new(0),
    });
    lua.set_app_data(PrintBudget(Cell::new(MAX_PRINT_PER_TICK)));
    lua.set_app_data(CallerFrame(RefCell::new(None)));

    lua.set_interrupt(|lua| {
        let budget = match lua.app_data_ref::<ExecutionBudget>() {
            Some(budget) => budget,
            None => return Ok(VmState::Continue),
        };
        let remaining = budget.0.get();
        if remaining == 0 {
            return Err(mlua::Error::RuntimeError(
                "script instruction budget exceeded (possible infinite loop)".to_string(),
            ));
        }
        budget.0.set(remaining - 1);
        Ok(VmState::Continue)
    });
}