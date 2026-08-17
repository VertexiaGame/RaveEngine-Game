use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

#[derive(Resource, Default)]
pub struct ChatboxState {
    pub text: String,
}

#[derive(Default)]
pub struct ChatboxTextures {
    pub menu_handle: Option<Handle<Image>>,
    pub chat_handle: Option<Handle<Image>>,
    pub menu_tex: Option<egui::TextureId>,
    pub chat_tex: Option<egui::TextureId>,
}

pub fn draw_chatbox(
    mut contexts: EguiContexts,
    mut chatbox_state: ResMut<ChatboxState>,
    keys: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut textures: Local<ChatboxTextures>,
) {
    let menu_handle = textures
        .menu_handle
        .get_or_insert_with(|| asset_server.load("content/game/ui/stuff/menu.png"))
        .clone();
    let chat_handle = textures
        .chat_handle
        .get_or_insert_with(|| asset_server.load("content/game/ui/stuff/chat.png"))
        .clone();

    if let Some(mut menu_image) = images.get_mut(&menu_handle) {
        if !matches!(menu_image.sampler, bevy::image::ImageSampler::Descriptor(_)) {
            let format = menu_image.texture_descriptor.format;
            if format == bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb 
                || format == bevy::render::render_resource::TextureFormat::Rgba8Unorm 
            {
                if let Some(ref mut data) = menu_image.data {
                    for chunk in data.chunks_exact_mut(4) {
                        let a = chunk[3] as f32 / 255.0;
                        chunk[0] = (chunk[0] as f32 * a) as u8;
                        chunk[1] = (chunk[1] as f32 * a) as u8;
                        chunk[2] = (chunk[2] as f32 * a) as u8;
                    }
                }
            }
            menu_image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
                address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
                mag_filter: bevy::image::ImageFilterMode::Linear,
                min_filter: bevy::image::ImageFilterMode::Linear,
                ..default()
            });
        }
    }

    if let Some(mut chat_image) = images.get_mut(&chat_handle) {
        if !matches!(chat_image.sampler, bevy::image::ImageSampler::Descriptor(_)) {
            let format = chat_image.texture_descriptor.format;
            if format == bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb 
                || format == bevy::render::render_resource::TextureFormat::Rgba8Unorm 
            {
                if let Some(ref mut data) = chat_image.data {
                    for chunk in data.chunks_exact_mut(4) {
                        let a = chunk[3] as f32 / 255.0;
                        chunk[0] = (chunk[0] as f32 * a) as u8;
                        chunk[1] = (chunk[1] as f32 * a) as u8;
                        chunk[2] = (chunk[2] as f32 * a) as u8;
                    }
                }
            }
            chat_image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
                address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
                mag_filter: bevy::image::ImageFilterMode::Linear,
                min_filter: bevy::image::ImageFilterMode::Linear,
                ..default()
            });
        }
    }

    let menu_tex = if let Some(tex) = textures.menu_tex {
        tex
    } else {
        let tex = contexts.add_image(bevy_egui::EguiTextureHandle::Strong(menu_handle));
        textures.menu_tex = Some(tex);
        tex
    };

    let chat_tex = if let Some(tex) = textures.chat_tex {
        tex
    } else {
        let tex = contexts.add_image(bevy_egui::EguiTextureHandle::Strong(chat_handle));
        textures.chat_tex = Some(tex);
        tex
    };

    let Ok(ctx) = contexts.ctx_mut() else { return; };

    let screen_rect = ctx.content_rect();
    let screen_width = screen_rect.width();

    let final_width = screen_width - 20.0f32;

    let request_focus = keys.just_pressed(KeyCode::Slash) && !ctx.egui_wants_keyboard_input();

    let scale_factor = (screen_width / 1280.0).clamp(0.7, 1.2);

    egui::Area::new(egui::Id::new("client_chatbox_area"))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(10.0, 0.0))
        .show(ctx, |ui| {
            let bg_color = egui::Color32::from_rgba_unmultiplied(61, 61, 61, 102);

            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0 * scale_factor);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0 * scale_factor, 0.0);

                    let button_size = egui::vec2(62.0 * scale_factor, 62.0 * scale_factor);
                    let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let visual_bg_color = if response.hovered() {
                        egui::Color32::from_rgba_unmultiplied(81, 81, 81, 150)
                    } else {
                        bg_color
                    };
                    ui.painter().rect_filled(rect, 4.0 * scale_factor, visual_bg_color);
                    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(35.0 * scale_factor, 35.0 * scale_factor));
                    ui.painter().image(
                        menu_tex,
                        icon_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );

                    let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let visual_bg_color = if response.hovered() {
                        egui::Color32::from_rgba_unmultiplied(81, 81, 81, 150)
                    } else {
                        bg_color
                    };
                    ui.painter().rect_filled(rect, 4.0 * scale_factor, visual_bg_color);
                    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(35.0 * scale_factor, 35.0 * scale_factor));
                    ui.painter().image(
                        chat_tex,
                        icon_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                });

                egui::Frame::NONE
                    .fill(bg_color)
                    .corner_radius(egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 })
                    .inner_margin(egui::Margin { left: 14, right: 14, top: 4, bottom: 4 })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        let mut visuals = egui::Visuals::dark();
                        visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
                        visuals.text_edit_bg_color = Some(egui::Color32::TRANSPARENT);
                        visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                        visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
                        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
                        visuals.selection.bg_fill = egui::Color32::from_rgb(116, 35, 203);
                        visuals.override_text_color = Some(egui::Color32::WHITE);
                        visuals.weak_text_color = Some(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 128));
                        ui.style_mut().visuals = visuals;

                        ui.set_width(final_width - 28.0f32);
                        ui.set_height(16.0f32);

                        let text_edit = egui::TextEdit::singleline(&mut chatbox_state.text)
                            .frame(egui::Frame::NONE)
                            .hint_text(
                                egui::RichText::new("Press \"/\" or click here to chat...")
                                    .italics()
                                    .size(14.0)
                                    .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 128))
                            )
                            .text_color(egui::Color32::WHITE)
                            .font(egui::FontId::new(14.0, egui::FontFamily::Proportional))
                            .desired_width(f32::INFINITY);

                        let response = ui.add(text_edit);

                        if request_focus {
                            response.request_focus();
                        }

                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if !chatbox_state.text.is_empty() {
                                info!("PLAYER_CHAT: {}", chatbox_state.text);
                                chatbox_state.text.clear();
                            }
                        }
                    });
            });
        });
}