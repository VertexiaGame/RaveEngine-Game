use bevy::prelude::*;
use mlua::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3(pub Vec3);

impl LuaUserData for Vector3 {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Add, |_, this, other: LuaAnyUserData| {
            if let Ok(other_vec) = other.borrow::<Vector3>() {
                Ok(Vector3(this.0 + other_vec.0))
            } else {
                Err(mlua::Error::RuntimeError("Vector3 expected for Add".to_string()))
            }
        });
        methods.add_meta_method(LuaMetaMethod::Sub, |_, this, other: LuaAnyUserData| {
            if let Ok(other_vec) = other.borrow::<Vector3>() {
                Ok(Vector3(this.0 - other_vec.0))
            } else {
                Err(mlua::Error::RuntimeError("Vector3 expected for Sub".to_string()))
            }
        });
        methods.add_meta_method(LuaMetaMethod::Mul, |_, this, scale: f32| {
            Ok(Vector3(this.0 * scale))
        });
        methods.add_meta_method(LuaMetaMethod::Div, |_, this, scale: f32| {
            Ok(Vector3(this.0 / scale))
        });
        methods.add_meta_method(LuaMetaMethod::Unm, |_, this, _: ()| {
            Ok(Vector3(-this.0))
        });
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, _: ()| {
            Ok(format!("{}, {}, {}", this.0.x, this.0.y, this.0.z))
        });
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaAnyUserData| {
            if let Ok(other_vec) = other.borrow::<Vector3>() {
                Ok(this.0 == other_vec.0)
            } else {
                Ok(false)
            }
        });

        fn last_vector_arg(args: &LuaMultiValue) -> Option<Vector3> {
            args.iter().rev().find_map(|v| match v {
                LuaValue::UserData(ud) => ud.borrow::<Vector3>().ok().map(|r| *r),
                _ => None,
            })
        }

        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            let this = *this;
            match key.as_str() {
                "X" | "x" => Ok(LuaValue::Number(this.0.x as f64)),
                "Y" | "y" => Ok(LuaValue::Number(this.0.y as f64)),
                "Z" | "z" => Ok(LuaValue::Number(this.0.z as f64)),
                "Magnitude" => Ok(LuaValue::Number(this.0.length() as f64)),
                "Unit" => Ok(LuaValue::UserData(lua.create_userdata(Vector3(this.0.normalize_or_zero()))?)),
                "Dot" => Ok(LuaValue::Function(lua.create_function(move |_, args: LuaMultiValue| {
                    let other = last_vector_arg(&args)
                        .ok_or_else(|| mlua::Error::RuntimeError("Vector3 expected for Dot".to_string()))?;
                    Ok(this.0.dot(other.0))
                })?)),
                "Cross" => Ok(LuaValue::Function(lua.create_function(move |lua, args: LuaMultiValue| {
                    let other = last_vector_arg(&args)
                        .ok_or_else(|| mlua::Error::RuntimeError("Vector3 expected for Cross".to_string()))?;
                    lua.create_userdata(Vector3(this.0.cross(other.0))).map(LuaValue::UserData)
                })?)),
                "Lerp" => Ok(LuaValue::Function(lua.create_function(move |lua, args: LuaMultiValue| {
                    let other = last_vector_arg(&args)
                        .ok_or_else(|| mlua::Error::RuntimeError("Vector3 expected for Lerp".to_string()))?;
                    let alpha = args.iter().rev().find_map(|v| match v {
                        LuaValue::Number(n) => Some(*n as f32),
                        LuaValue::Integer(i) => Some(*i as f32),
                        _ => None,
                    }).ok_or_else(|| mlua::Error::RuntimeError("Lerp expects an alpha number".to_string()))?;
                    lua.create_userdata(Vector3(this.0.lerp(other.0, alpha))).map(LuaValue::UserData)
                })?)),
                _ => Ok(LuaValue::Nil),
            }
        });
    }
}
