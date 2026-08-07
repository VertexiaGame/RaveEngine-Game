use bevy::prelude::*;
use mlua::prelude::*;

#[derive(Clone, Copy)]
pub struct WorkspaceService;

impl LuaUserData for WorkspaceService {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Eq, |_, _, other: LuaAnyUserData| {
            Ok(other.is::<WorkspaceService>())
        });

        methods.add_meta_method(LuaMetaMethod::Index, |lua, _, key: String| {
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &*world_ref.0 };

            match key.as_str() {
                "Gravity" => {
                    let g = world.get_resource::<avian3d::prelude::Gravity>().map(|g| -g.0.y / 0.28).unwrap_or(186.9);
                    let g = (g * 100.0).round() / 100.0;
                    Ok(LuaValue::Number(g as f64))
                }
                "ClassName" => Ok(LuaValue::String(lua.create_string("Workspace")?)),
                "Name" => Ok(LuaValue::String(lua.create_string("Workspace")?)),
                _ => {
                    if let Some(workspace_entity) = crate::scripting::userdata::instance::find_service_entity(world, "Workspace") {
                        let instance = crate::scripting::userdata::instance::Instance { entity: workspace_entity };
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

        methods.add_meta_method(LuaMetaMethod::NewIndex, |lua, _, (key, value): (String, LuaValue)| {
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &mut *world_ref.0 };

            match key.as_str() {
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