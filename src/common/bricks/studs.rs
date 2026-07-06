use bevy::prelude::*;
use bevy::pbr::MaterialExtension;
use bevy::shader::ShaderRef;
use bevy::render::render_resource::AsBindGroup;

#[derive(Resource)]
pub struct StudsAssets {
    pub stud: Handle<Image>,
    pub inlet: Handle<Image>,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct StudsExtension {
    #[texture(100)]
    #[sampler(101)]
    pub stud_texture: Handle<Image>,
    #[texture(102)]
    #[sampler(103)]
    pub inlet_texture: Handle<Image>,
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
    let stud = asset_server.load("content/game/studs/stud.png");
    let inlet = asset_server.load("content/game/studs/inlet.png");
    commands.insert_resource(StudsAssets { stud, inlet });
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