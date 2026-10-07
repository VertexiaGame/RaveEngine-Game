use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default, Serialize, Deserialize)]
pub enum BrickShape {
    #[default]
    Block,
    Sphere,
    Cylinder,
    Wedge,
    CornerWedge,
}

impl BrickShape {
    pub fn base_size_studs(self) -> Vec3 {
        match self {
            BrickShape::Block => Vec3::new(4.0, 1.0, 2.0),
            BrickShape::Sphere => Vec3::new(2.0, 2.0, 2.0),
            BrickShape::Cylinder => Vec3::new(4.0, 2.0, 2.0),
            BrickShape::Wedge => Vec3::new(4.0, 1.0, 2.0),
            BrickShape::CornerWedge => Vec3::new(4.0, 1.0, 2.0),
        }
    }

    pub fn base_half_extents_world(self) -> Vec3 {
        self.base_size_studs() * 0.28 * 0.5
    }

    pub fn to_u8(self) -> u8 {
        match self {
            BrickShape::Block => 0,
            BrickShape::Sphere => 1,
            BrickShape::Cylinder => 2,
            BrickShape::Wedge => 3,
            BrickShape::CornerWedge => 4,
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(BrickShape::Block),
            1 => Some(BrickShape::Sphere),
            2 => Some(BrickShape::Cylinder),
            3 => Some(BrickShape::Wedge),
            4 => Some(BrickShape::CornerWedge),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Block" | "Part" | "Cube" => Some(BrickShape::Block),
            "Sphere" | "Ball" => Some(BrickShape::Sphere),
            "Cylinder" => Some(BrickShape::Cylinder),
            "Wedge" => Some(BrickShape::Wedge),
            "CornerWedge" | "Corner Wedge" | "Cornerwedge" => Some(BrickShape::CornerWedge),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            BrickShape::Block => "Block",
            BrickShape::Sphere => "Sphere",
            BrickShape::Cylinder => "Cylinder",
            BrickShape::Wedge => "Wedge",
            BrickShape::CornerWedge => "CornerWedge",
        }
    }

    pub fn default_name_prefix(self) -> &'static str {
        match self {
            BrickShape::Block => "Part",
            BrickShape::Sphere => "Sphere",
            BrickShape::Cylinder => "Cylinder",
            BrickShape::Wedge => "Wedge",
            BrickShape::CornerWedge => "CornerWedge",
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Brick;

#[derive(Component, Clone, Copy, Debug, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct BrickShapeComponent {
    pub shape: BrickShape,
}

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct BrickPhysics {
    pub enabled: bool,
    pub bounciness: f32,
    pub player_can_collide: bool,
    pub friction: f32,
    pub gravity_scale: f32,
    pub mass: f32,
}

impl Default for BrickPhysics {
    fn default() -> Self {
        Self {
            enabled: false,
            bounciness: 0.0,
            player_can_collide: true,
            friction: 0.3,
            gravity_scale: 1.0,
            mass: 1.0,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct BrickColor {
    pub color: Color,
}

impl Default for BrickColor {
    fn default() -> Self {
        Self {
            color: Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0)),
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct BrickStuds {
    pub enabled: bool,
}

impl Default for BrickStuds {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrickMeshKey {
    pub shape: BrickShape,
    pub scale_key: [u32; 3],
}
