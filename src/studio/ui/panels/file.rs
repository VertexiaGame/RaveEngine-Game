use bevy::prelude::*;
use bevy_egui::egui;

pub fn draw_file_window(
    ctx: &egui::Context,
    open: &mut bool,
    onboarding_data: &mut crate::studio::ui::panels::onboarding::OnboardingData,
    file_dialog_state: &crate::studio::ui::resources::FileDialogState,
) {
    egui::Window::new("File")
        .open(open)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_size(egui::vec2(320.0, 200.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Save/Load Project").strong().size(14.0));
            ui.add_space(8.0);

            ui.label("File Path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut onboarding_data.save_path);
                let is_open = file_dialog_state.is_open.load(std::sync::atomic::Ordering::Relaxed);
                if ui.add_enabled(!is_open, egui::Button::new("Browse...")).clicked() {
                    file_dialog_state.is_open.store(true, std::sync::atomic::Ordering::Relaxed);
                    let tx = file_dialog_state.tx.clone();
                    std::thread::spawn(move || {
                        if let Some(path) = crate::studio::ui::resources::save_file_dialog("Rave Project") {
                            let _ = tx.send(crate::studio::ui::resources::FileDialogResult::BrowseSavePath(path));
                        } else {
                            let _ = tx.send(crate::studio::ui::resources::FileDialogResult::Cancel);
                        }
                    });
                }
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let save_enabled = !onboarding_data.save_path.is_empty();
                let is_open = file_dialog_state.is_open.load(std::sync::atomic::Ordering::Relaxed);
                if ui.add_enabled(save_enabled, egui::Button::new("Save")).clicked() {
                    if let Ok(state) = crate::common::core::vrtx::VrtxFileState::load_from_file(&onboarding_data.save_path) {
                        let _ = state.save_to_file(&onboarding_data.save_path);
                    }
                }

                if ui.add_enabled(!is_open, egui::Button::new("Open...")).clicked() {
                    file_dialog_state.is_open.store(true, std::sync::atomic::Ordering::Relaxed);
                    let tx = file_dialog_state.tx.clone();
                    std::thread::spawn(move || {
                        if let Some(path) = crate::studio::ui::resources::pick_file_dialog("Rave Project") {
                            let _ = tx.send(crate::studio::ui::resources::FileDialogResult::OpenFile(path));
                        } else {
                            let _ = tx.send(crate::studio::ui::resources::FileDialogResult::Cancel);
                        }
                    });
                }
            });
        });
}