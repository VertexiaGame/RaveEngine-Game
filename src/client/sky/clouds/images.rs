use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

pub const IMAGE_SIZE: u32 = 1440;

pub const RENDER_WIDTH: u32 = 1440;
pub const RENDER_HEIGHT: u32 = 810;

pub fn build_images(
    images: ResMut<Assets<Image>>,
) -> (Handle<Image>, Handle<Image>, Handle<Image>, Handle<Image>) {
    build_images_with_size(images, RENDER_WIDTH, RENDER_HEIGHT)
}

pub fn build_render_images_with_size(
    mut images: ResMut<Assets<Image>>,
    width: u32,
    height: u32,
) -> (Handle<Image>, Handle<Image>) {
    let mut cloud_render_image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4 * 4 * 2],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    cloud_render_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    let mut sky_image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4 * 4 * 2],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    sky_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    (images.add(cloud_render_image), images.add(sky_image))
}

pub fn build_images_with_size(
    mut images: ResMut<Assets<Image>>,
    width: u32,
    height: u32,
) -> (Handle<Image>, Handle<Image>, Handle<Image>, Handle<Image>) {
    let mut cloud_render_image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4 * 4 * 2],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    cloud_render_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    let mut cloud_atlas_image = Image::new_fill(
        Extent3d {
            width: IMAGE_SIZE,
            height: IMAGE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4 * 4 * 2],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    cloud_atlas_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    let mut cloud_worley_image = Image::new_fill(
        Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 32,
        },
        TextureDimension::D3,
        &[0; 4 * 4 * 2],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    cloud_worley_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    let mut sky_image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4 * 4 * 2],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    sky_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    (
        images.add(cloud_render_image),
        images.add(cloud_atlas_image),
        images.add(cloud_worley_image),
        images.add(sky_image),
    )
}