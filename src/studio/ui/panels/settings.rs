use bevy_egui::egui;

pub fn draw_settings_window(
    ctx: &egui::Context,
    open: &mut bool,
) {
    egui::Window::new("Settings")
        .open(open)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .default_size(egui::vec2(250.0, 120.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("hi hi hello").size(14.0).color(egui::Color32::from_rgb(20, 20, 20)));
        });
}