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
    if let Some(mut stud_image) = images.get_mut(&assets.stud) {
        if !matches!(stud_image.sampler, bevy::image::ImageSampler::Descriptor(_)) {
            stud_image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                ..default()
            });
        }
    }
    if let Some(mut stud_ambient_image) = images.get_mut(&assets.stud_ambient) {
        if !matches!(stud_ambient_image.sampler, bevy::image::ImageSampler::Descriptor(_)) {
            stud_ambient_image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                ..default()
            });
        }
    }
    if let Some(mut stud_height_image) = images.get_mut(&assets.stud_height) {
        if !matches!(stud_height_image.sampler, bevy::image::ImageSampler::Descriptor(_)) {
            stud_height_image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                ..default()
            });
        }
    }
    if let Some(mut inlet_ambient_image) = images.get_mut(&assets.inlet_ambient) {
        if !matches!(inlet_ambient_image.sampler, bevy::image::ImageSampler::Descriptor(_)) {
            inlet_ambient_image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                ..default()
            });
        }
    }
    if let Some(mut inlet_height_image) = images.get_mut(&assets.inlet_height) {
        if !matches!(inlet_height_image.sampler, bevy::image::ImageSampler::Descriptor(_)) {
            inlet_height_image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                ..default()
            });
        }
    }
    if let Some(mut inlet_image) = images.get_mut(&assets.inlet) {
        if !matches!(inlet_image.sampler, bevy::image::ImageSampler::Descriptor(_)) {
            inlet_image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                ..default()
            });
        }
    }
    *configured = true;
}

pub trait MapSamplers {}