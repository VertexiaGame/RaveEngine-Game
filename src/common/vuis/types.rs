use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Component, Clone, Debug)]
pub struct VuisNode {
    pub Id: String,
    pub BackgroundColor: Color,
    pub TextColor: Color,
    pub FontFamily: String,
    pub FontSizePx: f32,
    pub WidthPx: f32,
    pub HeightPx: f32,
    pub IsImage: bool,
    pub ImageData: Option<Vec<u8>>,
    pub HasText: bool,
    pub FontData: Option<Vec<u8>>,
    pub AnimTargetWidth: f32,
    pub AnimTargetHeight: f32,
    pub AnimTargetX: f32,
    pub AnimTargetY: f32,
    pub AnimTargetRotation: f32,
    pub AnimDuration: f32,
    pub PositionX: f32,
    pub PositionY: f32,
    pub Rotation: f32,
    pub BorderRadiusPx: f32,
    pub BorderWidthPx: f32,
    pub BorderColor: Color,
    pub IsGradient: bool,
    pub GradientColor1: Color,
    pub GradientColor2: Color,
    pub IsInput: bool,
    pub IsHidden: bool,
    pub IsBold: bool,
    pub IsItalic: bool,
    pub Placeholder: String,
    pub HasShadow: bool,
    pub ShadowColor: Color,
    pub ShadowOffsetX: f32,
    pub ShadowOffsetY: f32,
    pub ShadowBlur: f32,
    pub ShadowSpread: f32,
    pub IsGrid: bool,
    pub GridColumns: u32,
    pub GridRows: u32,
    pub GridColumnGap: f32,
    pub GridRowGap: f32,
    pub LayoutFlow: String,
    pub IsScrollable: bool,
    pub ScrollbarWidth: f32,
    pub ScrollbarColor: Color,
    pub ScrollbarTrackColor: Color,
    pub ScrollbarBorderRadius: f32,
}

impl Default for VuisNode {
    fn default() -> Self {
        Self {
            Id: "Node".to_string(),
            BackgroundColor: Color::LinearRgba(LinearRgba { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 }),
            TextColor: Color::LinearRgba(LinearRgba { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 }),
            FontFamily: "".to_string(),
            FontSizePx: 16.0,
            WidthPx: 100.0,
            HeightPx: 100.0,
            IsImage: false,
            ImageData: None,
            HasText: false,
            FontData: None,
            AnimTargetWidth: 100.0,
            AnimTargetHeight: 100.0,
            AnimTargetX: 0.0,
            AnimTargetY: 0.0,
            AnimTargetRotation: 0.0,
            AnimDuration: 0.0,
            PositionX: 0.0,
            PositionY: 0.0,
            Rotation: 0.0,
            BorderRadiusPx: 0.0,
            BorderWidthPx: 0.0,
            BorderColor: Color::LinearRgba(LinearRgba { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 }),
            IsGradient: false,
            GradientColor1: Color::LinearRgba(LinearRgba { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 }),
            GradientColor2: Color::LinearRgba(LinearRgba { red: 0.0, green: 0.0, blue: 0.0, alpha: 1.0 }),
            IsInput: false,
            IsHidden: false,
            IsBold: false,
            IsItalic: false,
            Placeholder: "".to_string(),
            HasShadow: false,
            ShadowColor: Color::LinearRgba(LinearRgba { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.5 }),
            ShadowOffsetX: 4.0,
            ShadowOffsetY: 4.0,
            ShadowBlur: 10.0,
            ShadowSpread: 0.0,
            IsGrid: false,
            GridColumns: 2,
            GridRows: 2,
            GridColumnGap: 0.0,
            GridRowGap: 0.0,
            LayoutFlow: "None".to_string(),
            IsScrollable: false,
            ScrollbarWidth: 8.0,
            ScrollbarColor: Color::LinearRgba(LinearRgba { red: 0.5, green: 0.5, blue: 0.5, alpha: 0.8 }),
            ScrollbarTrackColor: Color::LinearRgba(LinearRgba { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.2 }),
            ScrollbarBorderRadius: 4.0,
        }
    }
}

#[derive(Component, Clone, Default, Debug)]
pub struct VuisAnimationState {
    pub Timer: f32,
    pub Forward: bool,
    pub IsPlaying: bool,
}

#[derive(Component, Clone, Debug)]
pub struct PlaceholderTextComponent(pub Entity);

#[derive(Component, Clone, Debug)]
pub struct VuisRootContainer {
    pub design_width: f32,
    pub design_height: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VuisFile {
    pub Version: String,
    pub Root: VuisDataNode,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VuisDataNode {
    pub Id: String,
    pub ColorRgba: [f32; 4],
    pub TextColorRgba: Option<[f32; 4]>,
    pub FontFamily: Option<String>,
    pub FontSizePx: Option<f32>,
    pub WidthPx: f32,
    pub HeightPx: f32,
    pub IsImage: bool,
    pub Base64Image: Option<String>,
    pub HasText: bool,
    pub TextContent: Option<String>,
    pub Base64Font: Option<String>,
    pub AnimTargetWidth: f32,
    pub AnimTargetHeight: f32,
    pub AnimTargetX: Option<f32>,
    pub AnimTargetY: Option<f32>,
    pub AnimTargetRotation: Option<f32>,
    pub AnimDuration: f32,
    pub PositionX: f32,
    pub PositionY: f32,
    pub Rotation: f32,
    pub BorderRadiusPx: f32,
    pub BorderWidthPx: f32,
    pub BorderColorRgba: [f32; 4],
    pub IsGradient: bool,
    pub GradientColor1Rgba: [f32; 4],
    pub GradientColor2Rgba: [f32; 4],
    pub IsInput: bool,
    pub IsHidden: bool,
    pub IsBold: bool,
    pub IsItalic: bool,
    pub Placeholder: String,
    pub HasShadow: Option<bool>,
    pub ShadowColorRgba: Option<[f32; 4]>,
    pub ShadowOffsetX: Option<f32>,
    pub ShadowOffsetY: Option<f32>,
    pub ShadowBlur: Option<f32>,
    pub ShadowSpread: Option<f32>,
    pub IsGrid: Option<bool>,
    pub GridColumns: Option<u32>,
    pub GridRows: Option<u32>,
    pub GridColumnGap: Option<f32>,
    pub GridRowGap: Option<f32>,
    pub LayoutFlow: Option<String>,
    pub IsScrollable: Option<bool>,
    pub ScrollbarWidth: Option<f32>,
    pub ScrollbarColorRgba: Option<[f32; 4]>,
    pub ScrollbarTrackColorRgba: Option<[f32; 4]>,
    pub ScrollbarBorderRadius: Option<f32>,
    pub Children: Vec<VuisDataNode>,
}

pub fn load_image_from_bytes(bytes: &[u8]) -> Option<Image> {
    let mut image = Image::from_buffer(
        bytes,
        bevy::image::ImageType::Extension("png"),
        bevy::image::CompressedImageFormats::all(),
        true,
        bevy::image::ImageSampler::Default,
        bevy::asset::RenderAssetUsages::default(),
    ).ok();

    if image.is_none() {
        image = Image::from_buffer(
            bytes,
            bevy::image::ImageType::Extension("jpg"),
            bevy::image::CompressedImageFormats::all(),
            true,
            bevy::image::ImageSampler::Default,
            bevy::asset::RenderAssetUsages::default(),
        ).ok();
    }

    image
}