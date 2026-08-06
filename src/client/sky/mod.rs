pub mod clouds;

pub use clouds::{CloudsConfig, CloudsPlugin};

use bevy::camera::visibility::RenderLayers;
use bevy::color::ColorToComponents;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, StandardMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use std::f32::consts::PI;

const CAMERA_FAR_PLANE: f32 = 3_000.0;
const DAY_OF_YEAR: u32 = 172;
const SUN_ILLUMINANCE: f32 = 12_000.0;
const MOON_ILLUMINANCE: f32 = 100.0;
const FOG_EXTINCTION: f32 = 3.0e-5;

#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct LightingConfig {
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

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            time_of_day: 14.5,
            latitude: 45.0,
            sun_angular_radius: 0.035,
            moon_angular_radius: 0.040,
            night_ambient: Color::srgb(0.12, 0.22, 0.48),
            star_density: 0.85,
            sun_illuminance: SUN_ILLUMINANCE,
            moon_illuminance: MOON_ILLUMINANCE,
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

impl From<&LightingConfig> for crate::common::core::vrtx::VrtxLighting {
    fn from(config: &LightingConfig) -> Self {
        Self {
            time_of_day: config.time_of_day,
            latitude: config.latitude,
            sun_angular_radius: config.sun_angular_radius,
            moon_angular_radius: config.moon_angular_radius,
            night_ambient: config.night_ambient,
            star_density: config.star_density,
            sun_illuminance: config.sun_illuminance,
            moon_illuminance: config.moon_illuminance,
            ambient_brightness: config.ambient_brightness,
            fog_density: config.fog_density,
            volumetric_clouds: config.volumetric_clouds,
            cloud_render_scale: config.cloud_render_scale,
            cloud_raymarch_steps: config.cloud_raymarch_steps,
            cloud_shadow_steps: config.cloud_shadow_steps,
            planet_radius: config.planet_radius,
            cloud_bottom_height: config.cloud_bottom_height,
            cloud_top_height: config.cloud_top_height,
            cloud_coverage: config.cloud_coverage,
            cloud_density: config.cloud_density,
            cloud_detail_strength: config.cloud_detail_strength,
            cloud_base_edge_softness: config.cloud_base_edge_softness,
            cloud_bottom_softness: config.cloud_bottom_softness,
            cloud_base_scale: config.cloud_base_scale,
            cloud_detail_scale: config.cloud_detail_scale,
            cloud_shadow_step_size: config.cloud_shadow_step_size,
            cloud_shadow_step_multiply: config.cloud_shadow_step_multiply,
            cloud_forward_scattering_g: config.cloud_forward_scattering_g,
            cloud_backward_scattering_g: config.cloud_backward_scattering_g,
            cloud_scattering_lerp: config.cloud_scattering_lerp,
            cloud_min_transmittance: config.cloud_min_transmittance,
            cloud_reprojection_strength: config.cloud_reprojection_strength,
            cloud_wind_velocity: config.cloud_wind_velocity,
        }
    }
}

