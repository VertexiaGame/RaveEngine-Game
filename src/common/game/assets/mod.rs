pub mod components;
pub mod fetch;

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy::pbr::StandardMaterial;
use bevy::asset::RenderAssetUsages;
use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor,
    ImageType,
};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::TextureFormat;
use bevy::log::{info, warn};
use crate::common::game::bricks::components::{Brick, BrickShape, BrickShapeComponent};
use components::ImageFace;

const FACE_INSET: f32 = 0.04 * 0.28;
const FACE_OFFSET: f32 = 0.001;
const OVERLAY_THICKNESS: f32 = 0.001;
const BLOCK_MESH_HALF_EXTENTS: Vec3 = Vec3::new(2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28);

#[derive(Resource, Default)]
pub struct ImageAssetCache {
    pub images: HashMap<u32, Handle<Image>>,
    pub materials: HashMap<u32, Handle<StandardMaterial>>,
    pub plane_mesh: Option<Handle<Mesh>>,
}

#[derive(Resource, Default)]
pub struct PendingImageFetches(pub HashSet<u32>);

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<components::Image>()
            .register_type::<components::ImageFace>()
            .init_resource::<ImageAssetCache>()
            .init_resource::<PendingImageFetches>()
            .init_resource::<fetch::AssetFetchPool>();

        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.add_systems(Update, (
                setup_image_meshes,
                fetch_missing_images,
                apply_fetched_images,
                apply_cached_image_materials,
                update_image_visuals,
            ));
        }
    }
}

fn plane_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, -0.5, 0.0],
            [0.5, -0.5, 0.0],
            [0.5, 0.5, 0.0],
            [-0.5, 0.5, 0.0],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

fn setup_image_meshes(
    mut commands: Commands,
    mut cache: ResMut<ImageAssetCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Query<Entity, (With<components::Image>, Without<Mesh3d>)>,
) {
    if cache.plane_mesh.is_none() {
        cache.plane_mesh = Some(meshes.add(plane_mesh()));
    }
    if !cache.materials.contains_key(&0) {
        cache.materials.insert(0, materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            perceptual_roughness: 1.0,
            double_sided: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }));
    }
    let Some(handle) = cache.plane_mesh.clone() else {
        return;
    };
    for entity in &images {
        commands.entity(entity).insert(Mesh3d(handle.clone()));
    }
}

fn fetch_missing_images(
    images: Query<&components::Image>,
    cache: Res<ImageAssetCache>,
    mut pending: ResMut<PendingImageFetches>,
    mut pool: ResMut<fetch::AssetFetchPool>,
) {
    for image in &images {
        let asset_id = image.asset_id;
        if asset_id == 0 || cache.images.contains_key(&asset_id) || pending.0.contains(&asset_id) {
            continue;
        }
        pool.ensure_started();
        if pool.submit(asset_id) {
            pending.0.insert(asset_id);
        }
    }
}

fn apply_fetched_images(
    mut commands: Commands,
    pool: Res<fetch::AssetFetchPool>,
    mut pending: ResMut<PendingImageFetches>,
    mut cache: ResMut<ImageAssetCache>,
    mut image_assets: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Query<(Entity, &components::Image)>,
) {
    for (asset_id, result) in pool.drain_results() {
        pending.0.remove(&asset_id);
        match result {
            Ok(bytes) => {
                let Some(image_handle) = decode_image_asset(&bytes, &mut image_assets) else {
                    warn!("Asset {asset_id}: failed to decode image bytes");
                    continue;
                };
                let material = materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(image_handle.clone()),
                    perceptual_roughness: 1.0,
                    double_sided: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });
                info!("Asset {asset_id}: loaded image");
                cache.images.insert(asset_id, image_handle);
                cache.materials.insert(asset_id, material.clone());
                for (entity, image) in &images {
                    if image.asset_id == asset_id {
                        commands.entity(entity).insert(MeshMaterial3d(material.clone()));
                    }
                }
            }
            Err(e) => warn!("Asset {asset_id}: fetch failed: {e}"),
        }
    }
}

