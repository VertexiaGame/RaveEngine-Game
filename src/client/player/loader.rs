use std::path::Path;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::asset::RenderAssetUsages;
use tobj::LoadOptions;

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