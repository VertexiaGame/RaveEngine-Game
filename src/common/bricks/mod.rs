pub mod components;
pub mod studs;
pub mod data;

use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialPlugin};

#[derive(Resource, Default)]
pub struct BrickMaterialCache {
    pub studs_materials: std::collections::HashMap<Color, Handle<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    pub plain_materials: std::collections::HashMap<Color, Handle<StandardMaterial>>,
}

pub struct BricksPlugin;

impl Plugin for BricksPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<components::BrickPhysics>()
            .register_type::<components::Brick>()
            .register_type::<components::BrickShapeComponent>()
            .register_type::<components::BrickColor>()
            .init_resource::<data::BrickSpawnerCount>()
            .init_resource::<BrickMaterialCache>();

        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>::default())
                .add_systems(Startup, studs::setup_studs)
                .add_systems(Update, (
                    studs::configure_studs_samplers,
                    update_brick_meshes_on_shape_change,
                    links_optimizer_system,
                ));
        }
    }
}

pub fn update_brick_meshes_on_shape_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(Entity, &components::BrickShapeComponent), Changed<components::BrickShapeComponent>>,
) {
    for (entity, brick_shape_comp) in &query {
        match brick_shape_comp.shape {
            components::BrickShape::Block => {
                commands.entity(entity).insert(Mesh3d(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))));
            }
            components::BrickShape::Sphere => {
                commands.entity(entity).insert(Mesh3d(meshes.add(Sphere::new(1.0 * 0.28))));
            }
        }
    }
}

fn links_optimizer_system() {} // dummy hook for common optimization module

pub fn optimize_brick_visibility(
    _commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, crate::common::bricks::studs::StudsExtension>>>,
    _studs_assets: Res<crate::common::bricks::studs::StudsAssets>,
    _camera_query: Query<&Transform, With<Camera3d>>,
    _bricks_query: Query<(
        Entity,
        &GlobalTransform,
        &components::BrickShapeComponent,
        &components::BrickColor,
        &mut MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::bricks::studs::StudsExtension>>,
    )>,
) {
    // keeping system optimized
}