fn apply_cached_image_materials(
    mut commands: Commands,
    cache: Res<ImageAssetCache>,
    images: Query<(Entity, &components::Image), (With<Mesh3d>, Without<MeshMaterial3d<StandardMaterial>>)>,
) {
    for (entity, image) in &images {
        if let Some(material) = cache.materials.get(&image.asset_id) {
            commands.entity(entity).insert(MeshMaterial3d(material.clone()));
        }
    }
}

fn decode_image_asset(bytes: &[u8], images: &mut Assets<Image>) -> Option<Handle<Image>> {
    let mut image = Image::from_buffer(
        bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::all(),
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .ok();
    if image.is_none() {
        image = Image::from_buffer(
            bytes,
            ImageType::Extension("jpg"),
            CompressedImageFormats::all(),
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .ok();
    }
    if image.is_none() {
        image = Image::from_buffer(
            bytes,
            ImageType::Extension("webp"),
            CompressedImageFormats::all(),
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .ok();
    }
    if image.is_none() {
        image = Image::from_buffer(
            bytes,
            ImageType::Extension("gif"),
            CompressedImageFormats::all(),
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .ok();
    }
    let mut final_image = image?;

    let format = final_image.texture_descriptor.format;
    if (format == TextureFormat::Rgba8UnormSrgb || format == TextureFormat::Rgba8Unorm)
        && let Some(ref mut data) = final_image.data
    {
        for chunk in data.chunks_exact_mut(4) {
            let a = chunk[3] as f32 / 255.0;
            chunk[0] = (chunk[0] as f32 * a) as u8;
            chunk[1] = (chunk[1] as f32 * a) as u8;
            chunk[2] = (chunk[2] as f32 * a) as u8;
        }
    }

    final_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });

    Some(images.add(final_image))
}

fn face_normal(face: ImageFace) -> Vec3 {
    match face {
        ImageFace::Top => Vec3::Y,
        ImageFace::Bottom => Vec3::NEG_Y,
        ImageFace::Left => Vec3::NEG_X,
        ImageFace::Right => Vec3::X,
        ImageFace::Front => Vec3::Z,
        ImageFace::Back => Vec3::NEG_Z,
    }
}

fn update_image_visuals(
    mut images: Query<(
        &components::Image,
        Option<&ChildOf>,
        &mut Transform,
        Option<&mut Visibility>,
    )>,
    parents: Query<&BrickShapeComponent, (With<Brick>, Without<components::Image>)>,
) {
    for (image, child_of, mut transform, mut visibility) in &mut images {
        let Some(child_of) = child_of else {
            if let Some(visibility) = visibility.as_deref_mut() {
                *visibility = Visibility::Visible;
            }
            continue;
        };
        let Ok(parent_shape) = parents.get(child_of.parent()) else {
            if let Some(visibility) = visibility.as_deref_mut() {
                *visibility = Visibility::Visible;
            }
            continue;
        };
        if parent_shape.shape != BrickShape::Block {
            if let Some(visibility) = visibility.as_deref_mut() {
                *visibility = Visibility::Hidden;
            }
            continue;
        }
        if let Some(visibility) = visibility.as_deref_mut() {
            *visibility = Visibility::Visible;
        }

        let normal = face_normal(image.face.unwrap_or(ImageFace::Front));
        let (half_extent, width, height) = if normal.x != 0.0 {
            (BLOCK_MESH_HALF_EXTENTS.x, BLOCK_MESH_HALF_EXTENTS.y * 2.0, BLOCK_MESH_HALF_EXTENTS.z * 2.0)
        } else if normal.y != 0.0 {
            (BLOCK_MESH_HALF_EXTENTS.y, BLOCK_MESH_HALF_EXTENTS.x * 2.0, BLOCK_MESH_HALF_EXTENTS.z * 2.0)
        } else {
            (BLOCK_MESH_HALF_EXTENTS.z, BLOCK_MESH_HALF_EXTENTS.x * 2.0, BLOCK_MESH_HALF_EXTENTS.y * 2.0)
        };
        transform.translation = normal * half_extent + normal * FACE_OFFSET;
        transform.rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        transform.scale = Vec3::new(
            (width - FACE_INSET * 2.0).max(0.01),
            (height - FACE_INSET * 2.0).max(0.01),
            OVERLAY_THICKNESS,
        );
    }
}