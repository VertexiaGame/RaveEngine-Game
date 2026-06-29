use bevy::prelude::*;
use bevy::image::{ImageType, CompressedImageFormats, ImageSampler};
use bevy::asset::RenderAssetUsages;

#[derive(Resource)]
pub struct StudioUiAssets {
    pub move_icon: Handle<Image>,
    pub rotate_icon: Handle<Image>,
    pub scale_icon: Handle<Image>,
    pub add_icon: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct StudioUiTextureIds {
    pub move_tex: Option<bevy_egui::egui::TextureId>,
    pub rotate_tex: Option<bevy_egui::egui::TextureId>,
    pub scale_tex: Option<bevy_egui::egui::TextureId>,
    pub add_tex: Option<bevy_egui::egui::TextureId>,
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

    let final_image = image.unwrap_or_else(|| Image::default());
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

    commands.insert_resource(StudioUiAssets {
        move_icon,
        rotate_icon,
        scale_icon,
        add_icon,
    });
}