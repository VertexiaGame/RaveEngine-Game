use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component)]
pub struct Brick;

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Player {
    pub client_id: u64,
    pub speed: f32,
    pub jump_power: f32,
    #[serde(default = "default_player_gravity")]
    pub gravity: f32,
    #[serde(default)]
    pub speed_response: crate::common::game::movement::SpeedResponse,
    #[serde(default)]
    pub friction: f32,
    #[serde(default)]
    pub bounciness: f32,
    pub username: String,
}

fn default_player_gravity() -> f32 {
    crate::common::game::movement::DEFAULT_PLAYER_GRAVITY
}

impl Default for Player {
    fn default() -> Self {
        Self {
            client_id: 0,
            speed: 0.0,
            jump_power: 0.0,
            gravity: crate::common::game::movement::DEFAULT_PLAYER_GRAVITY,
            speed_response: crate::common::game::movement::SpeedResponse::Linear,
            friction: 0.0,
            bounciness: 0.0,
            username: String::new(),
        }
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct NetworkTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub velocity: Vec3,
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct PlayersServiceContainer;

#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct LightingServiceContainer;

#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct AssetServiceContainer;
