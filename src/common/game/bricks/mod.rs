pub mod components;
pub mod studs;
pub mod data;

use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialPlugin};

#[derive(Resource, Default)]
pub struct BrickMaterialCache {
    pub studs_materials: std::collections::HashMap<[u32; 4], Handle<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    pub plain_materials: std::collections::HashMap<[u32; 4], Handle<StandardMaterial>>,
    pub block_mesh: Option<Handle<Mesh>>,
    pub sphere_mesh: Option<Handle<Mesh>>,
}

#[derive(Resource)]
pub struct WorkspaceShowStuds {
    pub enabled: bool,
}

impl Default for WorkspaceShowStuds {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub struct BricksPlugin;

impl Plugin for BricksPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<components::BrickPhysics>()
            .register_type::<components::Brick>()
            .register_type::<components::BrickShapeComponent>()
            .register_type::<components::BrickColor>()
            .register_type::<components::BrickStuds>()
            .init_resource::<data::BrickSpawnerCount>()
            .init_resource::<BrickMaterialCache>()
            .init_resource::<WorkspaceShowStuds>();

        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>::default())
                .add_systems(Startup, studs::setup_studs)
                .add_systems(Update, (
                    studs::configure_studs_samplers,
                    update_brick_meshes_on_shape_change,
                    apply_workspace_show_studs,
                    links_optimizer_system,
                    optimize_brick_visibility,
                ));
        } else {
            app.add_systems(Update, apply_workspace_show_studs);
        }
    }
}

pub fn apply_workspace_show_studs(
    workspace: Res<WorkspaceShowStuds>,
    mut commands: Commands,
    query: Query<(Entity, Option<&components::BrickStuds>), With<components::Brick>>,
) {
    if !workspace.is_changed() {
        return;
    }
    for (entity, studs) in &query {
        let enabled = studs.map(|s| s.enabled).unwrap_or(true);
        commands.entity(entity).insert(components::BrickStuds { enabled });
    }
}

pub fn studs_material_for_color(
    cache: &mut BrickMaterialCache,
    studs_materials: &mut Assets<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>,
    studs_assets: &studs::StudsAssets,
    base_color: Color,
) -> Handle<ExtendedMaterial<StandardMaterial, studs::StudsExtension>> {
    let srgba = base_color.to_srgba();
    let cache_key = [
        srgba.red.to_bits(),
        srgba.green.to_bits(),
        srgba.blue.to_bits(),
        srgba.alpha.to_bits(),
    ];

    if let Some(existing) = cache.studs_materials.get(&cache_key) {
        existing.clone()
    } else {
        let new_mat = studs_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color,
                perceptual_roughness: 0.85,
                alpha_mode: if base_color.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
                ..default()
            },
            extension: studs::StudsExtension {
                stud_texture: studs_assets.stud.clone(),
                inlet_texture: studs_assets.inlet.clone(),
                stud_ambient_texture: studs_assets.stud_ambient.clone(),
                stud_height_texture: studs_assets.stud_height.clone(),
                inlet_ambient_texture: studs_assets.inlet_ambient.clone(),
                inlet_height_texture: studs_assets.inlet_height.clone(),
            },
        });
        cache.studs_materials.insert(cache_key, new_mat.clone());
        new_mat
    }
}

pub fn plain_material_for_color(
    cache: &mut BrickMaterialCache,
    plain_materials: &mut Assets<StandardMaterial>,
    base_color: Color,
) -> Handle<StandardMaterial> {
    let srgba = base_color.to_srgba();
    let cache_key = [
        srgba.red.to_bits(),
        srgba.green.to_bits(),
        srgba.blue.to_bits(),
        srgba.alpha.to_bits(),
    ];

    if let Some(existing) = cache.plain_materials.get(&cache_key) {
        existing.clone()
    } else {
        let new_mat = plain_materials.add(StandardMaterial {
            base_color,
            perceptual_roughness: 0.85,
            alpha_mode: if base_color.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
            ..default()
        });
        cache.plain_materials.insert(cache_key, new_mat.clone());
        new_mat
    }
}

pub fn update_brick_meshes_on_shape_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<BrickMaterialCache>,
    query: Query<(Entity, &components::BrickShapeComponent), Changed<components::BrickShapeComponent>>,
) {
    for (entity, brick_shape_comp) in &query {
        match brick_shape_comp.shape {
            components::BrickShape::Block => {
                if cache.block_mesh.is_none() {
                    cache.block_mesh = Some(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)));
                }
                commands.entity(entity).insert(Mesh3d(cache.block_mesh.clone().unwrap()));
            }
            components::BrickShape::Sphere => {
                if cache.sphere_mesh.is_none() {
                    cache.sphere_mesh = Some(meshes.add(Sphere::new(1.0 * 0.28)));
                }
                commands.entity(entity).insert(Mesh3d(cache.sphere_mesh.clone().unwrap()));
            }
        }
    }
}

