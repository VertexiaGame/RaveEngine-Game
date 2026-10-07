use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

pub fn configure_client_visuals(mut contexts: EguiContexts, mut initialized: Local<bool>) {
    if *initialized {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut visuals = egui::Visuals::dark();
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgba_unmultiplied(61, 61, 61, 102);
    visuals.window_fill = egui::Color32::TRANSPARENT;
    visuals.window_stroke = egui::Stroke::NONE;
    visuals.selection.bg_fill = egui::Color32::from_rgb(116, 35, 203);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
    ctx.set_visuals(visuals);

    let bold_font_bytes =
        std::fs::read(crate::common::assets_path::resolve_asset_path(
            "content/game/fonts/Ubuntu-Bold.ttf",
        ))
        .or_else(|_| {
            std::fs::read(crate::common::assets_path::resolve_asset_path(
                "content/game/fonts/Ubuntu-bold.ttf",
            ))
        })
        .unwrap_or_default();

    let medium_font_bytes =
        std::fs::read(crate::common::assets_path::resolve_asset_path(
            "content/game/fonts/Ubuntu-Medium.ttf",
        ))
        .or_else(|_| {
            std::fs::read(crate::common::assets_path::resolve_asset_path(
                "content/game/fonts/Ubuntu-medium.ttf",
            ))
        })
        .unwrap_or_default();

    let regular_font_bytes =
        std::fs::read(crate::common::assets_path::resolve_asset_path(
            "content/game/fonts/Ubuntu.ttf",
        ))
        .unwrap_or_default();

    let mut fonts = egui::FontDefinitions::default();
    let mut has_font = false;

    if !regular_font_bytes.is_empty() {
        fonts.font_data.insert(
            "Ubuntu-Regular".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(regular_font_bytes)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Ubuntu-Regular".to_owned());
        has_font = true;
    }

    if !medium_font_bytes.is_empty() {
        fonts.font_data.insert(
            "Ubuntu-Medium".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(medium_font_bytes)),
        );
        fonts.families.insert(
            egui::FontFamily::Name("Medium".into()),
            vec!["Ubuntu-Medium".to_owned()],
        );
    }

    if !bold_font_bytes.is_empty() {
        fonts.font_data.insert(
            "Ubuntu-Bold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bold_font_bytes)),
        );
        fonts.families.insert(
            egui::FontFamily::Name("Bold".into()),
            vec!["Ubuntu-Bold".to_owned()],
        );
        if !has_font {
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Ubuntu-Bold".to_owned());
            has_font = true;
        }
    }

    if has_font {
        ctx.set_fonts(fonts);
    }

    *initialized = true;
}
