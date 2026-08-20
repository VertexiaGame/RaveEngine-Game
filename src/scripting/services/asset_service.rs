use bevy::prelude::*;
use mlua::prelude::*;
use crate::common::game::assets::components::Image;
use crate::scripting::userdata::instance::Instance;

#[derive(Clone, Copy)]
pub struct AssetService;

impl LuaUserData for AssetService {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Eq, |_, _, other: LuaAnyUserData| {
            Ok(other.is::<AssetService>())
        });

        methods.add_meta_method(LuaMetaMethod::Index, |lua, _, key: mlua::LuaString| {
            match key.to_str()?.as_ref() {
                "ClassName" => Ok(LuaValue::String(lua.create_string("AssetService")?)),
                "Name" => Ok(LuaValue::String(lua.create_string("AssetService")?)),
                _ => Ok(LuaValue::Nil),
            }
        });

        methods.add_method("GetAsset", |lua, _, asset_id: u32| {
            crate::scripting::vm::sandbox::try_spawn_entity(lua)?;
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &mut *world_ref.0 };
            let entity = world
                .spawn((
                    Name::new("Image"),
                    Transform::default(),
                    Image {
                        asset_id,
                        face: None,
                    },
                    lightyear::prelude::Replicate::default(),
                ))
                .id();
            lua.create_userdata(Instance { entity }).map(LuaValue::UserData)
        });
    }
}