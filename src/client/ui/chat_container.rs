use bevy::ecs::{resource::Resource, system::ResMut};
use bevy_egui::{EguiContexts, egui::{self}};

struct MockMessage {
    username: &'static str,
    color: egui::Color32,
    text: &'static str,
}

const MOCK_MESSAGES: &[MockMessage] = &[
    MockMessage {
        username: "[devjuice]",
        color: egui::Color32::from_rgb(255, 180, 0),
        text: "Hi there!",
    },
    MockMessage {
        username: "[onewhosteps]",
        color: egui::Color32::from_rgb(50, 205, 50),
        text: "OMG HI DEVJUICE LMAD LMAD KMAD",
    },
];

#[derive(Resource, Default)]
pub struct ChatContState {
    pub messages: Vec<String>
}

pub fn draw_chat_container(
    mut contexts: EguiContexts,
    mut chat_cont_state: ResMut<ChatContState>
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };

    let screen_rect = ctx.content_rect();
    let screen_width = screen_rect.width();

    let scale_factor = (screen_width / 1280.0).clamp(0.7, 1.2);

    let bg_color = egui::Color32::from_rgba_unmultiplied(61, 61, 61, 102);

    egui::Area::new(egui::Id::new("client_chat_container_area"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(16.0, 16.0))
        .show(ctx, |ui| {
            let width = 350.0 * scale_factor;
            let height = 200.0 * scale_factor;
            ui.set_width(width);
            ui.set_height(height);

            egui::ScrollArea::vertical()
                .max_height(height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0 * scale_factor);

                        for msg in MOCK_MESSAGES {
                            let min_w = 138.0 * scale_factor;

                            egui::Frame::NONE
                                .fill(bg_color)
                                .corner_radius(4.0 * scale_factor)
                                .inner_margin(egui::Margin::symmetric(
                                    (10.0 * scale_factor) as i8,
                                    (2.3 * scale_factor) as i8,
                                ))
                                .show(ui, |ui| {
                                    ui.set_min_width(min_w);

                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(4.0 * scale_factor, 0.0);

                                        ui.add(egui::Label::new(
                                            egui::RichText::new(msg.username)
                                                .color(msg.color)
                                                .font(egui::FontId::new(13.0 * scale_factor, egui::FontFamily::Proportional))
                                        ).selectable(false));

                                        ui.add(egui::Label::new(
                                            egui::RichText::new(msg.text)
                                                .color(egui::Color32::WHITE)
                                                .font(egui::FontId::new(13.0 * scale_factor, egui::FontFamily::Proportional))
                                        ).selectable(false));
                                    });
                                });
                        }
                        for msg in chat_cont_state.messages.clone() {
                            let min_w = 138.0 * scale_factor;

                            egui::Frame::NONE
                                .fill(bg_color)
                                .corner_radius(4.0 * scale_factor)
                                .inner_margin(egui::Margin::symmetric(
                                    (10.0 * scale_factor) as i8,
                                    (2.3 * scale_factor) as i8,
                                ))
                                .show(ui, |ui| {
                                    ui.set_min_width(min_w);

                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(4.0 * scale_factor, 0.0);

                                        ui.add(egui::Label::new(
                                            egui::RichText::new("Locke")
                                                .color(egui::Color32::from_rgb(255, 180, 0))
                                                .font(egui::FontId::new(13.0 * scale_factor, egui::FontFamily::Proportional))
                                        ).selectable(false));

                                        ui.add(egui::Label::new(
                                            egui::RichText::new(msg)
                                                .color(egui::Color32::WHITE)
                                                .font(egui::FontId::new(13.0 * scale_factor, egui::FontFamily::Proportional))
                                        ).selectable(false));
                                    });
                                });
                        }
                    });
                });
        });
}

pub fn get_message(text: String) {
    println!("{}", text);
    //messages.push(text);
}