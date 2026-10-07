use bevy_egui::{EguiContexts, egui};

pub fn draw_health_bar(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let screen_rect = ctx.content_rect();
    let screen_width = screen_rect.width();

    let scale_factor = (screen_width / 1280.0).clamp(0.7, 1.2);

    egui::Area::new(egui::Id::new("client_health_bar_area"))
        .anchor(
            egui::Align2::CENTER_BOTTOM,
            egui::vec2(0.0, -12.0 * scale_factor),
        )
        .show(ctx, |ui| {
            let width = 300.0 * scale_factor;
            let height = 28.0 * scale_factor;
            let (rect, _response) =
                ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
            let painter = ui.painter();

            let inner_rect = rect.shrink(1.0);
            let gradient = egui::Shape::gradient_rect(
                inner_rect,
                egui::Direction::TopDown,
                [
                    egui::Color32::from_rgb(74, 184, 80),
                    egui::Color32::from_rgb(48, 120, 52),
                ],
            );
            painter.add(gradient);

            painter.rect_stroke(
                rect,
                6.0 * scale_factor,
                egui::Stroke::new(2.0 * scale_factor, egui::Color32::from_rgb(61, 61, 61)),
                egui::StrokeKind::Inside,
            );

            let font_size = 13.0 * scale_factor;
            let is_medium_loaded = ctx.fonts(|f| {
                f.families()
                    .contains(&egui::FontFamily::Name("Medium".into()))
            });
            let font = if is_medium_loaded {
                egui::FontId::new(font_size, egui::FontFamily::Name("Medium".into()))
            } else {
                egui::FontId::new(font_size, egui::FontFamily::Proportional)
            };

            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Health",
                font,
                egui::Color32::WHITE,
            );
        });
}
