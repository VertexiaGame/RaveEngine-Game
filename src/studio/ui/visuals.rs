use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

pub fn configure_visuals(mut contexts: EguiContexts) {
    if let Ok(ctx) = contexts.ctx_mut() {
        ctx.set_visuals(egui::Visuals::light());

        let font_bytes = std::fs::read("assets/content/game/fonts/Ubuntu.ttf")
            .or_else(|_| std::fs::read("content/game/fonts/Ubuntu.ttf"))
            .unwrap_or_default();
        if !font_bytes.is_empty() {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "Ubuntu".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(font_bytes)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Ubuntu".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("Ubuntu".to_owned());
            ctx.set_fonts(fonts);
        }
    }
}