#[cfg(feature = "bench")]
fn spawn_bricks_benchmark(mut commands: Commands) {
    use avian3d::prelude::*;

    commands.spawn((
        Name::new("BenchGround"),
        Transform::from_xyz(0.0, -0.56, 0.0),
        RigidBody::Static,
        Collider::cuboid(24.0, 0.56, 24.0),
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
    ));

    let mut index = 0u32;
    for x in 0..12u32 {
        for z in 0..12u32 {
            for y in 0..4u32 {
                let pos = Vec3::new(
                    (x as f32 - 5.5) * 1.2,
                    0.6 + y as f32 * 0.34,
                    (z as f32 - 5.5) * 0.65,
                );
                let shape = if index % 9 == 0 {
                    components::BrickShape::Sphere
                } else {
                    components::BrickShape::Block
                };
                let collider = match shape {
                    components::BrickShape::Block => Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
                    components::BrickShape::Sphere => Collider::sphere(1.0 * 0.28),
                };
                commands.spawn((
                    Name::new(format!("BenchBrick{}", index)),
                    components::Brick,
                    components::BrickShapeComponent { shape },
                    components::BrickPhysics::default(),
                    components::BrickColor::default(),
                    Transform::from_translation(pos),
                    RigidBody::Dynamic,
                    collider,
                    CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                    SleepingDisabled,
                ));
                index += 1;
            }
        }
    }
    info!("BENCH: Spawned {} dynamic bricks", index);
}

#[cfg(feature = "bench")]
fn toggle_brick_shapes(
    mut frame: Local<u64>,
    mut query: Query<&mut components::BrickShapeComponent, With<components::Brick>>,
) {
    *frame += 1;
    if *frame % 15 != 0 {
        return;
    }
    for (i, mut shape) in query.iter_mut().enumerate() {
        if i % 4 == 0 {
            shape.shape = if shape.shape == components::BrickShape::Block {
                components::BrickShape::Sphere
            } else {
                components::BrickShape::Block
            };
        }
    }
}

#[cfg(feature = "bench")]
fn record_bricks_assets(
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    mut stats: ResMut<crate::common::core::bench::BenchStats>,
) {
    stats.set_asset_counts(meshes.len(), materials.len());
}

#[cfg(feature = "bench")]
pub fn add_bricks_benchmark(app: &mut App) {
    app.init_resource::<BrickMaterialCache>()
        .init_asset::<StandardMaterial>()
        .add_systems(Startup, spawn_bricks_benchmark)
        .add_systems(Update, (
            update_brick_meshes_on_shape_change,
            toggle_brick_shapes,
        ))
        .add_systems(Last, record_bricks_assets.before(crate::common::core::bench::bench_finish_frame));
}

fn links_optimizer_system() {} // dummy hook for common optimization module

const STUD_LOD_DISTANCE_SQ: f32 = 80.0 * 80.0;

pub fn optimize_brick_visibility(
    mut commands: Commands,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    mut plain_materials: ResMut<Assets<StandardMaterial>>,
    studs_assets: Res<studs::StudsAssets>,
    camera_query: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    bricks_query: Query<(
        Entity,
        &GlobalTransform,
        &components::BrickColor,
        Option<&components::BrickStuds>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
        Option<&MeshMaterial3d<StandardMaterial>>,
    ), With<components::Brick>>,
    workspace_studs: Option<Res<WorkspaceShowStuds>>,
    mut cache: ResMut<BrickMaterialCache>,
    mut last_camera_position: Local<Option<Vec3>>,
) {
    let Some((camera_transform, camera)) = camera_query.iter().next() else {
        return;
    };
    if !camera.is_active {
        return;
    }

    let cam_pos = camera_transform.translation();
    let moved = last_camera_position
        .map(|previous| previous.distance_squared(cam_pos) > 4.0)
        .unwrap_or(true);
    *last_camera_position = Some(cam_pos);
    if !moved {
        return;
    }

    let show_studs_globally = workspace_studs.map(|w| w.enabled).unwrap_or(true);
    for (entity, transform, color, studs, studs_material, plain_material) in &bricks_query {
        let want_studs = show_studs_globally
            && studs.map(|s| s.enabled).unwrap_or(true)
            && transform.translation().distance_squared(cam_pos) <= STUD_LOD_DISTANCE_SQ;

        if want_studs == studs_material.is_some() {
            continue;
        }

        let base_color = if let Some(studs_mat_handle) = studs_material {
            studs_materials
                .get(&studs_mat_handle.0)
                .map(|mat| mat.base.base_color)
                .unwrap_or(color.color)
        } else if let Some(plain_mat_handle) = plain_material {
            plain_materials
                .get(&plain_mat_handle.0)
                .map(|mat| mat.base_color)
                .unwrap_or(color.color)
        } else {
            color.color
        };

        if want_studs {
            commands.entity(entity).insert(MeshMaterial3d(
                studs_material_for_color(&mut cache, &mut studs_materials, &studs_assets, base_color),
            ));
        } else {
            commands.entity(entity).insert(MeshMaterial3d(
                plain_material_for_color(&mut cache, &mut plain_materials, base_color),
            ));
        }
    }
}