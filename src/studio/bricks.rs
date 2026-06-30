use bevy::prelude::*;
use bevy::pbr::ExtendedMaterial;
use crate::common::components::Brick;

#[derive(Resource, Default)]
pub struct BrickSpawnerCount {
    pub count: u32,
}

pub fn spawn_brick(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>,
    studs_assets: &crate::studio::studs::StudsAssets,
    count: &mut BrickSpawnerCount,
    spawn_pos: Vec3,
) -> Entity {
    let current_index = count.count;
    count.count += 1;

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))),
        MeshMaterial3d(materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.84, 0.24, 0.16),
                perceptual_roughness: 0.9,
                ..default()
            },
            extension: crate::studio::studs::StudsExtension {
                stud_texture: studs_assets.stud.clone(),
                inlet_texture: studs_assets.inlet.clone(),
            },
        })),
        Transform::from_translation(spawn_pos),
        Brick,
        Pickable::default(),
        Name::new(format!("Part{}", current_index)),
    )).id()
}