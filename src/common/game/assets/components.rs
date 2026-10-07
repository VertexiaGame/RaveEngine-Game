use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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

#[derive(
    Component, Clone, Copy, Debug, PartialEq, Eq, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component)]
pub struct Image {
    pub asset_id: u32,
    pub face: Option<ImageFace>,
}

#[derive(
    Component, Clone, Copy, Debug, PartialEq, Eq, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component)]
pub struct Mesh {
    pub asset_id: u32,
    pub normalize: bool,
}
#[derive(
    Component, Clone, Copy, Debug, PartialEq, Eq, Reflect, Default, Serialize, Deserialize,
)]
#[reflect(Component)]
pub struct Texture {
    pub asset_id: u32,
    pub is_decal: bool,
}

impl Texture {
    pub fn as_content_id(&self) -> String {
        if self.is_decal {
            format!("image/{}", self.asset_id)
        } else {
            format!("mesh/{}", self.asset_id)
        }
    }
    pub fn parse_content_id(s: &str) -> Option<(u32, bool)> {
        let s = s.trim();
        if let Some(rest) = s.strip_prefix("mesh/") {
            rest.parse::<u32>().ok().map(|v| (v, false))
        } else {
            let stripped = s.strip_prefix("image/").unwrap_or(s);
            stripped.parse::<u32>().ok().map(|v| (v, true))
        }
    }
}
