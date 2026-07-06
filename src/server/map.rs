use bevy::prelude::*;
use lightyear::prelude::Replicate;
use avian3d::prelude::*;
use crate::common::vrtx::VrtxFileState;
use crate::common::bricks::components::{Brick, BrickShape, BrickShapeComponent, BrickPhysics, BrickColor};
use crate::common::components::NetworkTransform;
use crate::server::ServerSettings;

pub fn load_fallback_map(
    commands: &mut Commands,
) {
    commands.spawn((
        Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0)),
        Name::new("Baseplate"),
        Brick,
        BrickShapeComponent { shape: BrickShape::Block },
        BrickPhysics {
            enabled: false,
            bounciness: 0.3,
        },
        BrickColor { color: Color::srgb(0.28, 0.62, 0.32) },
        NetworkTransform {
            translation: Vec3::new(0.0, -0.14, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(25.0, 1.0, 50.0),
        },
        RigidBody::Static,
        Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
        Friction::new(0.3),
        Restitution::new(0.3),
        Replicate::default(),
    ));

    commands.spawn((
        Transform::from_xyz(0.0, 0.14, 0.0),
        Name::new("Part0"),
        Brick,
        BrickShapeComponent { shape: BrickShape::Block },
        BrickPhysics {
            enabled: true,
            bounciness: 0.3,
        },
        BrickColor { color: Color::srgb(0.84, 0.24, 0.16) },
        NetworkTransform {
            translation: Vec3::new(0.0, 0.14, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
        RigidBody::Dynamic,
        Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
        Friction::new(0.3),
        Restitution::new(0.3),
        SleepingDisabled,
        Replicate::default(),
    ));
}

pub fn load_map(
    mut commands: Commands,
    settings: Res<ServerSettings>,
) {
    let mut loaded = false;
    info!("Loading map: {}", settings.map_path);
    if let Ok(state) = VrtxFileState::load_from_file(&settings.map_path) {
        if state.version == 0 {
            info!("Loaded legacy VRTX file format (version 0)");
        }
        for brick in state.bricks {
            spawn_brick_entity(&mut commands, brick);
        }
        loaded = true;
        info!("Map loaded successfully");
    }

    if !loaded {
        info!("Failed to load map, spawning fallback map instead");
        load_fallback_map(&mut commands);
    }
}

pub fn spawn_brick_entity(commands: &mut Commands, brick: crate::common::vrtx::VrtxBrick) {
    let collider = match brick.shape {
        BrickShape::Block => {
            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
        }
        BrickShape::Sphere => {
            Collider::sphere(1.0 * 0.28)
        }
    };

    let body_type = if brick.physics_enabled {
        RigidBody::Dynamic
    } else {
        RigidBody::Static
    };

    commands.spawn((
        brick.transform,
        Name::new(brick.name.clone()),
        Brick,
        BrickShapeComponent { shape: brick.shape },
        BrickPhysics {
            enabled: brick.physics_enabled,
            bounciness: brick.bounciness,
        },
        BrickColor { color: brick.color },
        NetworkTransform {
            translation: brick.transform.translation,
            rotation: brick.transform.rotation,
            scale: brick.transform.scale,
        },
        body_type,
        collider,
        Friction::new(0.3),
        Restitution::new(brick.bounciness),
        SleepingDisabled,
        Replicate::default(),
    ));
}