use bevy::prelude::*;
use bevy::pbr::MaterialExtension;
use bevy::shader::ShaderRef;
use bevy::render::render_resource::AsBindGroup;

#[derive(Resource)]
pub struct StudsAssets {
    pub stud: Handle<Image>,
    pub stud_ambient: Handle<Image>,
    pub stud_height: Handle<Image>,
    pub inlet: Handle<Image>,
    pub inlet_ambient: Handle<Image>,
    pub inlet_height: Handle<Image>,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct StudsExtension {
    #[texture(100)]
    #[sampler(101)]
    pub stud_texture: Handle<Image>,
    #[texture(102)]
    #[sampler(103)]
    pub inlet_texture: Handle<Image>,
    #[texture(104)]
    #[sampler(105)]
    pub stud_ambient_texture: Handle<Image>,
    #[texture(106)]
    #[sampler(107)]
    pub stud_height_texture: Handle<Image>,
    #[texture(108)]
    #[sampler(109)]
    pub inlet_ambient_texture: Handle<Image>,
    #[texture(110)]
    #[sampler(111)]
    pub inlet_height_texture: Handle<Image>,
}

impl MapSamplers for StudsExtension {
}

impl MaterialExtension for StudsExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/studs.wgsl".into()
    }
}

pub fn setup_studs(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let stud = asset_server.load("content/game/studs/stud_normal.png");
    let stud_ambient = asset_server.load("content/game/studs/stud_ambient.png");
    let stud_height = asset_server.load("content/game/studs/stud_heightmap.png");
    let inlet = asset_server.load("content/game/studs/inlet.png");
    let inlet_ambient = asset_server.load("content/game/studs/inlet_ambient2.png");
    let inlet_height = asset_server.load("content/game/studs/inlet_height.png");
    commands.insert_resource(StudsAssets { stud, stud_ambient, stud_height, inlet, inlet_ambient, inlet_height });
}

pub fn configure_studs_samplers(
    stud_assets: Option<Res<StudsAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut configured: Local<bool>,
) {
    if *configured {
        return;
    }
    let Some(assets) = stud_assets else { return };
    let mut ready = true;
    ready &= configure_studs_image(&assets.stud, &mut images);
    ready &= configure_studs_image(&assets.stud_ambient, &mut images);
    ready &= configure_studs_image(&assets.stud_height, &mut images);
    ready &= configure_studs_image(&assets.inlet_ambient, &mut images);
    ready &= configure_studs_image(&assets.inlet_height, &mut images);
    ready &= configure_studs_image(&assets.inlet, &mut images);
    if ready {
        *configured = true;
    }
}

fn configure_studs_image(handle: &Handle<Image>, images: &mut Assets<Image>) -> bool {
    let Some(mut image) = images.get_mut(handle) else {
        return false;
    };
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        mipmap_filter: bevy::image::ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    });
    generate_mipmaps(&mut image);
    true
}

fn generate_mipmaps(image: &mut Image) {
    if image.texture_descriptor.mip_level_count > 1 {
        return;
    }
    let Some(mut data) = image.data.take() else {
        return;
    };
    let Some(bytes_per_pixel) = image.texture_descriptor.format.block_copy_size(None) else {
        return;
    };
    let mut width = image.texture_descriptor.size.width;
    let mut height = image.texture_descriptor.size.height;
    let mut mip_data = Vec::with_capacity(data.len());
    mip_data.extend_from_slice(&data);
    let mut mip_levels = 1u32;
    while width > 1 || height > 1 {
        let next_width = (width / 2).max(1);
        let next_height = (height / 2).max(1);
        let mut next_data = vec![0u8; (next_width * next_height) as usize * bytes_per_pixel as usize];
        for y in 0..next_height {
            for x in 0..next_width {
                for byte in 0..bytes_per_pixel as usize {
                    let mut sum = 0u32;
                    let mut count = 0u32;
                    for dy in 0..2u32 {
                        for dx in 0..2u32 {
                            let sample_x = (x * 2 + dx).min(width - 1);
                            let sample_y = (y * 2 + dy).min(height - 1);
                            sum += data[((sample_y * width + sample_x) as usize) * bytes_per_pixel as usize + byte] as u32;
                            count += 1;
                        }
                    }
                    next_data[((y * next_width + x) as usize) * bytes_per_pixel as usize + byte] = (sum / count) as u8;
                }
            }
        }
        mip_data.extend_from_slice(&next_data);
        data = next_data;
        width = next_width;
        height = next_height;
        mip_levels += 1;
    }
    image.data = Some(mip_data);
    image.texture_descriptor.mip_level_count = mip_levels;
}

pub trait MapSamplers {}