use bevy::prelude::*;
use mlua::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color3(pub Color);

impl LuaUserData for Color3 {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, _: ()| {
            let srgba = this.0.to_srgba();
            Ok(format!("{}, {}, {}", srgba.red, srgba.green, srgba.blue))
        });
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaAnyUserData| {
            if let Ok(other_col) = other.borrow::<Color3>() {
                Ok(this.0 == other_col.0)
            } else {
                Ok(false)
            }
        });

        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            let this = *this;
            let srgba = this.0.to_srgba();
            match key.as_str() {
                "R" | "r" => Ok(Some(LuaValue::Number(srgba.red as f64))),
                "G" | "g" => Ok(Some(LuaValue::Number(srgba.green as f64))),
                "B" | "b" => Ok(Some(LuaValue::Number(srgba.blue as f64))),
                "ToHex" => Ok(Some(LuaValue::Function(lua.create_function(move |_, _: ()| {
                    let srgba = this.0.to_srgba();
                    Ok(format!(
                        "#{:02X}{:02X}{:02X}",
                        (srgba.red * 255.0).round() as u32,
                        (srgba.green * 255.0).round() as u32,
                        (srgba.blue * 255.0).round() as u32
                    ))
                })?))),
                _ => Ok(None),
            }
        });
    }
}
