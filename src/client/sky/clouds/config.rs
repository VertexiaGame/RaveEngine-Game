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

#[derive(Clone, Copy)]
struct CloudsQualityLevel {
    render_scale: f32,
    raymarch_steps: u32,
    shadow_steps: u32,
}

const CLOUDS_QUALITY_LEVELS: [CloudsQualityLevel; 5] = [
    CloudsQualityLevel { render_scale: 1.0, raymarch_steps: 12, shadow_steps: 6 },
    CloudsQualityLevel { render_scale: 0.75, raymarch_steps: 12, shadow_steps: 6 },
    CloudsQualityLevel { render_scale: 0.5, raymarch_steps: 10, shadow_steps: 5 },
    CloudsQualityLevel { render_scale: 0.35, raymarch_steps: 8, shadow_steps: 4 },
    CloudsQualityLevel { render_scale: 0.25, raymarch_steps: 6, shadow_steps: 3 },
];

const CLOUDS_QUALITY_SAMPLE_INTERVAL_SECS: f32 = 0.5;
const CLOUDS_QUALITY_MIN_SAMPLES: u32 = 3;
const CLOUDS_QUALITY_UPGRADE_FRAME_MS: f32 = 30.0;
const CLOUDS_QUALITY_DOWNGRADE_FRAME_MS: f32 = 55.0;
const CLOUDS_QUALITY_EMERGENCY_FRAME_MS: f32 = 120.0;

#[derive(Resource)]
pub struct CloudsQuality {
    level: usize,
    sample_sum_secs: f32,
    sample_count: u32,
    elapsed_secs: f32,
}

impl Default for CloudsQuality {
    fn default() -> Self {
        Self {
            level: 0,
            sample_sum_secs: 0.0,
            sample_count: 0,
            elapsed_secs: 0.0,
        }
    }
}

pub fn quality_level_adjustment(avg_frame_ms: f32) -> isize {
    if avg_frame_ms > CLOUDS_QUALITY_EMERGENCY_FRAME_MS {
        2
    } else if avg_frame_ms > CLOUDS_QUALITY_DOWNGRADE_FRAME_MS {
        1
    } else if avg_frame_ms < CLOUDS_QUALITY_UPGRADE_FRAME_MS {
        -1
    } else {
        0
    }
}

pub fn adapt_clouds_quality(
    time: Res<Time>,
    mut config: ResMut<CloudsConfig>,
    mut quality: ResMut<CloudsQuality>,
) {
    if !config.enabled {
        return;
    }

    let level_settings = CLOUDS_QUALITY_LEVELS[quality.level];
    config.render_scale *= level_settings.render_scale;
    config.clouds_raymarch_steps_count =
        config.clouds_raymarch_steps_count.min(level_settings.raymarch_steps).max(1);
    config.clouds_shadow_raymarch_steps_count =
        config.clouds_shadow_raymarch_steps_count.min(level_settings.shadow_steps);

    let delta = time.delta_secs();
    quality.sample_sum_secs += delta;
    quality.sample_count += 1;
    quality.elapsed_secs += delta;

    if quality.elapsed_secs < CLOUDS_QUALITY_SAMPLE_INTERVAL_SECS
        || quality.sample_count < CLOUDS_QUALITY_MIN_SAMPLES
    {
        return;
    }

    let avg_frame_ms = quality.sample_sum_secs / quality.sample_count as f32 * 1000.0;
    quality.sample_sum_secs = 0.0;
    quality.sample_count = 0;
    quality.elapsed_secs = 0.0;

    let adjustment = quality_level_adjustment(avg_frame_ms);
    if adjustment == 0 {
        return;
    }
    let new_level = quality
        .level
        .saturating_add_signed(adjustment)
        .min(CLOUDS_QUALITY_LEVELS.len() - 1);
    if new_level == quality.level {
        return;
    }
    quality.level = new_level;
    info!(
        "Adaptive cloud quality: level {} (avg frame time {:.1} ms)",
        new_level, avg_frame_ms
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_level_adjustment_downgrades_when_slow() {
        assert_eq!(quality_level_adjustment(200.0), 2);
        assert_eq!(quality_level_adjustment(80.0), 1);
        assert_eq!(quality_level_adjustment(55.1), 1);
    }

    #[test]
    fn quality_level_adjustment_upgrades_when_fast() {
        assert_eq!(quality_level_adjustment(29.9), -1);
        assert_eq!(quality_level_adjustment(16.7), -1);
    }

    #[test]
    fn quality_level_adjustment_holds_in_hysteresis_band() {
        assert_eq!(quality_level_adjustment(30.0), 0);
        assert_eq!(quality_level_adjustment(55.0), 0);
        assert_eq!(quality_level_adjustment(40.0), 0);
    }
}
