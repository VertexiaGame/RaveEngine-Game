use bevy::prelude::*;
use mlua::prelude::*;
use super::vector3::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CFrame {
    pub position: Vec3,
    pub rotation: Quat,
}

impl CFrame {
    pub fn to_euler(&self) -> (f32, f32, f32) {
        let (x, y, z) = self.rotation.to_euler(EulerRot::XYZ);
        (x + 0.0, y + 0.0, z + 0.0)
    }
}

impl LuaUserData for CFrame {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Mul, |lua, this, other: LuaValue| {
            match other {
                LuaValue::UserData(ud) => {
                    if let Ok(other_cf) = ud.borrow::<CFrame>() {
                        let new_pos = this.position + this.rotation.mul_vec3(other_cf.position);
                        let new_rot = this.rotation * other_cf.rotation;
                        lua.create_userdata(CFrame { position: new_pos, rotation: new_rot }).map(LuaValue::UserData)
                    } else if let Ok(other_vec) = ud.borrow::<Vector3>() {
                        let new_pos = this.position + this.rotation.mul_vec3(other_vec.0);
                        lua.create_userdata(Vector3(new_pos)).map(LuaValue::UserData)
                    } else {
                        Err(mlua::Error::RuntimeError("Unsupported multiplier for CFrame".to_string()))
                    }
                }
                _ => Err(mlua::Error::RuntimeError("Unsupported multiplier for CFrame".to_string())),
            }
        });

        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, _: ()| {
            let (x, y, z) = (this.position.x, this.position.y, this.position.z);
            let (rx, ry, rz) = this.to_euler();
            Ok(format!("{}, {}, {}, {}, {}, {}", x, y, z, rx, ry, rz))
        });

        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            let this = *this;
            match key.as_str() {
                "Position" => Ok(Some(LuaValue::UserData(lua.create_userdata(Vector3(this.position))?))),
                "LookVector" => Ok(Some(LuaValue::UserData(lua.create_userdata(Vector3(this.rotation.mul_vec3(Vec3::NEG_Z).normalize_or_zero()))?))),
                "RightVector" => Ok(Some(LuaValue::UserData(lua.create_userdata(Vector3(this.rotation.mul_vec3(Vec3::X).normalize_or_zero()))?))),
                "UpVector" => Ok(Some(LuaValue::UserData(lua.create_userdata(Vector3(this.rotation.mul_vec3(Vec3::Y).normalize_or_zero()))?))),
                "Lerp" => Ok(Some(LuaValue::Function(lua.create_function(move |lua, args: LuaMultiValue| {
                    let goal = args.iter().rev().find_map(|v| match v {
                        LuaValue::UserData(ud) => ud.borrow::<CFrame>().ok().map(|r| *r),
                        _ => None,
                    }).ok_or_else(|| mlua::Error::RuntimeError("CFrame expected for Lerp".to_string()))?;
                    let alpha = args.iter().rev().find_map(|v| match v {
                        LuaValue::Number(n) => Some(*n as f32),
                        LuaValue::Integer(i) => Some(*i as f32),
                        _ => None,
                    }).ok_or_else(|| mlua::Error::RuntimeError("Lerp expects an alpha number".to_string()))?;
                    let pos = this.position.lerp(goal.position, alpha);
                    let rot = this.rotation.slerp(goal.rotation, alpha);
                    lua.create_userdata(CFrame { position: pos, rotation: rot }).map(LuaValue::UserData)
                })?))),
                "ToEulerAnglesXYZ" => Ok(Some(LuaValue::Function(lua.create_function(move |lua, _: ()| {
                    let (rx, ry, rz) = this.to_euler();
                    lua.create_userdata(Vector3(Vec3::new(rx, ry, rz))).map(LuaValue::UserData)
                })?))),
                _ => Ok(None),
            }
        });
    }
}
