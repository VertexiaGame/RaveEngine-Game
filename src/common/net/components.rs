use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Component)]
pub struct Brick;

#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Player {
    pub client_id: u64,
    pub speed: f32,
    pub jump_power: f32,
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct NetworkTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct PlayersServiceContainer;