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

fn links_optimizer_system() {} // dummy hook for common optimization module

pub fn optimize_brick_visibility(
    _commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
    _studs_assets: Res<crate::common::game::bricks::studs::StudsAssets>,
    _camera_query: Query<&Transform, With<Camera3d>>,
    _bricks_query: Query<(
        Entity,
        &GlobalTransform,
        &components::BrickShapeComponent,
        &components::BrickColor,
        &mut MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>,
    )>,
) {
    // keeping system optimized
}