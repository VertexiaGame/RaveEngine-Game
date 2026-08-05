use bevy::prelude::*;
use mlua::prelude::*;
use crate::client::sky::LightingConfig;
use crate::scripting::userdata::color3::Color3;

#[derive(Clone, Copy)]
pub struct LightingService;

fn config(world: &World) -> LightingConfig {
    world.get_resource::<LightingConfig>().cloned().unwrap_or_default()
}

fn as_f64(value: &LuaValue) -> Option<f64> {
    match value {
        LuaValue::Number(n) => Some(*n),
        LuaValue::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

fn parse_time_of_day(s: &str) -> Option<f32> {
    let parts: Vec<&str> = s.split(':').collect();
    let hours: f32 = parts.first()?.trim().parse().ok()?;
    let minutes: f32 = parts.get(1).map(|p| p.trim().parse().ok()).unwrap_or(Some(0.0))?;
    if !(0.0..=24.0).contains(&hours) || !(0.0..60.0).contains(&minutes) {
        return None;
    }
    Some((hours + minutes / 60.0).rem_euclid(24.0))
}

fn format_time_of_day(time_of_day: f32) -> String {
    let hours = (time_of_day.floor() as u32) % 24;
    let minutes = ((time_of_day - time_of_day.floor()) * 60.0).round() as u32;
    format!("{:02}:{:02}:00", hours, minutes)
}

impl LuaUserData for LightingService {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Eq, |_, _, other: LuaAnyUserData| {
            Ok(other.is::<LightingService>())
        });

        methods.add_meta_method(LuaMetaMethod::Index, |lua, _, key: String| {
            let world_ref = lua.app_data_ref::<crate::scripting::vm::server_vm::WorldRef>().unwrap();
            let world = unsafe { &*world_ref.0 };
            let config = config(world);

            match key.as_str() {
                "ClassName" => Ok(LuaValue::String(lua.create_string("Lighting")?)),
                "Name" => Ok(LuaValue::String(lua.create_string("Lighting")?)),
                "ClockTime" => Ok(LuaValue::Number(config.time_of_day as f64)),
                "TimeOfDay" => Ok(LuaValue::String(lua.create_string(format_time_of_day(config.time_of_day))?)),
                "Latitude" => Ok(LuaValue::Number(config.latitude as f64)),
                "SunAngularRadius" => Ok(LuaValue::Number(config.sun_angular_radius as f64)),
                "MoonAngularRadius" => Ok(LuaValue::Number(config.moon_angular_radius as f64)),
                "StarDensity" => Ok(LuaValue::Number(config.star_density as f64)),
                "NightAmbient" => lua.create_userdata(Color3(config.night_ambient)).map(LuaValue::UserData),
                "SunBrightness" => Ok(LuaValue::Number(config.sun_illuminance as f64)),
                "MoonBrightness" => Ok(LuaValue::Number(config.moon_illuminance as f64)),
                "AmbientBrightness" => Ok(LuaValue::Number(config.ambient_brightness as f64)),
                "FogDensity" => Ok(LuaValue::Number(config.fog_density as f64)),
                _ => {
                    if let Some(lighting_entity) = crate::scripting::userdata::instance::find_service_entity(world, "Lighting") {
                        let instance = crate::scripting::userdata::instance::Instance { entity: lighting_entity };
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
                "ClockTime" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.time_of_day = (val as f32).rem_euclid(24.0);
                        }
                    }
                }
                "TimeOfDay" => {
                    if let LuaValue::String(s) = value {
                        if let Some(parsed) = parse_time_of_day(&s.to_string_lossy()) {
                            if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                                cfg.time_of_day = parsed;
                            }
                        }
                    }
                }
                "Latitude" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.latitude = (val as f32).clamp(-90.0, 90.0);
                        }
                    }
                }
                "SunAngularRadius" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.sun_angular_radius = (val as f32).clamp(0.001, 0.5);
                        }
                    }
                }
                "MoonAngularRadius" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.moon_angular_radius = (val as f32).clamp(0.001, 0.5);
                        }
                    }
                }
                "StarDensity" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.star_density = (val as f32).clamp(0.0, 1.0);
                        }
                    }
                }
                "NightAmbient" => {
                    if let LuaValue::UserData(ud) = value {
                        if let Ok(col) = ud.borrow::<Color3>() {
                            if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                                cfg.night_ambient = col.0;
                            }
                        }
                    }
                }
                "SunBrightness" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.sun_illuminance = val.max(0.0) as f32;
                        }
                    }
                }
                "MoonBrightness" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.moon_illuminance = val.max(0.0) as f32;
                        }
                    }
                }
                "AmbientBrightness" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.ambient_brightness = val.max(0.0) as f32;
                        }
                    }
                }
                "FogDensity" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.fog_density = val.max(0.0) as f32;
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::testing::{eval, run_script, test_vm, test_world};

    #[test]
    fn lighting_properties_round_trip_through_lua() {
        let mut world = test_world();
        world.insert_resource(LightingConfig::default());
        let vm = test_vm(&mut world);

        let clock_time: f64 = eval(&vm, "return game.Lighting.ClockTime");
        assert_eq!(clock_time, 14.5);

        run_script(&vm, "game.Lighting.ClockTime = 8.25");
        assert_eq!(world.resource::<LightingConfig>().time_of_day, 8.25);

        run_script(&vm, "game.Lighting.TimeOfDay = '22:30:00'");
        assert_eq!(world.resource::<LightingConfig>().time_of_day, 22.5);

        let time_of_day: String = eval(&vm, "return game.Lighting.TimeOfDay");
        assert_eq!(time_of_day, "22:30:00");

        run_script(&vm, "game.Lighting.Latitude = 35.5");
        assert_eq!(world.resource::<LightingConfig>().latitude, 35.5);

        run_script(&vm, "game.Lighting.StarDensity = 0.25");
        assert_eq!(world.resource::<LightingConfig>().star_density, 0.25);

        run_script(&vm, "game.Lighting.NightAmbient = Color3.fromRGB(20, 40, 90)");
        let ambient = world.resource::<LightingConfig>().night_ambient.to_srgba();
        assert!((ambient.red - 20.0 / 255.0).abs() < 0.01);
        assert!((ambient.green - 40.0 / 255.0).abs() < 0.01);
        assert!((ambient.blue - 90.0 / 255.0).abs() < 0.01);

        run_script(&vm, "game.Lighting.SunBrightness = 15000");
        assert_eq!(world.resource::<LightingConfig>().sun_illuminance, 15000.0);

        run_script(&vm, "game.Lighting.MoonBrightness = 150");
        assert_eq!(world.resource::<LightingConfig>().moon_illuminance, 150.0);

        run_script(&vm, "game.Lighting.AmbientBrightness = 2.5");
        assert_eq!(world.resource::<LightingConfig>().ambient_brightness, 2.5);

        run_script(&vm, "game.Lighting.FogDensity = 0.75");
        assert_eq!(world.resource::<LightingConfig>().fog_density, 0.75);

        let class: String = eval(&vm, "return game.Lighting.ClassName");
        assert_eq!(class, "Lighting");
        let via_service: String = eval(&vm, "return game:GetService('Lighting').ClassName");
        assert_eq!(via_service, "Lighting");
    }
}
