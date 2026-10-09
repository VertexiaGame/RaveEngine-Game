pub mod animation;
pub mod loader;
pub mod play_camera;
pub mod sounds;

use bevy::prelude::*;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct CameraSettings {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub current_distance: f32,
    pub target_offset: Vec3,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(sounds::PlayerSoundsPlugin);
        app.add_plugins(crate::common::game::assets::sounds::SoundInstancesPlugin);
        app.add_systems(
            Update,
            (
                animation::update_avatar_anim_state,
                animation::pose_avatar_limbs.after(animation::update_avatar_anim_state),
            )
                .run_if(crate::client::is_playtesting),
        )
        .add_systems(
            PostUpdate,
            (
                crate::client::interpolate_local_player_transform
                    .before(crate::client::player::play_camera::update_camera),
                crate::client::player::play_camera::update_camera
                    .before(bevy::transform::TransformSystems::Propagate),
            )
                .run_if(crate::client::is_playtesting),
        );
    }
}
