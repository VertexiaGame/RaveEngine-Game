use bevy::prelude::*;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::anti_alias::fxaa::Fxaa;
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::camera::Hdr;
use bevy::post_process::bloom::Bloom;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use crate::common::components::Brick;

#[derive(Resource, Default)]
pub struct BrickSpawnerCount {
    pub count: u32,
}

pub fn setup_studio(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut count: ResMut<BrickSpawnerCount>,
) {
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            far: 3000.0,
            ..default()
        }),
        Hdr,
        Msaa::Off,
        Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
        MeshPickingCamera,
        FreeCamera::default(),
        DepthPrepass,
        NormalPrepass,
        MotionVectorPrepass,
        Fxaa::default(),
        Bloom::default(),
        ScreenSpaceAmbientOcclusion::default(),
        DistanceFog {
            color: Color::srgb(0.70, 0.90, 1.00),
            falloff: FogFalloff::Linear {
                start: 400.0,
                end: 1100.0,
            },
            ..default()
        },
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(50.0, 0.1, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.62, 0.32),
            perceptual_roughness: 0.95,
            reflectance: 0.08,
            metallic: 0.0,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.05, 0.0),
        Name::new("Baseplate"),
    ));

    spawn_brick(&mut commands, &mut meshes, &mut materials, &mut count);
}

pub fn spawn_brick(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    count: &mut BrickSpawnerCount,
) {
    let offset = (count.count as f32) * 5.0;
    let current_index = count.count;
    count.count += 1;

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 4.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.84, 0.24, 0.16),
            perceptual_roughness: 0.95,
            reflectance: 0.08,
            metallic: 0.0,
            ..default()
        })),
        Transform::from_xyz(offset, 0.5, 0.0),
        Brick,
        Pickable::default(),
        Name::new(format!("Part{}", current_index)),
    ));
}