impl From<crate::common::core::vrtx::VrtxLighting> for LightingConfig {
    fn from(lighting: crate::common::core::vrtx::VrtxLighting) -> Self {
        Self {
            time_of_day: lighting.time_of_day,
            latitude: lighting.latitude,
            sun_angular_radius: lighting.sun_angular_radius,
            moon_angular_radius: lighting.moon_angular_radius,
            night_ambient: lighting.night_ambient,
            star_density: lighting.star_density,
            sun_illuminance: lighting.sun_illuminance,
            moon_illuminance: lighting.moon_illuminance,
            ambient_brightness: lighting.ambient_brightness,
            fog_density: lighting.fog_density,
            volumetric_clouds: lighting.volumetric_clouds,
            cloud_render_scale: lighting.cloud_render_scale,
            cloud_raymarch_steps: lighting.cloud_raymarch_steps,
            cloud_shadow_steps: lighting.cloud_shadow_steps,
            planet_radius: lighting.planet_radius,
            cloud_bottom_height: lighting.cloud_bottom_height,
            cloud_top_height: lighting.cloud_top_height,
            cloud_coverage: lighting.cloud_coverage,
            cloud_density: lighting.cloud_density,
            cloud_detail_strength: lighting.cloud_detail_strength,
            cloud_base_edge_softness: lighting.cloud_base_edge_softness,
            cloud_bottom_softness: lighting.cloud_bottom_softness,
            cloud_base_scale: lighting.cloud_base_scale,
            cloud_detail_scale: lighting.cloud_detail_scale,
            cloud_shadow_step_size: lighting.cloud_shadow_step_size,
            cloud_shadow_step_multiply: lighting.cloud_shadow_step_multiply,
            cloud_forward_scattering_g: lighting.cloud_forward_scattering_g,
            cloud_backward_scattering_g: lighting.cloud_backward_scattering_g,
            cloud_scattering_lerp: lighting.cloud_scattering_lerp,
            cloud_min_transmittance: lighting.cloud_min_transmittance,
            cloud_reprojection_strength: lighting.cloud_reprojection_strength,
            cloud_wind_velocity: lighting.cloud_wind_velocity,
        }
    }
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct SkyUniformsGpu {
    pub sun_direction: Vec3,
    pub sun_angular_radius: f32,
    pub moon_direction: Vec3,
    pub moon_angular_radius: f32,
    pub sun_color: Vec3,
    pub time_of_day: f32,
    pub moon_color: Vec3,
    pub star_density: f32,
    pub ground_albedo: Vec3,
    pub night_ambient_color: Vec3,
    pub rayleigh_coeff: Vec3,
    pub mie_coeff: f32,
    pub mie_asymmetry: f32,
    pub horizon_blur: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyMaterialExtension {
    #[uniform(100)]
    uniforms: SkyUniformsGpu,
}

impl MaterialExtension for SkyMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/procedural_sky.wgsl".into()
    }
}

pub type CustomSkyMaterial = ExtendedMaterial<StandardMaterial, SkyMaterialExtension>;

#[derive(Component)]
pub struct SunLightEntity;

#[derive(Component)]
pub struct MoonLightEntity;

#[derive(Component)]
pub struct SkyCameraConfigured;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(clouds::CloudsPlugin)
            .add_plugins(MaterialPlugin::<CustomSkyMaterial>::default())
            .init_resource::<LightingConfig>()
            .register_type::<LightingConfig>()
            .add_systems(Startup, setup_sky_environment)
            .add_systems(Update, (configure_sky_cameras, sync_lighting_system));
    }
}

fn setup_sky_environment(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: SUN_ILLUMINANCE,
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.6,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 1000.0,
            first_cascade_far_bound: 15.0,
            overlap_proportion: 0.2,
        }
        .build(),
        Transform::IDENTITY,
        SunLightEntity,
    ));

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.55, 0.72, 1.0),
            illuminance: MOON_ILLUMINANCE,
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.6,
            ..default()
        },
        Transform::IDENTITY,
        MoonLightEntity,
    ));
}

fn configure_sky_cameras(
    mut commands: Commands,
    cameras: Query<(Entity, &Camera), (With<Camera3d>, Without<SkyCameraConfigured>, Without<RenderLayers>)>,
    mut projections: Query<&mut Projection>,
) {
    for (entity, camera) in &cameras {
        if !camera.is_active {
            continue;
        }
        if let Ok(mut projection) = projections.get_mut(entity) {
            if let Projection::Perspective(settings) = &mut *projection {
                if settings.far < CAMERA_FAR_PLANE {
                    settings.far = CAMERA_FAR_PLANE;
                }
            }
        }
        commands.entity(entity).insert((
            SkyCameraConfigured,
            Tonemapping::AgX,
            DistanceFog {
                color: Color::srgb(0.55, 0.72, 0.90),
                directional_light_color: Color::srgb(1.0, 0.96, 0.88),
                directional_light_exponent: 15.0,
                falloff: FogFalloff::Atmospheric {
                    extinction: Vec3::splat(FOG_EXTINCTION),
                    inscattering: Vec3::new(0.55, 0.72, 0.90) * FOG_EXTINCTION,
                },
            },
        ));
    }
}

