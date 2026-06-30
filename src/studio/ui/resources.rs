use bevy::prelude::*;
use bevy::pbr::ExtendedMaterial;

#[derive(Resource, Default)]
pub struct CopiedEntityBuffer {
    pub transform: Option<Transform>,
    pub mesh: Option<Mesh3d>,
    pub material: Option<MeshMaterial3d<StandardMaterial>>,
    pub studs_material: Option<MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>>,
    pub name: Option<String>,
    pub is_brick: bool,
}

#[derive(Resource, Default)]
pub struct HierarchyDraggedEntity {
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct SettingsWindow {
    pub open: bool,
}