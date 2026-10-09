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
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Sound {
    pub asset_id: u32,
    pub volume: f32,
    pub speed: f32,
    pub looped: bool,
    pub replicate_time: bool,
    pub playing: bool,
    pub spatial: bool,
    pub position: f32,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            asset_id: 0,
            volume: 50.0,
            speed: 1.0,
            looped: false,
            replicate_time: false,
            playing: false,
            spatial: false,
            position: 0.0,
        }
    }
}

impl Sound {
    pub const MAX_VOLUME: f32 = 100.0;
    pub const MIN_SPEED: f32 = 0.1;
    pub const MAX_SPEED: f32 = 20.0;

    pub fn clamp_volume(volume: f32) -> f32 {
        if volume.is_finite() {
            volume.clamp(0.0, Self::MAX_VOLUME)
        } else {
            Self::default().volume
        }
    }

    pub fn clamp_speed(speed: f32) -> f32 {
        if speed.is_finite() {
            speed.clamp(Self::MIN_SPEED, Self::MAX_SPEED)
        } else {
            1.0
        }
    }

    pub fn linear_volume(&self) -> f32 {
        Self::clamp_volume(self.volume) / Self::MAX_VOLUME
    }

    pub fn clamped_speed(&self) -> f32 {
        Self::clamp_speed(self.speed)
    }

    pub fn as_content_id(&self) -> String {
        format!("sound/{}", self.asset_id)
    }

    pub fn parse_content_id(s: &str) -> Option<u32> {
        let s = s.trim();
        let stripped = s.strip_prefix("sound/").unwrap_or(s);
        stripped.parse::<u32>().ok()
    }
}
