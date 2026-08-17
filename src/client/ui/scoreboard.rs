use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

#[derive(Default)]
pub struct ScoreboardCache {
    names: Vec<String>,
    count: usize,
}

pub fn draw_scoreboard(
    mut contexts: EguiContexts,
    query_players: Query<Ref<crate::common::net::components::Player>>,
    mut cache: Local<ScoreboardCache>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };

    let screen_rect = ctx.content_rect();
    let screen_width = screen_rect.width();

    let scale_factor = if screen_width > 1280.0 {
        (1.0 + (screen_width / 1280.0 - 1.0) * 0.8).min(1.7)
    } else {
        1.0
    };

    let base_width = 190.0 * scale_factor;
    let horizontal_margin = 12.0 * scale_factor;

    let header_v_margin = 10.0 * scale_factor;
    let header_inner_width = base_width - (horizontal_margin * 2.0);
    let header_font_size = 13.5 * scale_factor;

    let player_v_margin = 4.0 * scale_factor;
    let player_inner_width = base_width - (horizontal_margin * 2.0);
    let player_font_size = 11.5 * scale_factor;

    let item_spacing = 5.0 * scale_factor;
    let max_height = 250.0 * scale_factor;

    let mut changed = false;
    let mut count = 0;
    for player in &query_players {
        count += 1;
        changed |= player.is_changed();
    }
    if changed || count != cache.count {
        let mut players_map = std::collections::HashMap::new();
        for player in &query_players {
            let display_name = if player.username.is_empty() {
                format!("Player_{}", player.client_id)
            } else {
                player.username.clone()
            };
            players_map.insert(player.client_id, display_name);
        }
        cache.names = players_map.into_values().collect();
        cache.names.sort();
        cache.count = count;
        if cache.names.is_empty() {
            cache.names.push("LocalPlayer".to_string());
        }
    }

    let bg_color = egui::Color32::from_rgba_unmultiplied(61, 61, 61, 102);

    let is_bold_loaded = ctx.fonts(|f| f.families().contains(&egui::FontFamily::Name("Bold".into())));

    let title_font = if is_bold_loaded {
        egui::FontId::new(header_font_size, egui::FontFamily::Name("Bold".into()))
    } else {
        egui::FontId::new(header_font_size, egui::FontFamily::Proportional)
    };

    let player_font = if is_bold_loaded {
        egui::FontId::new(player_font_size, egui::FontFamily::Name("Bold".into()))
    } else {
        egui::FontId::new(player_font_size, egui::FontFamily::Proportional)
    };

    egui::Area::new(egui::Id::new("client_scoreboard_area"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 16.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, item_spacing);

                egui::Frame::NONE
                    .fill(bg_color)
                    .corner_radius(4.0 * scale_factor)
                    .inner_margin(egui::Margin::symmetric(horizontal_margin as i8, header_v_margin as i8))
                    .show(ui, |ui| {
                        ui.set_width(header_inner_width);
                        ui.horizontal(|ui| {
                            ui.add(egui::Label::new(
                                egui::RichText::new("Scoreboard")
                                    .color(egui::Color32::WHITE)
                                    .font(title_font),
                            ).selectable(false));
                        });
                    });

                egui::ScrollArea::vertical()
                    .max_height(max_height)
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        ui.set_width(base_width);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(0.0, item_spacing);

                            for username in &cache.names {
                                egui::Frame::NONE
                                    .fill(bg_color)
                                    .corner_radius(4.0 * scale_factor)
                                    .inner_margin(egui::Margin::symmetric(horizontal_margin as i8, player_v_margin as i8))
                                    .show(ui, |ui| {
                                        ui.set_width(player_inner_width);
                                        ui.horizontal(|ui| {
                                            ui.add(egui::Label::new(
                                                egui::RichText::new(username)
                                                    .color(egui::Color32::WHITE)
                                                    .font(player_font.clone()),
                                            ).selectable(false));
                                        });
                                    });
                            }
                        });
                    });
            });
        });
}