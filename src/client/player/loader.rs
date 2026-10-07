use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use tobj::LoadOptions;
use crate::common::game::bricks::{
    BRICK_METALLIC, BRICK_PERCEPTUAL_ROUGHNESS, BRICK_REFLECTANCE,
};

pub const AVATAR_BODY_PARTS: [(&str, &str); 6] = [
    ("Head", "content/game/character/Head.obj"),
    ("Torso", "content/game/character/Torso.obj"),
    ("LeftArm", "content/game/character/LeftArm.obj"),
    ("RightArm", "content/game/character/RightArm.obj"),
    ("LeftLeg", "content/game/character/LeftLeg.obj"),
    ("RightLeg", "content/game/character/RightLeg.obj"),
];

pub struct AvatarBodyPart {
    pub name: &'static str,
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub struct AvatarBodyAssets {
    pub parts: Vec<AvatarBodyPart>,
}

impl AvatarBodyAssets {
    pub fn load(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let mut parts = Vec::new();
        for (name, path) in AVATAR_BODY_PARTS {
            match load_obj_file(path, meshes, materials).into_iter().next() {
                Some((mesh, material)) => parts.push(AvatarBodyPart {
                    name,
                    mesh,
                    material,
                }),
                None => warn!("Failed to load avatar body part {} from {}", name, path),
            }
        }
        Self { parts }
    }
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

    let base_path = crate::common::assets_path::resolve_asset_path(path)
        .to_string_lossy()
        .into_owned();

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
            let vertex_count = positions.len();
            bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

            if tobj_mesh.normals.len() / 3 == vertex_count {
                let mut normals = Vec::new();
                for i in 0..vertex_count {
                    normals.push([
                        tobj_mesh.normals[3 * i],
                        tobj_mesh.normals[3 * i + 1],
                        tobj_mesh.normals[3 * i + 2],
                    ]);
                }
                bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            }

            if tobj_mesh.texcoords.len() / 2 == vertex_count {
                let mut uvs = Vec::new();
                for i in 0..vertex_count {
                    uvs.push([tobj_mesh.texcoords[2 * i], tobj_mesh.texcoords[2 * i + 1]]);
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
                    let std_mat = StandardMaterial {
                        base_color,
                        perceptual_roughness: BRICK_PERCEPTUAL_ROUGHNESS,
                        metallic: BRICK_METALLIC,
                        reflectance: BRICK_REFLECTANCE,
                        ..default()
                    };
                    mat_handle = materials.add(std_mat);
                }
            }

            if mat_handle == Handle::default() {
                let std_mat = StandardMaterial {
                    base_color: Color::srgb(0.8, 0.8, 0.8),
                    perceptual_roughness: BRICK_PERCEPTUAL_ROUGHNESS,
                    metallic: BRICK_METALLIC,
                    reflectance: BRICK_REFLECTANCE,
                    ..default()
                };
                mat_handle = materials.add(std_mat);
            }

            parts.push((mesh_handle, mat_handle));
        }
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_all_six_body_parts_with_normals() {
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let assets = AvatarBodyAssets::load(&mut meshes, &mut materials);

        let names: Vec<&str> = assets.parts.iter().map(|part| part.name).collect();
        assert_eq!(
            names,
            vec![
                "Head", "Torso", "LeftArm", "RightArm", "LeftLeg", "RightLeg"
            ]
        );

        for part in &assets.parts {
            let mesh = meshes
                .get(&part.mesh)
                .unwrap_or_else(|| panic!("{} mesh missing", part.name));
            assert!(mesh.count_vertices() > 0, "{} has no vertices", part.name);
            assert!(
                mesh.contains_attribute(Mesh::ATTRIBUTE_POSITION),
                "{} has no positions",
                part.name
            );
            assert!(
                mesh.contains_attribute(Mesh::ATTRIBUTE_NORMAL),
                "{} has no normals",
                part.name
            );
            assert!(mesh.indices().is_some(), "{} has no indices", part.name);
            assert!(
                materials.get(&part.material).is_some(),
                "{} material missing",
                part.name
            );
        }
    }
}
