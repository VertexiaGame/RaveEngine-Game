use bevy::prelude::*;
use bevy::image::{ImageType, CompressedImageFormats, ImageSampler};
use bevy::asset::RenderAssetUsages;

#[derive(Resource)]
pub struct StudioUiAssets {
    pub move_icon: Handle<Image>,
    pub rotate_icon: Handle<Image>,
    pub scale_icon: Handle<Image>,
    pub add_icon: Handle<Image>,
    pub workspace_icon: Handle<Image>,
    pub brick_icon: Handle<Image>,
    pub players_icon: Handle<Image>,
    pub lighting_icon: Handle<Image>,
    pub thumb_empty: Handle<Image>,
    pub thumb_baseplate: Handle<Image>,
    pub play_icon: Handle<Image>,
    pub playc_icon: Handle<Image>,
    pub stopp_icon: Handle<Image>,
    pub script_icon: Handle<Image>,
    pub localscript_icon: Handle<Image>,
    pub modulescript_icon: Handle<Image>,
    pub image_icon: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct StudioUiTextureIds {
    pub move_tex: Option<bevy_egui::egui::TextureId>,
    pub rotate_tex: Option<bevy_egui::egui::TextureId>,
    pub scale_tex: Option<bevy_egui::egui::TextureId>,
    pub add_tex: Option<bevy_egui::egui::TextureId>,
    pub workspace_tex: Option<bevy_egui::egui::TextureId>,
    pub brick_tex: Option<bevy_egui::egui::TextureId>,
    pub players_tex: Option<bevy_egui::egui::TextureId>,
    pub lighting_tex: Option<bevy_egui::egui::TextureId>,
    pub thumb_empty_tex: Option<bevy_egui::egui::TextureId>,
    pub thumb_baseplate_tex: Option<bevy_egui::egui::TextureId>,
    pub play_tex: Option<bevy_egui::egui::TextureId>,
    pub playc_tex: Option<bevy_egui::egui::TextureId>,
    pub stopp_tex: Option<bevy_egui::egui::TextureId>,
    pub script_tex: Option<bevy_egui::egui::TextureId>,
    pub localscript_tex: Option<bevy_egui::egui::TextureId>,
    pub modulescript_tex: Option<bevy_egui::egui::TextureId>,
    pub image_tex: Option<bevy_egui::egui::TextureId>,
}

fn load_icon_image(path: &str, images: &mut Assets<Image>) -> Handle<Image> {
    let bytes = std::fs::read(path).unwrap_or_else(|_| {
        std::fs::read(format!("assets/{}", path)).unwrap_or_default()
    });
    if bytes.is_empty() {
        return Handle::default();
    }

    let mut image = Image::from_buffer(
        &bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::all(),
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    ).ok();

    if image.is_none() {
        image = Image::from_buffer(
            &bytes,
            ImageType::Extension("jpg"),
            CompressedImageFormats::all(),
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        ).ok();
    }

    let mut final_image = image.unwrap_or_else(|| Image::default());

    let format = final_image.texture_descriptor.format;
    if format == bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb 
        || format == bevy::render::render_resource::TextureFormat::Rgba8Unorm 
    {
        if let Some(ref mut data) = final_image.data {
            for chunk in data.chunks_exact_mut(4) {
                let a = chunk[3] as f32 / 255.0;
                chunk[0] = (chunk[0] as f32 * a) as u8;
                chunk[1] = (chunk[1] as f32 * a) as u8;
                chunk[2] = (chunk[2] as f32 * a) as u8;
            }
        }
    }

    final_image.sampler = ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
        address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });

    images.add(final_image)
}

pub fn setup_ui_assets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let move_icon = load_icon_image("content/studio/icons/Tools/Move.png", &mut images);
    let rotate_icon = load_icon_image("content/studio/icons/Tools/Rotate.png", &mut images);
    let scale_icon = load_icon_image("content/studio/icons/Tools/Scale.png", &mut images);
    let add_icon = load_icon_image("content/studio/icons/Tools/Add.png", &mut images);
    let workspace_icon = load_icon_image("content/studio/icons/Items/workspace.png", &mut images);
    let brick_icon = load_icon_image("content/studio/icons/Items/brick.png", &mut images);
    let players_icon = load_icon_image("content/studio/icons/Items/players.png", &mut images);
    let lighting_icon = load_icon_image("content/studio/icons/Items/lighting.png", &mut images);
    let thumb_empty = load_icon_image("content/studio/thumb/empty.png", &mut images);
    let thumb_baseplate = load_icon_image("content/studio/thumb/baseplate.png", &mut images);
    let play_icon = load_icon_image("content/studio/icons/Tools/play.png", &mut images);
    let playc_icon = load_icon_image("content/studio/icons/Tools/playc.png", &mut images);
    let stopp_icon = load_icon_image("content/studio/icons/Tools/stopp.png", &mut images);
    let script_icon = load_icon_image("content/studio/icons/Items/script.png", &mut images);
    let localscript_icon = load_icon_image("content/studio/icons/Items/localscript.png", &mut images);
    let mut modulescript_icon = load_icon_image("content/studio/icons/Items/modulescript.png", &mut images);
    if modulescript_icon == Handle::default() {
        modulescript_icon = script_icon.clone();
    }
    let image_icon = load_icon_image("content/studio/icons/Items/image.png", &mut images);

    commands.insert_resource(StudioUiAssets {
        move_icon,
        rotate_icon,
        scale_icon,
        add_icon,
        workspace_icon,
        brick_icon,
        players_icon,
        lighting_icon,
        thumb_empty,
        thumb_baseplate,
        play_icon,
        playc_icon,
        stopp_icon,
        script_icon,
        localscript_icon,
        modulescript_icon,
        image_icon,
    });
}