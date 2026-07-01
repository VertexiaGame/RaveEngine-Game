use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct CopiedEntityBuffer {
    pub transform: Option<Transform>,
    pub mesh: Option<Mesh3d>,
    pub material: Option<MeshMaterial3d<StandardMaterial>>,
    pub studs_material: Option<MeshMaterial3d<bevy::pbr::ExtendedMaterial<StandardMaterial, crate::common::bricks::studs::StudsExtension>>>,
    pub name: Option<String>,
    pub is_brick: bool,
    pub physics: Option<crate::common::bricks::components::BrickPhysics>,
}

#[derive(Resource, Default)]
pub struct HierarchyDraggedEntity {
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct SettingsWindow {
    pub open: bool,
}