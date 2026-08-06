use bevy::prelude::*;

#[derive(Resource, Clone, Copy)]
pub struct CloudsConfig {
    pub clouds_raymarch_steps_count: u32,
    pub clouds_shadow_raymarch_steps_count: u32,
    pub planet_radius: f32,
    pub clouds_bottom_height: f32,
    pub clouds_top_height: f32,
    pub clouds_coverage: f32,
    pub clouds_detail_strength: f32,
    pub clouds_base_edge_softness: f32,
    pub clouds_bottom_softness: f32,
    pub clouds_density: f32,
    pub clouds_shadow_raymarch_step_size: f32,
    pub clouds_shadow_raymarch_step_multiply: f32,
    pub forward_scattering_g: f32,
    pub backward_scattering_g: f32,
    pub scattering_lerp: f32,
    pub clouds_ambient_color_top: Vec4,
    pub clouds_ambient_color_bottom: Vec4,
    pub clouds_min_transmittance: f32,
    pub clouds_base_scale: f32,
    pub clouds_detail_scale: f32,
    pub sun_dir: Vec4,
    pub sun_color: Vec4,
    pub reprojection_strength: f32,
    pub ui_visible: bool,
    pub render_resolution: Vec2,
    pub render_scale: f32,
    pub wind_velocity: Vec3,
    pub enabled: bool,
}

impl Default for CloudsConfig {
    fn default() -> Self {
        let sun_dir = Vec3::new(-0.7, 0.5, 0.75).normalize();
        Self {
            clouds_raymarch_steps_count: 12,
            clouds_shadow_raymarch_steps_count: 6,
            planet_radius: 6_371_000.0,
            clouds_bottom_height: 1250.0,
            clouds_top_height: 2400.0,
            clouds_coverage: 0.48,
            clouds_detail_strength: 0.27,
            clouds_base_edge_softness: 0.1,
            clouds_bottom_softness: 0.25,
            clouds_density: 0.03,
            clouds_shadow_raymarch_step_size: 10.0,
            clouds_shadow_raymarch_step_multiply: 1.3,
            forward_scattering_g: 0.8,
            backward_scattering_g: -0.2,
            scattering_lerp: 0.5,
            clouds_ambient_color_top: Vec4::new(149.0, 167.0, 200.0, 0.0) * (1.5 / 225.0),
            clouds_ambient_color_bottom: Vec4::new(39.0, 67.0, 87.0, 0.0) * (1.5 / 225.0),
            clouds_min_transmittance: 0.1,
            clouds_base_scale: 1.5,
            clouds_detail_scale: 42.0,
            sun_dir: Vec4::new(sun_dir.x, sun_dir.y, sun_dir.z, 0.0),
            sun_color: Vec4::new(1.0, 0.9, 0.85, 1.0) * 0.8,
            reprojection_strength: 0.95,
            ui_visible: false,
            render_resolution: Vec2::new(1440.0, 810.0),
            render_scale: 1.0,
            wind_velocity: Vec3::new(-1.1, 0.0, 2.3),
            enabled: true,
        }
    }
}