fn sync_lighting_system(
    config: Res<LightingConfig>,
    mut sun_query: Query<
        (&mut DirectionalLight, &mut Transform),
        (With<SunLightEntity>, Without<MoonLightEntity>),
    >,
    mut moon_query: Query<
        (&mut DirectionalLight, &mut Transform),
        (With<MoonLightEntity>, Without<SunLightEntity>),
    >,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    clouds_config: Option<ResMut<CloudsConfig>>,
    mut fog_query: Query<&mut DistanceFog>,
) {
    let sun_direction = solar_direction(&config);
    let moon_direction = -sun_direction;
    let solar_elevation = sun_direction.y;

    let day_weight = smoothstep(0.02, 0.30, solar_elevation);
    let night_weight = 1.0 - smoothstep(-0.15, 0.05, solar_elevation);
    let sunset_weight = (1.0 - day_weight - night_weight).max(0.0);
    let sun_has_dominance = day_weight + sunset_weight >= night_weight;

    if let Ok((mut sun_light, mut sun_transform)) = sun_query.single_mut() {
        let dir = -sun_direction;
        let up = if dir.y.abs() > 0.95 { Vec3::Z } else { Vec3::Y };
        if let Ok(dir_3d) = Dir3::new(dir) {
            *sun_transform = Transform::from_translation(Vec3::ZERO).looking_to(dir_3d, up);
        }
        sun_light.illuminance = (day_weight + sunset_weight * 0.35) * config.sun_illuminance;
        sun_light.color = sun_light_color(solar_elevation);
        let sun_shadows_enabled = day_weight + sunset_weight > 0.02 && sun_has_dominance;
        if sun_light.shadow_maps_enabled != sun_shadows_enabled {
            sun_light.shadow_maps_enabled = sun_shadows_enabled;
        }
    }

    if let Ok((mut moon_light, mut moon_transform)) = moon_query.single_mut() {
        let dir = -moon_direction;
        let up = if dir.y.abs() > 0.95 { Vec3::Z } else { Vec3::Y };
        if let Ok(dir_3d) = Dir3::new(dir) {
            *moon_transform = Transform::from_translation(Vec3::ZERO).looking_to(dir_3d, up);
        }
        moon_light.illuminance = night_weight * config.moon_illuminance;
        moon_light.color = Color::srgb(0.55, 0.72, 1.0);
        let moon_shadows_enabled = night_weight > 0.02 && !sun_has_dominance;
        if moon_light.shadow_maps_enabled != moon_shadows_enabled {
            moon_light.shadow_maps_enabled = moon_shadows_enabled;
        }
    }

    let day_ambient_vec = Vec3::new(0.55, 0.72, 0.92);
    let sunset_ambient_vec = Vec3::new(0.82, 0.48, 0.38);
    let night_ambient_vec = config.night_ambient.to_linear().to_vec3().max(Vec3::new(0.12, 0.22, 0.48));
    let current_ambient = day_weight * day_ambient_vec + sunset_weight * sunset_ambient_vec + night_weight * night_ambient_vec;

    ambient_light.color = Color::from(LinearRgba::new(
        current_ambient.x,
        current_ambient.y,
        current_ambient.z,
        1.0,
    ));
    ambient_light.brightness = (day_weight * 1400.0 + sunset_weight * 800.0 + night_weight * 350.0) * config.ambient_brightness;

    let day_fog_vec = Vec3::new(0.55, 0.72, 0.90);
    let sunset_fog_vec = Vec3::new(0.82, 0.50, 0.40);
    let night_fog_vec = Vec3::new(0.03, 0.06, 0.15);
    let fog_color_vec = day_weight * day_fog_vec + sunset_weight * sunset_fog_vec + night_weight * night_fog_vec;
    let fog_color = Color::from(LinearRgba::new(fog_color_vec.x, fog_color_vec.y, fog_color_vec.z, 1.0));
    let sun_inscatter_color = sun_light_color(solar_elevation);

    for mut fog in &mut fog_query {
        fog.color = fog_color;
        fog.directional_light_color = sun_inscatter_color;
        fog.falloff = FogFalloff::Atmospheric {
            extinction: Vec3::splat(FOG_EXTINCTION) * config.fog_density,
            inscattering: fog_color_vec * FOG_EXTINCTION * config.fog_density,
        };
    }

    if let Some(mut clouds_config) = clouds_config {
        clouds_config.sun_dir = Vec4::new(sun_direction.x, sun_direction.y, sun_direction.z, 0.0);

        let day_sun_col = Vec4::new(1.0, 0.96, 0.88, 1.0) * 0.85;
        let sunset_sun_col = Vec4::new(1.0, 0.52, 0.18, 1.0) * 0.90;
        let night_sun_col = Vec4::new(0.35, 0.52, 0.85, 1.0) * 0.25;
        clouds_config.sun_color = day_weight * day_sun_col + sunset_weight * sunset_sun_col + night_weight * night_sun_col;

        let day_ambient_top = Vec4::new(149.0, 167.0, 200.0, 0.0) * (1.5 / 225.0);
        let sunset_ambient_top = Vec4::new(160.0, 110.0, 140.0, 0.0) * (1.2 / 225.0);
        let night_ambient_top = Vec4::new(15.0, 28.0, 60.0, 0.0) * (0.8 / 225.0);
        clouds_config.clouds_ambient_color_top = day_weight * day_ambient_top + sunset_weight * sunset_ambient_top + night_weight * night_ambient_top;

        let day_ambient_bottom = Vec4::new(39.0, 67.0, 87.0, 0.0) * (1.5 / 225.0);
        let sunset_ambient_bottom = Vec4::new(60.0, 35.0, 45.0, 0.0) * (1.2 / 225.0);
        let night_ambient_bottom = Vec4::new(5.0, 10.0, 22.0, 0.0) * (0.8 / 225.0);
        clouds_config.clouds_ambient_color_bottom = day_weight * day_ambient_bottom + sunset_weight * sunset_ambient_bottom + night_weight * night_ambient_bottom;

        clouds_config.enabled = config.volumetric_clouds;
        clouds_config.render_scale = config.cloud_render_scale;
        clouds_config.clouds_raymarch_steps_count = config.cloud_raymarch_steps;
        clouds_config.clouds_shadow_raymarch_steps_count = config.cloud_shadow_steps;
        clouds_config.planet_radius = config.planet_radius;
        clouds_config.clouds_bottom_height = config.cloud_bottom_height;
        clouds_config.clouds_top_height = config.cloud_top_height;
        clouds_config.clouds_coverage = config.cloud_coverage;
        clouds_config.clouds_density = config.cloud_density;
        clouds_config.clouds_detail_strength = config.cloud_detail_strength;
        clouds_config.clouds_base_edge_softness = config.cloud_base_edge_softness;
        clouds_config.clouds_bottom_softness = config.cloud_bottom_softness;
        clouds_config.clouds_base_scale = config.cloud_base_scale;
        clouds_config.clouds_detail_scale = config.cloud_detail_scale;
        clouds_config.clouds_shadow_raymarch_step_size = config.cloud_shadow_step_size;
        clouds_config.clouds_shadow_raymarch_step_multiply = config.cloud_shadow_step_multiply;
        clouds_config.forward_scattering_g = config.cloud_forward_scattering_g;
        clouds_config.backward_scattering_g = config.cloud_backward_scattering_g;
        clouds_config.scattering_lerp = config.cloud_scattering_lerp;
        clouds_config.clouds_min_transmittance = config.cloud_min_transmittance;
        clouds_config.reprojection_strength = config.cloud_reprojection_strength;
        clouds_config.wind_velocity = config.cloud_wind_velocity;
    }
}

