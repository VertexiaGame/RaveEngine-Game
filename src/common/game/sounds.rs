use bevy::prelude::*;
use std::sync::OnceLock;

pub const STEP_SOUND_PATH: &str = "content/game/sounds/step.wav";
pub const JUMP_SOUND_PATH_LOWER: &str = "content/game/sounds/jmp.wav";
pub const JUMP_SOUND_PATH_UPPER: &str = "content/game/sounds/jmp.WAV";

pub const WALK_SPEED_THRESHOLD: f32 = 1.0;
pub const JUMP_VELOCITY_THRESHOLD: f32 = 2.5;
pub const STEP_LOOP_HEARTBEAT_SECS: f32 = 1.0;
pub const SERVER_STEP_MIN_INTERVAL_SECS: f64 = 0.12;
pub const SERVER_JUMP_MIN_INTERVAL_SECS: f64 = 0.3;
pub const SOUND_CULL_DISTANCE: f32 = 120.0;
pub const SPATIAL_SCALE: f32 = 0.15;
pub const STEP_VOLUME: f32 = 0.7;
pub const JUMP_VOLUME: f32 = 0.8;

static JUMP_PATH_CACHE: OnceLock<&'static str> = OnceLock::new();

pub fn jump_sound_path() -> &'static str {
    *JUMP_PATH_CACHE.get_or_init(|| {
        let lower = crate::common::assets_path::resolve_asset_path(JUMP_SOUND_PATH_LOWER);
        if lower.exists() {
            JUMP_SOUND_PATH_LOWER
        } else {
            JUMP_SOUND_PATH_UPPER
        }
    })
}

pub fn is_walking(grounded: bool, horizontal_speed: f32) -> bool {
    grounded && horizontal_speed.is_finite() && horizontal_speed > WALK_SPEED_THRESHOLD
}

pub fn is_jump_start(was_grounded: bool, grounded: bool, velocity_y: f32) -> bool {
    was_grounded && !grounded && velocity_y.is_finite() && velocity_y > JUMP_VELOCITY_THRESHOLD
}

pub fn server_should_accept_sound(
    kind: crate::common::net::messages::PlayerSoundKind,
    now_secs: f64,
    last_step_secs: Option<f64>,
    last_jump_secs: Option<f64>,
) -> bool {
    match kind {
        crate::common::net::messages::PlayerSoundKind::StepStart => last_step_secs
            .map_or(true, |last| now_secs - last >= SERVER_STEP_MIN_INTERVAL_SECS),
        crate::common::net::messages::PlayerSoundKind::StepStop => true,
        crate::common::net::messages::PlayerSoundKind::Jump => last_jump_secs
            .map_or(true, |last| now_secs - last >= SERVER_JUMP_MIN_INTERVAL_SECS),
    }
}

pub fn sound_volume(kind: crate::common::net::messages::PlayerSoundKind) -> f32 {
    match kind {
        crate::common::net::messages::PlayerSoundKind::StepStart
        | crate::common::net::messages::PlayerSoundKind::StepStop => STEP_VOLUME,
        crate::common::net::messages::PlayerSoundKind::Jump => JUMP_VOLUME,
    }
}

pub fn within_hearing_range(listener: Vec3, source: Vec3) -> bool {
    listener.distance_squared(source) <= SOUND_CULL_DISTANCE * SOUND_CULL_DISTANCE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::net::messages::PlayerSoundKind;

    #[test]
    fn walking_requires_grounded_and_speed() {
        assert!(is_walking(true, 4.0));
        assert!(!is_walking(false, 4.0));
        assert!(!is_walking(true, 0.2));
        assert!(!is_walking(true, f32::NAN));
    }

    #[test]
    fn jump_requires_leaving_ground_with_upward_velocity() {
        assert!(is_jump_start(true, false, 14.0));
        assert!(!is_jump_start(true, true, 14.0));
        assert!(!is_jump_start(false, false, 14.0));
        assert!(!is_jump_start(true, false, 0.5));
        assert!(!is_jump_start(true, false, -3.0));
    }

    #[test]
    fn server_rate_limits_steps_and_jumps() {
        assert!(server_should_accept_sound(
            PlayerSoundKind::StepStart,
            1.0,
            None,
            None
        ));
        assert!(!server_should_accept_sound(
            PlayerSoundKind::StepStart,
            1.05,
            Some(1.0),
            None
        ));
        assert!(server_should_accept_sound(
            PlayerSoundKind::StepStart,
            1.2,
            Some(1.0),
            None
        ));
        assert!(server_should_accept_sound(
            PlayerSoundKind::StepStop,
            1.01,
            Some(1.0),
            None
        ));
        assert!(!server_should_accept_sound(
            PlayerSoundKind::Jump,
            1.1,
            None,
            Some(1.0)
        ));
        assert!(server_should_accept_sound(
            PlayerSoundKind::Jump,
            1.4,
            None,
            Some(1.0)
        ));
    }

    #[test]
    fn culls_distant_sounds() {
        assert!(within_hearing_range(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)));
        assert!(!within_hearing_range(
            Vec3::ZERO,
            Vec3::new(200.0, 0.0, 0.0)
        ));
    }
}
