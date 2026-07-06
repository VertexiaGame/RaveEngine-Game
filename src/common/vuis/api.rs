use bevy::prelude::*;
use bevy::ui::{BackgroundGradient, LinearGradient, ColorStop, InterpolationColorSpace};
use base64::prelude::*;
use flate2::read::GzDecoder;
use std::io::Read;
use std::fs;
use crate::common::vuis::types::{VuisNode, VuisAnimationState, VuisFile, VuisDataNode};

#[derive(Resource, Default)]
pub struct VuisEngine;

impl VuisEngine {
    pub fn load(
        &self,
        commands: &mut Commands,
        images: &mut ResMut<Assets<Image>>,
        fonts: &mut ResMut<Assets<Font>>,
        parent: Entity,
        file_path: &str,
    ) -> Result<Entity, String> {
        info!("VUIS: Attempting to load file from path: {}", file_path);
        let compressed_data = fs::read(file_path)
            .map_err(|e| format!("Failed to read VUIS file from {}: {}", file_path, e))?;

        debug!("VUIS: Successfully read {} bytes of compressed data", compressed_data.len());
        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut json_string = String::new();
        decoder.read_to_string(&mut json_string)
            .map_err(|e| format!("Failed to decompress VUIS file: {}", e))?;

        debug!("VUIS: Successfully decompressed into {} bytes of JSON", json_string.len());
        let file_data = serde_json::from_str::<VuisFile>(&json_string)
            .map_err(|e| format!("Failed to parse VUIS JSON structure: {}", e))?;

        info!("VUIS: Parsed file version: {}, spawning root node tree", file_data.Version);
        let root_entity = commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(1920.0),
                height: Val::Px(1080.0),
                ..default()
            },
            UiTransform::IDENTITY,
            crate::common::vuis::types::VuisRootContainer {
                design_width: 1920.0,
                design_height: 1080.0,
            },
        )).id();

        commands.entity(parent).add_child(root_entity);

        for child_data in &file_data.Root.Children {
            self.spawn_data_tree(commands, images, fonts, root_entity, child_data);
        }

        Ok(root_entity)
    }

    pub fn add_element(
        &self,
        commands: &mut Commands,
        parent: Entity,
        node: VuisNode,
    ) -> Entity {
        info!(
            "VUIS: API call to add element dynamically under parent: {:?}. ID: {}, Pos: ({}, {}), Size: ({}x{})",
            parent, node.Id, node.PositionX, node.PositionY, node.WidthPx, node.HeightPx
        );
        let entity = commands.spawn((
            node.clone(),
            VuisAnimationState::default(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(node.PositionX),
                top: Val::Px(node.PositionY),
                width: if node.WidthPx <= 0.0 { Val::Auto } else { Val::Px(node.WidthPx) },
                height: if node.HeightPx <= 0.0 { Val::Auto } else { Val::Px(node.HeightPx) },
                border_radius: BorderRadius::all(Val::Px(node.BorderRadiusPx)),
                overflow: if node.IsScrollable { Overflow::scroll_y() } else { Overflow::visible() },
                ..default()
            },
            BackgroundColor(node.BackgroundColor),
            UiTransform::IDENTITY,
        )).id();

        if node.IsHidden {
            commands.entity(entity).insert(Visibility::Hidden);
        }

        if node.HasShadow {
            commands.entity(entity).insert(BoxShadow::new(
                node.ShadowColor,
                Val::Px(node.ShadowOffsetX),
                Val::Px(node.ShadowOffsetY),
                Val::Px(node.ShadowSpread),
                Val::Px(node.ShadowBlur),
            ));
        }

        if node.IsGradient {
            commands.entity(entity).insert(BackgroundGradient::from(LinearGradient {
                color_space: InterpolationColorSpace::Oklaba,
                angle: 0.0,
                stops: vec![
                    ColorStop::percent(node.GradientColor1, 0.0),
                    ColorStop::percent(node.GradientColor2, 100.0),
                ],
            }));
        }

        if node.BorderWidthPx > 0.0 {
            commands.entity(entity).insert((
                Node {
                    border: UiRect::all(Val::Px(node.BorderWidthPx)),
                    ..default()
                },
                BorderColor::all(node.BorderColor),
            ));
        }

        if node.HasText {
            let mut text_commands = commands.spawn((
                Text::new(""),
                TextColor(node.TextColor),
                TextFont {
                    font_size: FontSize::Px(node.FontSizePx),
                    ..default()
                },
            ));
            if node.HasShadow {
                text_commands.insert(TextShadow {
                    offset: Vec2::new(node.ShadowOffsetX, node.ShadowOffsetY),
                    color: node.ShadowColor,
                });
            }
            let text_entity = text_commands.id();
            commands.entity(entity).add_child(text_entity);
        }

        commands.entity(parent).add_child(entity);
        entity
    }

    pub fn edit_element(
        &self,
        commands: &mut Commands,
        entity: Entity,
        edit_fn: impl FnOnce(&mut VuisNode) + Send + 'static,
    ) -> bool {
        info!("VUIS: API call to edit element: {:?}", entity);
        commands.queue(move |world: &mut World| {
            if let Some(mut component) = world.get_mut::<VuisNode>(entity) {
                edit_fn(&mut component);
            }
        });
        true
    }

    fn spawn_data_tree(
        &self,
        commands: &mut Commands,
        images: &mut ResMut<Assets<Image>>,
        fonts: &mut ResMut<Assets<Font>>,
        parent_entity: Entity,
        data: &VuisDataNode,
    ) {
        info!(
            "VUIS: Spawning data tree node: {} under parent: {:?}. Pos: ({}, {}), Size: ({}x{}), HasText: {}, TextContent: {:?}",
            data.Id, parent_entity, data.PositionX, data.PositionY, data.WidthPx, data.HeightPx, data.HasText, data.TextContent
        );
        let mut image_data = Option::None;
        let mut image_handle = Option::None;

        if let Some(base64_img) = &data.Base64Image {
            if let Ok(decoded) = BASE64_STANDARD.decode(base64_img) {
                if let Some(loaded_image) = crate::common::vuis::types::load_image_from_bytes(&decoded) {
                    image_handle = Some(images.add(loaded_image));
                    image_data = Some(decoded);
                }
            }
        }

        let mut font_data = Option::None;
        let mut font_handle = Option::None;

        if let Some(base64_fnt) = &data.Base64Font {
            if let Ok(decoded) = BASE64_STANDARD.decode(base64_fnt) {
                let loaded_font = Font::from_bytes(decoded.clone());
                font_handle = Some(fonts.add(loaded_font));
                font_data = Some(decoded);
            }
        }

        let node = VuisNode {
            Id: data.Id.clone(),
            BackgroundColor: Color::LinearRgba(LinearRgba {
                red: data.ColorRgba[0],
                green: data.ColorRgba[1],
                blue: data.ColorRgba[2],
                alpha: data.ColorRgba[3],
            }),
            TextColor: if let Some(tc) = data.TextColorRgba {
                Color::LinearRgba(LinearRgba {
                    red: tc[0],
                    green: tc[1],
                    blue: tc[2],
                    alpha: tc[3],
                })
            } else {
                Color::LinearRgba(LinearRgba { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 })
            },
            FontFamily: data.FontFamily.clone().unwrap_or_default(),
            FontSizePx: data.FontSizePx.unwrap_or(16.0),
            WidthPx: data.WidthPx,
            HeightPx: data.HeightPx,
            IsImage: data.IsImage,
            ImageData: image_data,
            HasText: data.HasText,
            FontData: font_data,
            AnimTargetWidth: data.AnimTargetWidth,
            AnimTargetHeight: data.AnimTargetHeight,
            AnimTargetX: data.AnimTargetX.unwrap_or(data.PositionX),
            AnimTargetY: data.AnimTargetY.unwrap_or(data.PositionY),
            AnimTargetRotation: data.AnimTargetRotation.unwrap_or(data.Rotation),
            AnimDuration: data.AnimDuration,
            PositionX: data.PositionX,
            PositionY: data.PositionY,
            Rotation: data.Rotation,
            BorderRadiusPx: data.BorderRadiusPx,
            BorderWidthPx: data.BorderWidthPx,
            BorderColor: Color::LinearRgba(LinearRgba {
                red: data.BorderColorRgba[0],
                green: data.BorderColorRgba[1],
                blue: data.BorderColorRgba[2],
                alpha: data.BorderColorRgba[3],
            }),
            IsGradient: data.IsGradient,
            GradientColor1: Color::LinearRgba(LinearRgba {
                red: data.GradientColor1Rgba[0],
                green: data.GradientColor1Rgba[1],
                blue: data.GradientColor1Rgba[2],
                alpha: data.GradientColor1Rgba[3],
            }),
            GradientColor2: Color::LinearRgba(LinearRgba {
                red: data.GradientColor2Rgba[0],
                green: data.GradientColor2Rgba[1],
                blue: data.GradientColor2Rgba[2],
                alpha: data.GradientColor2Rgba[3],
            }),
            IsInput: data.IsInput,
            IsHidden: data.IsHidden,
            IsBold: data.IsBold,
            IsItalic: data.IsItalic,
            Placeholder: data.Placeholder.clone(),
            HasShadow: data.HasShadow.unwrap_or(false),
            ShadowColor: if let Some(sc) = data.ShadowColorRgba {
                Color::LinearRgba(LinearRgba {
                    red: sc[0],
                    green: sc[1],
                    blue: sc[2],
                    alpha: sc[3],
                })
            } else {
                Color::LinearRgba(LinearRgba { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.5 })
            },
            ShadowOffsetX: data.ShadowOffsetX.unwrap_or(4.0),
            ShadowOffsetY: data.ShadowOffsetY.unwrap_or(4.0),
            ShadowBlur: data.ShadowBlur.unwrap_or(10.0),
            ShadowSpread: data.ShadowSpread.unwrap_or(0.0),
            IsGrid: data.IsGrid.unwrap_or(false),
            GridColumns: data.GridColumns.unwrap_or(2),
            GridRows: data.GridRows.unwrap_or(2),
            GridColumnGap: data.GridColumnGap.unwrap_or(0.0),
            GridRowGap: data.GridRowGap.unwrap_or(0.0),
            LayoutFlow: data.LayoutFlow.clone().unwrap_or_else(|| "None".to_string()),
            IsScrollable: data.IsScrollable.unwrap_or(false),
            ScrollbarWidth: data.ScrollbarWidth.unwrap_or(8.0),
            ScrollbarColor: if let Some(sc) = data.ScrollbarColorRgba {
                Color::LinearRgba(LinearRgba {
                    red: sc[0],
                    green: sc[1],
                    blue: sc[2],
                    alpha: sc[3],
                })
            } else {
                Color::LinearRgba(LinearRgba { red: 0.5, green: 0.5, blue: 0.5, alpha: 0.8 })
            },
            ScrollbarTrackColor: if let Some(stc) = data.ScrollbarTrackColorRgba {
                Color::LinearRgba(LinearRgba {
                    red: stc[0],
                    green: stc[1],
                    blue: stc[2],
                    alpha: stc[3],
                })
            } else {
                Color::LinearRgba(LinearRgba { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.2 })
            },
            ScrollbarBorderRadius: data.ScrollbarBorderRadius.unwrap_or(4.0),
        };

        let mut entity_commands = commands.spawn((
            node.clone(),
            VuisAnimationState::default(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(data.PositionX),
                top: Val::Px(data.PositionY),
                width: if node.WidthPx <= 0.0 { Val::Auto } else { Val::Px(node.WidthPx) },
                height: if node.HeightPx <= 0.0 { Val::Auto } else { Val::Px(node.HeightPx) },
                border: UiRect::all(Val::Px(data.BorderWidthPx)),
                border_radius: BorderRadius::all(Val::Px(data.BorderRadiusPx)),
                align_items: if data.HasText { AlignItems::Center } else { AlignItems::default() },
                justify_content: if data.HasText { JustifyContent::Center } else { JustifyContent::default() },
                display: if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { Display::Grid } else { Display::Flex },
                grid_template_columns: if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { vec![RepeatedGridTrack::flex(node.GridColumns as u16, 1.0)] } else { Vec::new() },
                grid_template_rows: if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { vec![RepeatedGridTrack::flex(node.GridRows as u16, 1.0)] } else { Vec::new() },
                column_gap: if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { Val::Px(node.GridColumnGap) } else { Val::Auto },
                row_gap: if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { Val::Px(node.GridRowGap) } else { Val::Auto },
                overflow: if node.IsScrollable { Overflow::scroll_y() } else { Overflow::visible() },
                scrollbar_width: if node.IsScrollable { node.ScrollbarWidth } else { 0.0 },
                ..default()
            },
            BackgroundColor(node.BackgroundColor),
            UiTransform {
                rotation: Rot2::radians(-data.Rotation),
                ..default()
            },
        ));

        if node.IsScrollable {
            entity_commands.insert(ScrollPosition::default());
        }

        if node.IsHidden {
            entity_commands.insert(Visibility::Hidden);
        }

        if let Some(handle) = image_handle {
            entity_commands.insert(ImageNode::new(handle));
        }

        if node.HasShadow {
            entity_commands.insert(BoxShadow::new(
                node.ShadowColor,
                Val::Px(node.ShadowOffsetX),
                Val::Px(node.ShadowOffsetY),
                Val::Px(node.ShadowSpread),
                Val::Px(node.ShadowBlur),
            ));
        }

        if node.IsGradient {
            entity_commands.insert(BackgroundGradient::from(LinearGradient {
                color_space: InterpolationColorSpace::Oklaba,
                angle: 0.0,
                stops: vec![
                    ColorStop::percent(node.GradientColor1, 0.0),
                    ColorStop::percent(node.GradientColor2, 100.0),
                ],
            }));
        }

        if node.BorderWidthPx > 0.0 {
            entity_commands.insert(BorderColor::all(node.BorderColor));
        }

        let spawned_entity = entity_commands.id();

        if node.HasText {
            let mut text_commands = commands.spawn((
                Text::new(data.TextContent.clone().unwrap_or_default()),
                TextColor(node.TextColor),
            ));

            if let Some(handle) = font_handle {
                text_commands.insert(TextFont { font: FontSource::Handle(handle), font_size: FontSize::Px(node.FontSizePx), ..default() });
            } else {
                text_commands.insert(TextFont { font_size: FontSize::Px(node.FontSizePx), ..default() });
            }

            if node.IsInput {
                text_commands.insert(bevy::text::EditableText::default());
            }

            if node.HasShadow {
                text_commands.insert(TextShadow {
                    offset: Vec2::new(node.ShadowOffsetX, node.ShadowOffsetY),
                    color: node.ShadowColor,
                });
            }

            let text_entity = text_commands.id();
            commands.entity(spawned_entity).add_child(text_entity);
        }

        commands.entity(parent_entity).add_child(spawned_entity);

        for child_data in &data.Children {
            self.spawn_data_tree(commands, images, fonts, spawned_entity, child_data);
        }
    }
}