fn initial_sky_uniforms(config: &LightingConfig) -> SkyUniformsGpu {
    let sun_direction = solar_direction(config);
    SkyUniformsGpu {
        sun_direction,
        sun_angular_radius: config.sun_angular_radius,
        moon_direction: -sun_direction,
        moon_angular_radius: config.moon_angular_radius,
        sun_color: Vec3::new(1.0, 0.95, 0.85),
        time_of_day: config.time_of_day,
        moon_color: Vec3::new(0.65, 0.82, 1.0),
        star_density: config.star_density,
        ground_albedo: Vec3::new(0.18, 0.38, 0.18),
        night_ambient_color: config.night_ambient.to_linear().to_vec3(),
        rayleigh_coeff: Vec3::new(5.8e-6, 1.35e-5, 3.31e-5),
        mie_coeff: 4.0e-6,
        mie_asymmetry: 0.76,
        horizon_blur: 0.08,
    }
}

fn solar_declination() -> f32 {
    (23.44 * (2.0 * PI * (284.0 + DAY_OF_YEAR as f32) / 365.0).sin()).to_radians()
}

fn solar_altitude_rad(config: &LightingConfig) -> f32 {
    let hour_angle = (config.time_of_day - 12.0) * (PI / 12.0);
    let latitude_rad = config.latitude.to_radians();
    let declination = solar_declination();
    (latitude_rad.sin() * declination.sin()
        + latitude_rad.cos() * declination.cos() * hour_angle.cos())
    .asin()
}

