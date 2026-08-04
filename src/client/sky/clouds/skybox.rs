use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct SkyboxMesh;

pub(crate) struct SkyboxMaterials<M: Material> {
    pub nx: MeshMaterial3d<M>,
    pub ny: MeshMaterial3d<M>,
    pub nz: MeshMaterial3d<M>,
    pub px: MeshMaterial3d<M>,
    pub py: MeshMaterial3d<M>,
    pub pz: MeshMaterial3d<M>,
}

impl<M: Material> SkyboxMaterials<M> {
    pub fn from_one_material(material: MeshMaterial3d<M>) -> Self {
        Self {
            nx: material.clone(),
            ny: material.clone(),
            nz: material.clone(),
            px: material.clone(),
            py: material.clone(),
            pz: material.clone(),
        }
    }
}

pub(crate) fn init_skybox_mesh<M: Material>(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    standard_materials: SkyboxMaterials<M>,
) {
    let mesh = meshes.add(Cuboid::new(2.0, 2.0, 2.0));

    commands.spawn((
        Mesh3d(mesh),
        standard_materials.px,
        Transform::default(),
        SkyboxMesh,
        NotShadowCaster,
        NotShadowReceiver,
    ));
}

pub(crate) fn update_skybox_transform(
    camera_query: Query<(&Transform, &Camera, &Projection), (With<Camera3d>, Without<SkyboxMesh>)>,
    mut skybox: Query<&mut Transform, With<SkyboxMesh>>,
) {
    for (camera_transform, camera, projection) in &camera_query {
        if !camera.is_active {
            continue;
        }
        let far = match projection {
            Projection::Perspective(pers) => pers.far,
            _ => continue,
        };
        let scale = (far * 0.4).min(5000.0);

        for mut transform in skybox.iter_mut() {
            transform.translation = camera_transform.translation;
            transform.scale = Vec3::splat(scale);
        }
        break;
    }
}