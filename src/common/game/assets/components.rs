use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFace {
    Top,
    Bottom,
    Left,
    Right,
    #[default]
    Front,
    Back,
}

impl ImageFace {
    pub fn as_str(self) -> &'static str {
        match self {
            ImageFace::Top => "top",
            ImageFace::Bottom => "bottom",
            ImageFace::Left => "left",
            ImageFace::Right => "right",
            ImageFace::Front => "front",
            ImageFace::Back => "back",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Top" | "top" => Some(ImageFace::Top),
            "Bottom" | "bottom" => Some(ImageFace::Bottom),
            "Left" | "left" => Some(ImageFace::Left),
            "Right" | "right" => Some(ImageFace::Right),
            "Front" | "front" => Some(ImageFace::Front),
            "Back" | "back" => Some(ImageFace::Back),
            _ => None,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Image {
    pub asset_id: u32,
    pub face: Option<ImageFace>,
}