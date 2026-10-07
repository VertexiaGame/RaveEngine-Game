use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

#[derive(Default)]
pub struct ScoreboardCache {
    names: Vec<String>,
    count: usize,
}

#[derive(Default)]
pub struct ScoreboardTextures {
    pub ld_handle: Option<Handle<Image>>,
    pub ld_tex: Option<egui::TextureId>,
}

#[derive(Resource)]
pub struct ScoreboardPanelState {
    pub visible: bool,
}
impl Default for ScoreboardPanelState {
    fn default() -> Self { Self { visible: true } }
}

pub fn draw_scoreboard(
    mut contexts: EguiContexts,
    query_players: Query<Ref<crate::common::net::components::Player>>,
    mut cache: Local<ScoreboardCache>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut textures: Local<ScoreboardTextures>,
    mut panel_state: ResMut<ScoreboardPanelState>,
) {
    let ld_handle = textures
        .ld_handle
        .get_or_insert_with(|| asset_server.load("content/game/ui/stuff/ld.png"))
        .clone();
    if let Some(mut ld_image) = images.get_mut(&ld_handle) {
        if !matches!(ld_image.sampler, bevy::image::ImageSampler::Descriptor(_)) {
            let format = ld_image.texture_descriptor.format;
            if format == bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
                || format == bevy::render::render_resource::TextureFormat::Rgba8Unorm
            {
                if let Some(ref mut data) = ld_image.data {
                    for chunk in data.chunks_exact_mut(4) {
                        let a = chunk[3] as f32 / 255.0;
                        chunk[0] = (chunk[0] as f32 * a) as u8;
                        chunk[1] = (chunk[1] as f32 * a) as u8;
                        chunk[2] = (chunk[2] as f32 * a) as u8;
                    }
                }
            }
            ld_image.sampler =
                bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                    address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
                    address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
                    mag_filter: bevy::image::ImageFilterMode::Linear,
                    min_filter: bevy::image::ImageFilterMode::Linear,
                    mipmap_filter: bevy::image::ImageFilterMode::Linear,
                    ..default()
                });
        }
    }
    let ld_tex = if let Some(tex) = textures.ld_tex {
        tex
    } else {
        let tex = contexts.add_image(bevy_egui::EguiTextureHandle::Strong(ld_handle.clone()));
        textures.ld_tex = Some(tex);
        tex
    };

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let screen_rect = ctx.content_rect();
    let screen_width = screen_rect.width();

    let scale_factor = (screen_width / 1280.0).clamp(0.7, 1.2);

    let base_width = (240.0 * scale_factor).round();
    let horizontal_margin = (14.0 * scale_factor).round();

    let header_v_margin = (8.0 * scale_factor).round();
    let header_inner_width = base_width - (horizontal_margin * 2.0);
    let header_font_size = (13.0 * scale_factor).max(11.0).round();

    let player_v_margin = (4.0 * scale_factor).round();
    let player_inner_width = base_width - (horizontal_margin * 2.0);
    let player_font_size = (11.0 * scale_factor).max(10.0).round();

    let item_spacing = (5.0 * scale_factor).round();
    let max_height = (260.0 * scale_factor).round();

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

    let is_bold_loaded = ctx.fonts(|f| {
        f.families()
            .contains(&egui::FontFamily::Name("Bold".into()))
    });

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

    let header_stats_font = if is_bold_loaded {
        egui::FontId::new((10.5 * scale_factor).max(9.5).round(), egui::FontFamily::Name("Bold".into()))
    } else {
        egui::FontId::new((10.5 * scale_factor).max(9.5).round(), egui::FontFamily::Proportional)
    };

    let ld_icon_size = {
        let max = (28.0 * scale_factor).round();
        let mut size = egui::vec2(max, max);
        if let Some(img) = images.get(&ld_handle) {
            let w = img.width() as f32;
            let h = img.height() as f32;
            if w > 0.0 && h > 0.0 {
                let aspect = w / h;
                if aspect > 1.0 {
                    size.y = max / aspect;
                } else {
                    size.x = max * aspect;
                }
            }
        }
        size
    };

    let anim = ctx.animate_bool(egui::Id::new("scoreboard_visible"), panel_state.visible);

    let button_resp = egui::Area::new(egui::Id::new("client_menu_button_area"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 10.0))
        .show(ctx, |ui| {
            let button_size = egui::vec2((52.0 * scale_factor).round(), (52.0 * scale_factor).round());
            let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let visual_bg = if response.hovered() {
                egui::Color32::from_rgba_unmultiplied(81, 81, 81, 150)
            } else {
                bg_color
            };
            ui.painter().rect_filled(rect, (4.0 * scale_factor).round(), visual_bg);
            let icon_rect =
                egui::Rect::from_center_size(rect.center(), ld_icon_size);
            ui.painter().image(
                ld_tex,
                icon_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            response
        })
        .inner;
    if button_resp.clicked() {
        panel_state.visible = !panel_state.visible;
    }

    if anim <= 0.01 {
        return;
    }

    let scoreboard_top = (10.0 + 52.0 * scale_factor + 8.0 * scale_factor).round();
    let bg_anim = egui::Color32::from_rgba_unmultiplied(61, 61, 61, (102.0 * anim).round() as u8);
    let title_a = (255.0 * anim).round() as u8;
    let stats_a = (190.0 * anim).round() as u8;
    let player_a = (210.0 * anim).round() as u8;
    egui::Area::new(egui::Id::new("client_scoreboard_area"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, scoreboard_top))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, item_spacing);

                egui::Frame::NONE
                    .fill(bg_anim)
                    .corner_radius((4.0 * scale_factor).round())
                    .inner_margin(egui::Margin::symmetric(
                        horizontal_margin.round() as i8,
                        header_v_margin.round() as i8,
                    ))
                    .show(ui, |ui| {
                        ui.set_width(header_inner_width);
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("Scoreboard")
                                        .color(egui::Color32::from_rgba_unmultiplied(
                                            255, 255, 255, title_a,
                                        ))
                                        .font(title_font.clone()),
                                )
                                .selectable(false),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new("W/O")
                                                .color(egui::Color32::from_rgba_unmultiplied(
                                                    255, 255, 255, stats_a,
                                                ))
                                                .font(header_stats_font.clone()),
                                        )
                                        .selectable(false),
                                    );
                                    ui.add_space((10.0 * scale_factor).round());
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new("K/O")
                                                .color(egui::Color32::from_rgba_unmultiplied(
                                                    255, 255, 255, stats_a,
                                                ))
                                                .font(header_stats_font.clone()),
                                        )
                                        .selectable(false),
                                    );
                                },
                            );
                        });
                    });

                egui::ScrollArea::vertical()
                    .max_height(max_height)
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        ui.set_width(base_width);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(0.0, item_spacing);

                            for (idx, username) in cache.names.iter().enumerate() {
                                let (k, w) = if idx < 2 {
                                    ("12", "1")
                                } else if idx % 2 == 0 {
                                    ("12", "1")
                                } else {
                                    ("1", "12")
                                };
                                let row_bg = bg_anim;
                                egui::Frame::NONE
                                    .fill(row_bg)
                                    .corner_radius((4.0 * scale_factor).round())
                                    .inner_margin(egui::Margin::symmetric(
                                        horizontal_margin.round() as i8,
                                        player_v_margin.round() as i8,
                                    ))
                                    .show(ui, |ui| {
                                        ui.set_width(player_inner_width);
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(username)
                                                        .color(egui::Color32::from_rgba_unmultiplied(
                                                            255, 255, 255, title_a,
                                                        ))
                                                        .font(player_font.clone()),
                                                )
                                                .selectable(false),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(w)
                                                                .color(egui::Color32::from_rgba_unmultiplied(
                                                                    255, 255, 255, player_a,
                                                                ))
                                                                .font(player_font.clone()),
                                                        )
                                                        .selectable(false),
                                                    );
                                                    ui.add_space((14.0 * scale_factor).round());
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(k)
                                                                .color(egui::Color32::from_rgba_unmultiplied(
                                                                    255, 255, 255, player_a,
                                                                ))
                                                                .font(player_font.clone()),
                                                        )
                                                        .selectable(false),
                                                    );
                                                },
                                            );
                                        });
                                    });
                            }
                        });
                    });
            });
        });
}