fn solar_direction(config: &LightingConfig) -> Vec3 {
    let hour_angle = (config.time_of_day - 12.0) * (PI / 12.0);
    let latitude_rad = config.latitude.to_radians();
    let declination = solar_declination();
    let altitude = solar_altitude_rad(config);

    let cos_azimuth = (declination.sin() - latitude_rad.sin() * altitude.sin())
        / (latitude_rad.cos() * altitude.cos().max(1.0e-4));
    let azimuth = if hour_angle.sin() > 0.0 {
        2.0 * PI - cos_azimuth.clamp(-1.0, 1.0).acos()
    } else {
        cos_azimuth.clamp(-1.0, 1.0).acos()
    };

    Vec3::new(
        altitude.cos() * azimuth.sin(),
        altitude.sin(),
        altitude.cos() * azimuth.cos(),
    )
    .normalize()
}

fn sun_light_color(altitude: f32) -> Color {
    let day_factor = smoothstep(0.02, 0.35, altitude);
    mix_color(Color::srgb(1.0, 0.48, 0.12), Color::srgb(1.0, 0.96, 0.88), day_factor)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix_val(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let va = a.to_linear();
    let vb = b.to_linear();
    Color::from(LinearRgba::new(
        va.red + (vb.red - va.red) * t,
        va.green + (vb.green - va.green) * t,
        va.blue + (vb.blue - va.blue) * t,
        1.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solar_direction_is_normalized() {
        let config = LightingConfig::default();
        assert!((solar_direction(&config).length() - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn solar_altitude_peaks_at_noon() {
        let mut morning = LightingConfig {
            time_of_day: 9.0,
            ..default()
        };
        let noon = LightingConfig {
            time_of_day: 12.0,
            ..default()
        };
        let evening = LightingConfig {
            time_of_day: 15.0,
            ..default()
        };

        let noon_altitude = solar_altitude_rad(&noon);
        assert!(solar_altitude_rad(&morning) < noon_altitude);
        assert!(solar_altitude_rad(&evening) < noon_altitude);
        assert!(noon_altitude > 0.0);

        morning.time_of_day = 22.0;
        assert!(solar_altitude_rad(&morning) < 0.0);
    }

    #[test]
    fn sun_visibility_vanishes_at_night() {
        let night = LightingConfig {
            time_of_day: 22.0,
            ..default()
        };
        let elevation = solar_direction(&night).y;
        assert!(smoothstep(-0.15, 0.15, elevation) == 0.0);
    }

    #[test]
    fn smoothstep_interpolates_between_edges() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
    }
}