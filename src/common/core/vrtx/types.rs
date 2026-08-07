use bevy::prelude::*;

pub const FORMAT_VERSION: u32 = 7;

#[derive(Debug, Clone, PartialEq)]
pub struct VrtxBrick {
    pub name: String,
    pub transform: Transform,
    pub shape: crate::common::game::bricks::components::BrickShape,
    pub color: Color,
    pub physics_enabled: bool,
    pub bounciness: f32,
    pub player_can_collide: bool,
    pub friction: f32,
    pub gravity_scale: f32,
    pub mass: f32,
    pub show_studs: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VrtxScript {
    pub name: String,
    pub script_type: u8,
    pub code: String,
    pub parent_name: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VrtxSettings {
    pub ssao: bool,
    pub contact_shadows: bool,
    pub bloom: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VrtxLighting {
    pub time_of_day: f32,
    pub latitude: f32,
    pub sun_angular_radius: f32,
    pub moon_angular_radius: f32,
    pub night_ambient: Color,
    pub star_density: f32,
    pub sun_illuminance: f32,
    pub moon_illuminance: f32,
    pub ambient_brightness: f32,
    pub fog_density: f32,
    pub volumetric_clouds: bool,
    pub cloud_render_scale: f32,
    pub cloud_raymarch_steps: u32,
    pub cloud_shadow_steps: u32,
    pub planet_radius: f32,
    pub cloud_bottom_height: f32,
    pub cloud_top_height: f32,
    pub cloud_coverage: f32,
    pub cloud_density: f32,
    pub cloud_detail_strength: f32,
    pub cloud_base_edge_softness: f32,
    pub cloud_bottom_softness: f32,
    pub cloud_base_scale: f32,
    pub cloud_detail_scale: f32,
    pub cloud_shadow_step_size: f32,
    pub cloud_shadow_step_multiply: f32,
    pub cloud_forward_scattering_g: f32,
    pub cloud_backward_scattering_g: f32,
    pub cloud_scattering_lerp: f32,
    pub cloud_min_transmittance: f32,
    pub cloud_reprojection_strength: f32,
    pub cloud_wind_velocity: Vec3,
}

impl Default for VrtxLighting {
    fn default() -> Self {
        Self {
            time_of_day: 14.5,
            latitude: 45.0,
            sun_angular_radius: 0.035,
            moon_angular_radius: 0.040,
            night_ambient: Color::srgb(0.12, 0.22, 0.48),
            star_density: 0.85,
            sun_illuminance: 12_000.0,
            moon_illuminance: 100.0,
            ambient_brightness: 1.0,
            fog_density: 1.0,
            volumetric_clouds: true,
            cloud_render_scale: 1.0,
            cloud_raymarch_steps: 12,
            cloud_shadow_steps: 6,
            planet_radius: 6_371_000.0,
            cloud_bottom_height: 1250.0,
            cloud_top_height: 2400.0,
            cloud_coverage: 0.48,
            cloud_density: 0.03,
            cloud_detail_strength: 0.27,
            cloud_base_edge_softness: 0.1,
            cloud_bottom_softness: 0.25,
            cloud_base_scale: 1.5,
            cloud_detail_scale: 42.0,
            cloud_shadow_step_size: 10.0,
            cloud_shadow_step_multiply: 1.3,
            cloud_forward_scattering_g: 0.8,
            cloud_backward_scattering_g: -0.2,
            cloud_scattering_lerp: 0.5,
            cloud_min_transmittance: 0.1,
            cloud_reprojection_strength: 0.95,
            cloud_wind_velocity: Vec3::new(-1.1, 0.0, 2.3),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VrtxFileState {
    pub version: u32,
    pub gravity: Vec3,
    pub settings: VrtxSettings,
    pub lighting: VrtxLighting,
    pub camera_transform: Transform,
    pub bricks: Vec<VrtxBrick>,
    pub scripts: Vec<VrtxScript>,
}
