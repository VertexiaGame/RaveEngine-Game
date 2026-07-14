use std::path::Path;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::asset::RenderAssetUsages;
use avian3d::prelude::*;
use tobj::LoadOptions;
use super::{Player, PlayerController, CameraSettings, PlayerCamera};

#[derive(Resource)]
pub struct PlayerCharacterAssets {
    pub avatar_scene: Handle<WorldAsset>,
}

pub fn load_obj_file(
    path: &str,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Vec<(Handle<Mesh>, Handle<StandardMaterial>)> {
    let mut parts = Vec::new();
    let load_options = LoadOptions {
        single_index: true,
        triangulate: true,
        ..Default::default()
    };

    let base_path = if Path::new(path).exists() {
        path.to_string()
    } else {
        format!("assets/{}", path)
    };

    if let Ok((models, mtl_result)) = tobj::load_obj(&base_path, &load_options) {
        let tobj_materials = mtl_result.unwrap_or_default();

        for model in models {
            let tobj_mesh = &model.mesh;
            let mut bevy_mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );

            let mut positions = Vec::new();
            for i in 0..tobj_mesh.positions.len() / 3 {
                positions.push([
                    tobj_mesh.positions[3 * i],
                    tobj_mesh.positions[3 * i + 1],
                    tobj_mesh.positions[3 * i + 2],
                ]);
            }
            bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

            if !tobj_mesh.normals.is_empty() {
                let mut normals = Vec::new();
                for i in 0..tobj_mesh.normals.len() / 3 {
                    normals.push([
                        tobj_mesh.normals[3 * i + 1],
                        tobj_mesh.normals[3 * i + 2],
                    ]);
                }
                bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            }

            if !tobj_mesh.texcoords.is_empty() {
                let mut uvs = Vec::new();
                for i in 0..tobj_mesh.texcoords.len() / 2 {
                    uvs.push([
                        tobj_mesh.texcoords[2 * i],
                        tobj_mesh.texcoords[2 * i + 1],
                    ]);
                }
                bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            }

            bevy_mesh.insert_indices(Indices::U32(tobj_mesh.indices.clone()));
            let mesh_handle = meshes.add(bevy_mesh);

            let mut mat_handle = Handle::default();
            if let Some(mat_id) = tobj_mesh.material_id {
                if mat_id < tobj_materials.len() {
                    let tobj_mat = &tobj_materials[mat_id];
                    let base_color = if let Some(kd) = tobj_mat.diffuse {
                        Color::Srgba(Srgba::new(kd[0], kd[1], kd[2], 1.0))
                    } else {
                        Color::WHITE
                    };
                    let roughness = if let Some(ns) = tobj_mat.shininess {
                        (1.0 - (ns / 1000.0)).clamp(0.0, 1.0)
                    } else {
                        0.5
                    };
                    let std_mat = StandardMaterial {
                        base_color,
                        perceptual_roughness: roughness,
                        ..default()
                    };
                    mat_handle = materials.add(std_mat);
                }
            }

            if mat_handle == Handle::default() {
                let std_mat = StandardMaterial {
                    base_color: Color::srgb(0.8, 0.8, 0.8),
                    perceptual_roughness: 0.5,
                    ..default()
                };
                mat_handle = materials.add(std_mat);
            }

            parts.push((mesh_handle, mat_handle));
        }
    }

    parts
}

pub fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let player_id = commands
        .spawn((
            Name::new("Player"),
            Player,
            PlayerController {
                move_speed: 16.0 * 0.28,
                jump_power: 50.0 * 0.28,
            },
            Transform::from_xyz(0.0, 3.0, 0.0),
            GlobalTransform::default(),
            RigidBody::Dynamic,
            Collider::capsule(1.0 * 0.28, 3.0 * 0.28),
            CollisionLayers::from_bits(0b0010, 0b0011),
            LockedAxes::ROTATION_LOCKED,
            Friction::new(0.0),
            Restitution::new(0.0),
            GravityScale(1.0),
            CollidingEntities::default(),
            SleepingDisabled,
        ))
        .id();

    let avatar_scene = asset_server.load("content/game/character/Legacy/Av.glb#Scene0");

    let child_id = commands
        .spawn((
            WorldAssetRoot(avatar_scene),
            Transform::from_translation(Vec3::new(0.0, -0.7, 0.0))
                .with_scale(Vec3::splat(0.28)),
            GlobalTransform::default(),
        ))
        .id();
    commands.entity(player_id).add_child(child_id);

    commands.spawn((
        Camera3d::default(),
        PlayerCamera,
        CameraSettings {
            yaw: 0.0,
            pitch: -0.35,
            distance: 4.5,
            current_distance: 4.5,
            target_offset: Vec3::new(0.0, 0.55, 0.0),
        },
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}