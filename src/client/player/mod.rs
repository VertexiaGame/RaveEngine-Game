pub mod play_camera;
pub mod loader;
pub mod animation;
pub mod model;

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
        app.init_resource::<animation::PlayerAnimationGraphLoaded>()
            .init_resource::<animation::AvatarAnimationsRetargeted>()
            .add_systems(
                Update,
                (
                    animation::add_missing_animation_players,
                    animation::build_avatar_animation_graph,
                    animation::retarget_avatar_clips,
                    animation::init_player_animations,
                    animation::track_remote_player_animation,
                    animation::track_remote_player_grounded,
                    animation::track_local_player_animation
                        .after(animation::track_remote_player_animation),
                    animation::animate_player.after(animation::track_local_player_animation),
                ).run_if(crate::client::is_playtesting),
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