use crate::common::core::vrtx::VrtxFileState;
use crate::common::game::bricks::components::{
    Brick, BrickColor, BrickPhysics, BrickShape, BrickShapeComponent, BrickStuds,
};
use crate::common::net::components::NetworkTransform;
use crate::server::ServerSettings;
use bevy::prelude::*;
use lightyear::prelude::Replicate;

pub fn load_fallback_map(commands: &mut Commands) {
    commands.spawn((
        Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0)),
        Name::new("Baseplate"),
        Brick,
        BrickShapeComponent {
            shape: BrickShape::Block,
        },
        BrickPhysics {
            enabled: false,
            bounciness: 0.3,
            player_can_collide: true,
            friction: 0.3,
            gravity_scale: 1.0,
            mass: 1.0,
        },
        BrickColor {
            color: Color::srgb(0.18, 0.38, 0.18),
        },
        BrickStuds::default(),
        NetworkTransform {
            translation: Vec3::new(0.0, -0.14, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(25.0, 1.0, 50.0),
            velocity: Vec3::ZERO,
        },
        Replicate::default(),
    ));

    commands.spawn((
        Transform::from_xyz(0.0, 0.14, 0.0),
        Name::new("Part0"),
        Brick,
        BrickShapeComponent {
            shape: BrickShape::Block,
        },
        BrickPhysics {
            enabled: true,
            bounciness: 0.3,
            player_can_collide: true,
            friction: 0.3,
            gravity_scale: 1.0,
            mass: 1.0,
        },
        BrickColor {
            color: Color::srgb(0.84, 0.24, 0.16),
        },
        BrickStuds::default(),
        NetworkTransform {
            translation: Vec3::new(0.0, 0.14, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            velocity: Vec3::ZERO,
        },
        Replicate::default(),
    ));
}

