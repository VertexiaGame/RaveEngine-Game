use bevy::prelude::*;
use mlua::prelude::*;
use crate::client::sky::LightingConfig;
use crate::scripting::userdata::color3::Color3;
use crate::scripting::userdata::vector3::Vector3;

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
                "VolumetricClouds" => Ok(LuaValue::Boolean(config.volumetric_clouds)),
                "CloudRenderScale" => Ok(LuaValue::Number(config.cloud_render_scale as f64)),
                "CloudRaymarchSteps" => Ok(LuaValue::Number(config.cloud_raymarch_steps as f64)),
                "CloudShadowSteps" => Ok(LuaValue::Number(config.cloud_shadow_steps as f64)),
                "PlanetRadius" => Ok(LuaValue::Number(config.planet_radius as f64)),
                "CloudBottomHeight" => Ok(LuaValue::Number(config.cloud_bottom_height as f64)),
                "CloudTopHeight" => Ok(LuaValue::Number(config.cloud_top_height as f64)),
                "CloudCoverage" => Ok(LuaValue::Number(config.cloud_coverage as f64)),
                "CloudDensity" => Ok(LuaValue::Number(config.cloud_density as f64)),
                "CloudDetailStrength" => Ok(LuaValue::Number(config.cloud_detail_strength as f64)),
                "CloudBaseEdgeSoftness" => Ok(LuaValue::Number(config.cloud_base_edge_softness as f64)),
                "CloudBottomSoftness" => Ok(LuaValue::Number(config.cloud_bottom_softness as f64)),
                "CloudBaseScale" => Ok(LuaValue::Number(config.cloud_base_scale as f64)),
                "CloudDetailScale" => Ok(LuaValue::Number(config.cloud_detail_scale as f64)),
                "CloudShadowStepSize" => Ok(LuaValue::Number(config.cloud_shadow_step_size as f64)),
                "CloudShadowStepMultiply" => Ok(LuaValue::Number(config.cloud_shadow_step_multiply as f64)),
                "CloudForwardScatteringG" => Ok(LuaValue::Number(config.cloud_forward_scattering_g as f64)),
                "CloudBackwardScatteringG" => Ok(LuaValue::Number(config.cloud_backward_scattering_g as f64)),
                "CloudScatteringLerp" => Ok(LuaValue::Number(config.cloud_scattering_lerp as f64)),
                "CloudMinTransmittance" => Ok(LuaValue::Number(config.cloud_min_transmittance as f64)),
                "CloudReprojectionStrength" => Ok(LuaValue::Number(config.cloud_reprojection_strength as f64)),
                "CloudWindVelocity" => lua
                    .create_userdata(Vector3(config.cloud_wind_velocity))
                    .map(LuaValue::UserData),
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
                "VolumetricClouds" => {
                    if let LuaValue::Boolean(b) = value {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.volumetric_clouds = b;
                        }
                    }
                }
                "CloudRenderScale" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_render_scale = (val as f32).clamp(0.25, 1.0);
                        }
                    }
                }
                "CloudRaymarchSteps" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_raymarch_steps = (val as u32).clamp(1, 100);
                        }
                    }
                }
                "CloudShadowSteps" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_shadow_steps = (val as u32).clamp(1, 50);
                        }
                    }
                }
                "PlanetRadius" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.planet_radius = (val as f32).clamp(5e4, 1e7);
                        }
                    }
                }
                "CloudBottomHeight" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_bottom_height = (val as f32).clamp(1.0, 5e3);
                        }
                    }
                }
                "CloudTopHeight" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_top_height = (val as f32).clamp(1.0, 5e3);
                        }
                    }
                }
                "CloudCoverage" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_coverage = (val as f32).clamp(0.0, 1.0);
                        }
                    }
                }
                "CloudDensity" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_density = (val as f32).clamp(0.001, 1.0);
                        }
                    }
                }
                "CloudDetailStrength" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_detail_strength = (val as f32).clamp(0.0, 1.0);
                        }
                    }
                }
                "CloudBaseEdgeSoftness" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_base_edge_softness = (val as f32).clamp(0.0, 1.0);
                        }
                    }
                }
                "CloudBottomSoftness" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_bottom_softness = (val as f32).clamp(0.01, 10.0);
                        }
                    }
                }
                "CloudBaseScale" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_base_scale = (val as f32).clamp(0.1, 100.0);
                        }
                    }
                }
                "CloudDetailScale" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_detail_scale = (val as f32).clamp(1.0, 100.0);
                        }
                    }
                }
                "CloudShadowStepSize" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_shadow_step_size = (val as f32).clamp(1.0, 100.0);
                        }
                    }
                }
                "CloudShadowStepMultiply" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_shadow_step_multiply = (val as f32).clamp(0.1, 10.0);
                        }
                    }
                }
                "CloudForwardScatteringG" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_forward_scattering_g = (val as f32).clamp(-10.0, 10.0);
                        }
                    }
                }
                "CloudBackwardScatteringG" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_backward_scattering_g = (val as f32).clamp(-10.0, 10.0);
                        }
                    }
                }
                "CloudScatteringLerp" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_scattering_lerp = (val as f32).clamp(0.01, 100.0);
                        }
                    }
                }
                "CloudMinTransmittance" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_min_transmittance = (val as f32).clamp(0.01, 100.0);
                        }
                    }
                }
                "CloudReprojectionStrength" => {
                    if let Some(val) = as_f64(&value) {
                        if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                            cfg.cloud_reprojection_strength = (val as f32).clamp(0.0, 1.0);
                        }
                    }
                }
                "CloudWindVelocity" => {
                    if let LuaValue::UserData(ud) = value {
                        if let Ok(v) = ud.borrow::<Vector3>() {
                            if let Some(mut cfg) = world.get_resource_mut::<LightingConfig>() {
                                cfg.cloud_wind_velocity = v.0;
                            }
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

        run_script(&vm, "game.Lighting.VolumetricClouds = false");
        assert!(!world.resource::<LightingConfig>().volumetric_clouds);
        let volumetric: bool = eval(&vm, "return game.Lighting.VolumetricClouds");
        assert!(!volumetric);

        run_script(&vm, "game.Lighting.CloudRenderScale = 0.5");
        assert_eq!(world.resource::<LightingConfig>().cloud_render_scale, 0.5);

        run_script(&vm, "game.Lighting.CloudRaymarchSteps = 32");
        assert_eq!(world.resource::<LightingConfig>().cloud_raymarch_steps, 32);

        run_script(&vm, "game.Lighting.CloudShadowSteps = 10");
        assert_eq!(world.resource::<LightingConfig>().cloud_shadow_steps, 10);

        run_script(&vm, "game.Lighting.CloudCoverage = 0.75");
        assert_eq!(world.resource::<LightingConfig>().cloud_coverage, 0.75);

        run_script(&vm, "game.Lighting.CloudDensity = 0.06");
        assert_eq!(world.resource::<LightingConfig>().cloud_density, 0.06);

        run_script(&vm, "game.Lighting.CloudDetailStrength = 0.5");
        assert_eq!(world.resource::<LightingConfig>().cloud_detail_strength, 0.5);

        run_script(&vm, "game.Lighting.CloudBaseScale = 2.5");
        assert_eq!(world.resource::<LightingConfig>().cloud_base_scale, 2.5);

        run_script(&vm, "game.Lighting.CloudDetailScale = 60.0");
        assert_eq!(world.resource::<LightingConfig>().cloud_detail_scale, 60.0);

        run_script(&vm, "game.Lighting.CloudMinTransmittance = 0.25");
        assert_eq!(world.resource::<LightingConfig>().cloud_min_transmittance, 0.25);

        run_script(&vm, "game.Lighting.CloudReprojectionStrength = 0.8");
        assert_eq!(world.resource::<LightingConfig>().cloud_reprojection_strength, 0.8);

        run_script(&vm, "game.Lighting.CloudWindVelocity = Vector3.new(5, 0, -3)");
        assert_eq!(
            world.resource::<LightingConfig>().cloud_wind_velocity,
            bevy::math::Vec3::new(5.0, 0.0, -3.0)
        );
        let wind: mlua::AnyUserData = eval(&vm, "return game.Lighting.CloudWindVelocity");
        let wind = wind.borrow::<Vector3>().unwrap();
        assert_eq!(wind.0, bevy::math::Vec3::new(5.0, 0.0, -3.0));

        run_script(&vm, "game.Lighting.CloudCoverage = 2.0");
        assert_eq!(world.resource::<LightingConfig>().cloud_coverage, 1.0);

        let class: String = eval(&vm, "return game.Lighting.ClassName");
        assert_eq!(class, "Lighting");
        let via_service: String = eval(&vm, "return game:GetService('Lighting').ClassName");
        assert_eq!(via_service, "Lighting");
    }
}
