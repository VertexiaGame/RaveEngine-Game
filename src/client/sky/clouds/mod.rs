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
    images::{build_images, build_render_images_with_size, RENDER_HEIGHT, RENDER_WIDTH},
    render::{CloudsMaterial, CloudsShaderPlugin},
    skybox::{init_skybox_mesh, update_skybox_transform, SkyboxMaterials},
    ui::ui_system,
    uniforms::CloudsImage,
};

#[derive(Resource)]
pub(crate) struct CloudsMaterialHandle(pub Handle<CloudsMaterial>);

pub struct CloudsPlugin;

impl Plugin for CloudsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CloudsConfig::default())
            .add_plugins((CloudsComputePlugin, CloudsShaderPlugin))
            .add_systems(Startup, clouds_setup)
            .add_systems(
                Update,
                update_clouds_resolution,
            )
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
    commands.insert_resource(CloudsMaterialHandle(material.clone()));
    init_skybox_mesh(
        &mut commands,
        meshes,
        SkyboxMaterials::from_one_material(MeshMaterial3d(material)),
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

fn update_clouds_resolution(
    mut commands: Commands,
    images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<CloudsMaterial>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut config: ResMut<CloudsConfig>,
    material_handle: Option<Res<CloudsMaterialHandle>>,
    clouds_image: Option<Res<CloudsImage>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    if window.width() < 50.0 || window.height() < 50.0 {
        return;
    }

    let max_scale = (RENDER_WIDTH as f32 / window.width())
        .min(RENDER_HEIGHT as f32 / window.height())
        .min(1.0);
    let total_scale = max_scale * config.render_scale;
    let target_width = (window.width() * total_scale)
        .round()
        .clamp(240.0, RENDER_WIDTH as f32) as u32;
    let target_height = (window.height() * total_scale)
        .round()
        .clamp(135.0, RENDER_HEIGHT as f32) as u32;

    let current = config.render_resolution;
    if (target_width as f32 - current.x).abs() <= 8.0
        && (target_height as f32 - current.y).abs() <= 8.0
    {
        return;
    }

    let Some(clouds_image) = clouds_image else {
        return;
    };

    let (cloud_render_image, sky_image) =
        build_render_images_with_size(images, target_width, target_height);

    commands.insert_resource(CloudsImage {
        cloud_render_image: cloud_render_image.clone(),
        cloud_atlas_image: clouds_image.cloud_atlas_image.clone(),
        cloud_worley_image: clouds_image.cloud_worley_image.clone(),
        sky_image: sky_image.clone(),
    });
    if let Some(handle) = material_handle {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.cloud_render_image = cloud_render_image;
            material.sky_image = sky_image;
        }
    }
    config.render_resolution = Vec2::new(target_width as f32, target_height as f32);
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