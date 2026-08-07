use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct GameChannel;

pub struct InputChannel;

#[derive(Message, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PlayerInputMessage {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub jump: bool,
    pub yaw: f32,
    pub in_first_person: bool,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct HelloMessage {
    pub ukey: String,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct KickMessage {
    pub reason: String,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct AuthSuccessMessage {
    pub uid: i32,
    pub username: String,
}