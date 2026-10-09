use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use crate::client::ui::chat_container::ChatPanelState;

#[derive(Resource, Default)]
pub struct ChatboxState {
    pub text: String,
    pub cooldown_until: f64,
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
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut textures: Local<ChatboxTextures>,
    time: Res<Time>,
    mut visibility: ResMut<ChatPanelState>,
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
            menu_image.sampler =
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
            chat_image.sampler =
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

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let screen_rect = ctx.content_rect();
    let screen_width = screen_rect.width();
    let scale_factor = (screen_width / 1280.0).clamp(0.7, 1.2);

    let resp = egui::Area::new(egui::Id::new("client_chatbox_area"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 10.0))
        .show(ctx, |ui| {
            let bg_color = egui::Color32::from_rgba_unmultiplied(61, 61, 61, 102);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2((8.0 * scale_factor).round(), 0.0);
                let button_size = egui::vec2((52.0 * scale_factor).round(), (52.0 * scale_factor).round());
                let (rect, response) =
                    ui.allocate_exact_size(button_size, egui::Sense::CLICK);
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let visual_bg_color = if response.hovered() {
                    egui::Color32::from_rgba_unmultiplied(81, 81, 81, 150)
                } else {
                    bg_color
                };
                ui.painter()
                    .rect_filled(rect, (4.0 * scale_factor).round(), visual_bg_color);
                let icon_rect = egui::Rect::from_center_size(
                    rect.center(),
                    egui::vec2((28.0 * scale_factor).round(), (28.0 * scale_factor).round()),
                );
                ui.painter().image(
                    menu_tex,
                    icon_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                let (rect2, response2) =
                    ui.allocate_exact_size(button_size, egui::Sense::CLICK);
                if response2.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let visual_bg_color2 = if response2.hovered() {
                    egui::Color32::from_rgba_unmultiplied(81, 81, 81, 150)
                } else {
                    bg_color
                };
                ui.painter()
                    .rect_filled(rect2, (4.0 * scale_factor).round(), visual_bg_color2);
                let icon_rect2 = egui::Rect::from_center_size(
                    rect2.center(),
                    egui::vec2((28.0 * scale_factor).round(), (28.0 * scale_factor).round()),
                );
                ui.painter().image(
                    chat_tex,
                    icon_rect2,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                let chat_clicked = response2.clicked();
                if response.hovered()
                    || response2.hovered()
                    || response.clicked()
                    || chat_clicked
                {
                    visibility.last_active = time.elapsed_secs_f64();
                }
                if chat_clicked {
                    visibility.visible = !visibility.visible;
                }
            });
        });
    if resp.response.hovered() {
        visibility.last_active = time.elapsed_secs_f64();
    }
}
