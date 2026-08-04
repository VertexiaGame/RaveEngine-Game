pub mod compute;
pub mod config;
pub mod images;
pub mod render;
pub mod skybox;
pub mod ui;
pub mod uniforms;

pub use config::CloudsConfig;

use bevy::prelude::*;

use self::{
    compute::{CameraMatrices, CloudsComputePlugin},
    images::build_images,
    render::{CloudsMaterial, CloudsShaderPlugin},
    skybox::{init_skybox_mesh, update_skybox_transform, SkyboxMaterials},
    ui::ui_system,
    uniforms::CloudsImage,
};

pub struct CloudsPlugin;

impl Plugin for CloudsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CloudsConfig::default())
            .add_plugins((CloudsComputePlugin, CloudsShaderPlugin))
            .add_systems(Startup, clouds_setup)
            .add_systems(
                PostUpdate,
                (update_skybox_transform, update_camera_matrices)
                    .after(TransformSystems::Propagate),
            );
        app.add_systems(bevy_egui::EguiPrimaryContextPass, ui_system);
    }
}

fn clouds_setup(
    mut commands: Commands,
    images: ResMut<Assets<Image>>,
    meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CloudsMaterial>>,
) {
    let (cloud_render_image, cloud_atlas_image, cloud_worley_image, sky_image) =
        build_images(images);

    let material = materials.add(CloudsMaterial {
        cloud_render_image: cloud_render_image.clone(),
        cloud_atlas_image: cloud_atlas_image.clone(),
        cloud_worley_image: cloud_worley_image.clone(),
        sky_image: sky_image.clone(),
    });
    init_skybox_mesh(
        &mut commands,
        meshes,
        SkyboxMaterials::from_one_material(MeshMaterial3d(material.clone())),
    );
    commands.insert_resource(CloudsImage {
        cloud_render_image,
        cloud_atlas_image,
        cloud_worley_image,
        sky_image,
    });
    commands.insert_resource(CameraMatrices {
        translation: Vec3::ZERO,
        inverse_camera_projection: Mat4::IDENTITY,
        inverse_camera_view: Mat4::IDENTITY,
    });
}

fn update_camera_matrices(
    cam_query: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    mut config: ResMut<CameraMatrices>,
) {
    for (camera_transform, camera) in &cam_query {
        if camera.is_active {
            config.translation = camera_transform.translation();
            config.inverse_camera_view = camera_transform.to_matrix();
            config.inverse_camera_projection = camera.computed.clip_from_view.inverse();
            break;
        }
    }
}