pub fn load_map(mut commands: Commands, settings: Res<ServerSettings>) {
    let mut loaded = false;
    info!("Loading map: {}", settings.map_path);

    let loaded_state = VrtxFileState::load_from_file(&settings.map_path).ok();

    if let Some(state) = loaded_state {
        commands.insert_resource(avian3d::prelude::Gravity(state.gravity));
        if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write() {
            *shared = crate::studio::tools::PlayersService {
                speed: state.players.speed,
                jump_power: state.players.jump_power,
                gravity: state.players.gravity,
                speed_response: state.players.speed_response,
                friction: state.players.friction,
                bounciness: state.players.bounciness,
            };
        }
        let mut named_entities =
            std::collections::HashMap::with_capacity(state.bricks.len() + state.meshes.len());
        for brick in state.bricks {
            let name = brick.name.clone();
            let entity = spawn_brick_entity(&mut commands, brick);
            named_entities.insert(name, entity);
        }
        for script in state.scripts {
            let mut cmd = commands.spawn(Name::new(script.name));
            match script.script_type {
                0 => {
                    cmd.insert(crate::scripting::ecs::ServerScript {
                        code: script.code,
                        enabled: script.enabled,
                        started: false,
                        running_code: String::new(),
                    });
                }
                1 => {
                    cmd.insert((
                        crate::scripting::ecs::LocalScript {
                            code: script.code,
                            enabled: script.enabled,
                            started: false,
                            running_code: String::new(),
                        },
                        lightyear::prelude::Replicate::default(),
                    ));
                }
                _ => {
                    cmd.insert((
                        crate::scripting::ecs::ModuleScript { code: script.code },
                        lightyear::prelude::Replicate::default(),
                    ));
                }
            }
            let new_script_entity = cmd.id();
            if let Some(ref p_name) = script.parent_name {
                if let Some(&parent_entity) = named_entities.get(p_name) {
                    commands.entity(parent_entity).add_child(new_script_entity);
                }
            }
        }
        for image in state.images {
            let face = image
                .face
                .as_deref()
                .and_then(crate::common::game::assets::components::ImageFace::from_str);
            let cmd = commands.spawn((
                image.transform,
                Name::new(image.name),
                crate::common::game::assets::components::Image {
                    asset_id: image.asset_id,
                    face,
                },
                NetworkTransform {
                    translation: image.transform.translation,
                    rotation: image.transform.rotation,
                    scale: image.transform.scale,
                    velocity: Vec3::ZERO,
                },
                Replicate::default(),
            ));
            let new_image_entity = cmd.id();
            if let Some(ref p_name) = image.parent_name {
                if let Some(&parent_entity) = named_entities.get(p_name) {
                    commands.entity(parent_entity).add_child(new_image_entity);
                }
            }
        }
        let mut mesh_entities: Vec<(Entity, u32)> = Vec::with_capacity(state.meshes.len());
        for mesh in state.meshes {
            let mesh_asset_id = mesh.asset_id;
            let cmd = commands.spawn((
                mesh.transform,
                Name::new(mesh.name.clone()),
                crate::common::game::assets::components::Mesh {
                    asset_id: mesh.asset_id,
                    normalize: mesh.normalize,
                },
                crate::common::game::bricks::components::BrickPhysics {
                    enabled: mesh.physics_enabled,
                    bounciness: mesh.bounciness,
                    player_can_collide: mesh.player_can_collide,
                    friction: mesh.friction,
                    gravity_scale: mesh.gravity_scale,
                    mass: mesh.mass,
                },
                avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                NetworkTransform {
                    translation: mesh.transform.translation,
                    rotation: mesh.transform.rotation,
                    scale: mesh.transform.scale,
                    velocity: Vec3::ZERO,
                },
                Replicate::default(),
            ));
            let new_mesh_entity = cmd.id();
            named_entities.insert(mesh.name, new_mesh_entity);
            mesh_entities.push((new_mesh_entity, mesh_asset_id));
            if let Some(ref p_name) = mesh.parent_name {
                if let Some(&parent_entity) = named_entities.get(p_name) {
                    commands.entity(parent_entity).add_child(new_mesh_entity);
                }
            }
        }
        let mut textured_meshes =
            std::collections::HashSet::with_capacity(state.textures.len());
        for texture in state.textures {
            let parsed = crate::common::game::assets::components::Texture::parse_content_id(
                &texture.id_string,
            );
            let cmd = commands.spawn((
                Name::new(texture.name),
                crate::common::game::assets::components::Texture {
                    asset_id: parsed.map(|(id, _)| id).unwrap_or(0),
                    is_decal: parsed.map(|(_, decal)| decal).unwrap_or(false),
                },
                Replicate::default(),
            ));
            let new_texture_entity = cmd.id();
            if let Some(ref p_name) = texture.parent_name {
                if let Some(&parent_entity) = named_entities.get(p_name) {
                    commands.entity(parent_entity).add_child(new_texture_entity);
                    textured_meshes.insert(parent_entity);
                }
            }
        }
        for (mesh_entity, mesh_asset_id) in mesh_entities {
            if !textured_meshes.contains(&mesh_entity) {
                let texture_entity = commands
                    .spawn((
                        Name::new("Texture"),
                        crate::common::game::assets::components::Texture {
                            asset_id: mesh_asset_id,
                            is_decal: false,
                        },
                        Replicate::default(),
                    ))
                    .id();
                commands.entity(mesh_entity).add_child(texture_entity);
            }
        }
        loaded = true;
        info!("Map loaded successfully");
    }

    if !loaded {
        info!("Failed to load map, spawning fallback map instead");
        load_fallback_map(&mut commands);
    }
}

pub fn spawn_brick_entity(
    commands: &mut Commands,
    brick: crate::common::core::vrtx::VrtxBrick,
) -> Entity {
    commands
        .spawn((
            brick.transform,
            Name::new(brick.name),
            Brick,
            BrickShapeComponent { shape: brick.shape },
            BrickPhysics {
                enabled: brick.physics_enabled,
                bounciness: brick.bounciness,
                player_can_collide: brick.player_can_collide,
                friction: brick.friction,
                gravity_scale: brick.gravity_scale,
                mass: brick.mass,
            },
            BrickColor { color: brick.color },
            BrickStuds {
                enabled: brick.show_studs,
            },
            NetworkTransform {
                translation: brick.transform.translation,
                rotation: brick.transform.rotation,
                scale: brick.transform.scale,
                velocity: Vec3::ZERO,
            },
            Replicate::default(),
        ))
        .id()
}
