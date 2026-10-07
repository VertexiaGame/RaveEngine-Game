use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self},
};
use lightyear::prelude::*;
use crate::client::ui::chatbox::ChatboxState;
use crate::common::net::messages::{
    CHAT_COOLDOWN_SECS, CHAT_MAX_CHARS, CHAT_MAX_MESSAGES, ChatSendMessage,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ChatEntry {
    pub username: String,
    pub text: String,
}

#[derive(Resource, Default)]
pub struct ChatContState {
    pub messages: std::collections::VecDeque<ChatEntry>,
}

impl ChatContState {
    pub fn push_entry(&mut self, username: String, text: String) {
        if self.messages.len() >= CHAT_MAX_MESSAGES {
            self.messages.pop_front();
        }
        self.messages.push_back(ChatEntry { username, text });
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[derive(Resource, Default)]
pub struct LocalUsername(pub String);

#[derive(Resource)]
pub struct ChatPanelState {
    pub last_active: f64,
    pub visible: bool,
}
impl Default for ChatPanelState {
    fn default() -> Self {
        Self {
            last_active: 0.0,
            visible: true,
        }
    }
}

fn username_label(username: &str) -> String {
    format!("[{}]", username)
}

fn username_color(username: &str, local_username: &str) -> egui::Color32 {
    if !local_username.is_empty() && username == local_username {
        egui::Color32::from_rgb(74, 168, 184)
    } else {
        egui::Color32::from_rgb(93, 177, 123)
    }
}

fn chat_text_color(username: &str, local_username: &str) -> egui::Color32 {
    if !local_username.is_empty() && username == local_username {
        egui::Color32::from_rgb(251, 202, 86)
    } else {
        egui::Color32::WHITE
    }
}

pub fn draw_chat_container(
    mut contexts: EguiContexts,
    chat_cont_state: Res<ChatContState>,
    mut chatbox_state: ResMut<ChatboxState>,
    mut visibility: ResMut<ChatPanelState>,
    local_username: Res<LocalUsername>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut senders: Query<&mut MessageSender<ChatSendMessage>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    if chatbox_state.text.chars().count() > CHAT_MAX_CHARS {
        chatbox_state.text = chatbox_state.text.chars().take(CHAT_MAX_CHARS).collect();
    }

    let screen_rect = ctx.content_rect();
    let screen_width = screen_rect.width();
    let scale_factor = (screen_width / 1280.0).clamp(0.7, 1.2);
    let _bg_color = egui::Color32::from_rgba_unmultiplied(61, 61, 61, 153);
    let now = time.elapsed_secs_f64();
    let cooldown_remaining = (chatbox_state.cooldown_until - now).max(0.0);
    let on_cooldown = cooldown_remaining > 0.001;

    let menu_h = (52.0 * scale_factor).round();
    let gap = (8.0 * scale_factor).round();
    let chat_top = (10.0 + menu_h + gap).round();
    let mut hovered = false;
    if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
        let outer_rect = egui::Rect::from_min_size(
            egui::pos2(10.0, chat_top),
            egui::vec2((500.0 * scale_factor).round(), (274.0 * scale_factor).round()),
        );
        if outer_rect.contains(pos) {
            hovered = true;
        }
        let button_rect = egui::Rect::from_min_size(
            egui::pos2(10.0, 10.0),
            egui::vec2((112.0 * scale_factor).round(), (52.0 * scale_factor).round()),
        );
        if button_rect.contains(pos) {
            hovered = true;
        }
    }
    if ctx.input(|i| i.pointer.primary_down()) && hovered {
        hovered = true;
    }

    let input_id = egui::Id::new("client_chat_input");
    let focused = ctx.memory(|m| m.has_focus(input_id));

    if hovered || focused {
        visibility.last_active = now;
    }
    let mut request_focus = false;
    if keys.just_pressed(KeyCode::Slash)
        && !ctx.egui_wants_keyboard_input()
        && !on_cooldown
    {
        visibility.last_active = now;
        request_focus = true;
    }
    if keys.just_pressed(KeyCode::Enter) && focused {
        visibility.last_active = now;
    }
    if !chatbox_state.text.is_empty() && focused {
        visibility.last_active = now;
    }
    if on_cooldown {
        visibility.last_active = now;
    }

    let auto_visible = focused || hovered || on_cooldown || (now - visibility.last_active < 3.5);
    let visible = visibility.visible && auto_visible;
    let anim = ctx.animate_bool(input_id.with("vis"), visible);
    let msg_anim = ctx.animate_bool(input_id.with("msg_vis"), visibility.visible);

    let width = (500.0 * scale_factor).round();
    let frame_height = (250.0 * scale_factor).round();
    let chatbox_height = (24.0 * scale_factor).round();
    let border_width = (1.5 * scale_factor).round().max(1.0);
    let anchor = egui::vec2(10.0, chat_top);

    let _area = egui::Area::new(egui::Id::new("client_chat_container_area"))
        .anchor(egui::Align2::LEFT_TOP, anchor)
        .show(ctx, |ui| {
            ui.set_width(width);
            let _frame = egui::Frame::NONE
                .fill(egui::Color32::TRANSPARENT)
                .inner_margin(egui::Margin::same(0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        let hover_bg = egui::Color32::from_rgba_unmultiplied(
                            61,
                            61,
                            61,
                            (anim * 42.0).clamp(0.0, 42.0) as u8,
                        );
                        let messages_frame = egui::Frame::NONE
                            .fill(hover_bg)
                            .corner_radius(egui::CornerRadius {
                                nw: ((3.0 * scale_factor).round() as u8),
                                ne: ((3.0 * scale_factor).round() as u8),
                                sw: 0,
                                se: 0,
                            })
                            .inner_margin(egui::Margin::same(
                                (10.0 * scale_factor).round() as i8
                            ))
                            .show(ui, |ui| {
                                ui.set_width((width - 20.0 * scale_factor).round());
                                ui.set_height((frame_height - 20.0 * scale_factor).round());
                                egui::ScrollArea::vertical()
                                    .max_height((frame_height - 20.0 * scale_factor).round())
                                    .auto_shrink([false, false])
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing =
                                                egui::vec2(0.0, (5.0 * scale_factor).round());
                                            let msg_bg = egui::Color32::from_rgba_unmultiplied(
                                                61,
                                                61,
                                                61,
                                                (153.0 * msg_anim).clamp(0.0, 153.0) as u8,
                                            );
                                            for msg in chat_cont_state.messages.iter() {
                                                let name_color = username_color(
                                                    &msg.username,
                                                    &local_username.0,
                                                );
                                                let body_color = chat_text_color(
                                                    &msg.username,
                                                    &local_username.0,
                                                );
                                                egui::Frame::NONE
                                                    .fill(msg_bg)
                                                    .corner_radius((4.0 * scale_factor).round())
                                                    .inner_margin(egui::Margin::symmetric(
                                                        (5.0 * scale_factor).round() as i8,
                                                        (4.0 * scale_factor).round() as i8,
                                                    ))
                                                    .show(ui, |ui| {
                                                        ui.set_min_width(
                                                            (138.0 * scale_factor).round(),
                                                        );
                                                        ui.horizontal_wrapped(|ui| {
                                                            ui.spacing_mut().item_spacing =
                                                                egui::vec2(
                                                                    (10.0 * scale_factor).round(),
                                                                    0.0,
                                                                );
                                                            ui.add(
                                                                egui::Label::new(
                                                                    egui::RichText::new(
                                                                        username_label(&msg.username),
                                                                    )
                                                                    .color(
                                                                        name_color.gamma_multiply(
                                                                            msg_anim,
                                                                        ),
                                                                    )
                                                                    .font(egui::FontId::new(
                                                                        (14.0 * scale_factor)
                                                                            .max(11.0)
                                                                            .round(),
                                                                        egui::FontFamily::Proportional,
                                                                    )),
                                                                )
                                                                .selectable(false),
                                                            );
                                                            ui.add(
                                                                egui::Label::new(
                                                                    egui::RichText::new(&msg.text)
                                                                        .color(
                                                                            body_color.gamma_multiply(
                                                                                msg_anim,
                                                                            ),
                                                                        )
                                                                        .font(egui::FontId::new(
                                                                            (14.0 * scale_factor)
                                                                                .max(11.0)
                                                                                .round(),
                                                                            egui::FontFamily::Proportional,
                                                                        )),
                                                                )
                                                                .selectable(false),
                                                            );
                                                        });
                                                    });
                                            }
                                        });
                                    });
                            });
                        let input_rect = if anim > 0.01 {
                            let input_alpha = (anim * 153.0).clamp(0.0, 153.0) as u8;
                            let input_bg = egui::Color32::from_rgba_unmultiplied(
                                61, 61, 61, input_alpha,
                            );
                            let frame = egui::Frame::NONE
                                .fill(input_bg)
                                .corner_radius(egui::CornerRadius {
                                    nw: 0,
                                    ne: 0,
                                    sw: 4,
                                    se: 4,
                                })
                                .inner_margin(egui::Margin {
                                    left: (14.0 * scale_factor).round() as i8,
                                    right: (14.0 * scale_factor).round() as i8,
                                    top: (4.0 * scale_factor).round() as i8,
                                    bottom: (4.0 * scale_factor).round() as i8,
                                })
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(0.0, (2.0 * scale_factor).round());
                                    let mut visuals = egui::Visuals::dark();
                                    visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
                                    visuals.text_edit_bg_color =
                                        Some(egui::Color32::TRANSPARENT);
                                    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                    visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                    visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                                    visuals.widgets.noninteractive.bg_stroke =
                                        egui::Stroke::NONE;
                                    visuals.selection.stroke =
                                        egui::Stroke::new(1.0, egui::Color32::BLACK);
                                    visuals.selection.bg_fill =
                                        egui::Color32::from_rgb(116, 35, 203);
                                    visuals.override_text_color =
                                        Some(egui::Color32::WHITE);
                                    visuals.weak_text_color = Some(
                                        egui::Color32::from_rgba_unmultiplied(
                                            255, 255, 255, 128,
                                        ),
                                    );
                                    ui.style_mut().visuals = visuals;
                                    ui.set_width((width - 28.0 * scale_factor).round());
                                    let hint_alpha = (anim * 128.0).clamp(0.0, 128.0) as u8;
                                    let hint_text = if on_cooldown {
                                        format!(
                                            "Wait {:.0}s before chatting again...",
                                            cooldown_remaining.ceil()
                                        )
                                    } else {
                                        "Press \"/\" or click here to chat...".to_string()
                                    };
                                    let text_edit =
                                        egui::TextEdit::singleline(&mut chatbox_state.text)
                                            .id(input_id)
                                            .frame(egui::Frame::NONE)
                                            .interactive(!on_cooldown)
                                            .hint_text(
                                                egui::RichText::new(hint_text)
                                                .italics()
                                                .size((14.0 * scale_factor).max(11.0).round())
                                                .color(
                                                    egui::Color32::from_rgba_unmultiplied(
                                                        255, 255, 255, hint_alpha,
                                                    ),
                                                ),
                                            )
                                            .text_color(egui::Color32::from_rgba_unmultiplied(
                                                255,
                                                255,
                                                255,
                                                (anim * 255.0) as u8,
                                            ))
                                            .font(egui::FontId::new(
                                                (14.0 * scale_factor).max(11.0).round(),
                                                egui::FontFamily::Proportional,
                                            ))
                                            .desired_width(f32::INFINITY);
                                    let response = ui.add(text_edit);
                                    if request_focus {
                                        response.request_focus();
                                    }
                                    if response.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        let trimmed = chatbox_state.text.trim().to_string();
                                        if trimmed.is_empty() {
                                            chatbox_state.text.clear();
                                        } else if on_cooldown {
                                            visibility.last_active = now;
                                        } else if let Some(mut sender) =
                                            senders.iter_mut().next()
                                        {
                                            let _ = sender.send::<
                                                crate::common::net::messages::GameChannel,
                                            >(
                                                ChatSendMessage { text: trimmed },
                                            );
                                            chatbox_state.cooldown_until =
                                                now + CHAT_COOLDOWN_SECS;
                                            chatbox_state.text.clear();
                                            visibility.last_active = now;
                                        }
                                    }
                                    if response.clicked() {
                                        visibility.last_active = now;
                                    }
                                    let char_count =
                                        chatbox_state.text.chars().count();
                                    if focused || char_count > 0 {
                                        ui.horizontal(|ui| {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    let counter_alpha =
                                                        (anim * 160.0).clamp(0.0, 160.0) as u8;
                                                    let counter_color =
                                                        if char_count >= CHAT_MAX_CHARS {
                                                            egui::Color32::from_rgba_unmultiplied(
                                                                255, 120, 120, counter_alpha,
                                                            )
                                                        } else {
                                                            egui::Color32::from_rgba_unmultiplied(
                                                                255, 255, 255, counter_alpha,
                                                            )
                                                        };
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(format!(
                                                                "{}/{}",
                                                                char_count, CHAT_MAX_CHARS
                                                            ))
                                                            .color(counter_color)
                                                            .font(egui::FontId::new(
                                                                (11.0 * scale_factor)
                                                                    .max(10.0)
                                                                    .round(),
                                                                egui::FontFamily::Proportional,
                                                            )),
                                                        )
                                                        .selectable(false),
                                                    );
                                                },
                                            );
                                        });
                                    }
                                });
                            Some(frame.response.rect)
                        } else {
                            ui.allocate_space(egui::vec2(width, chatbox_height));
                            None
                        };
                        if anim > 0.01 {
                            let border_color = egui::Color32::from_rgba_unmultiplied(
                                61,
                                61,
                                61,
                                (anim * 153.0).clamp(0.0, 153.0) as u8,
                            );
                            let mr = messages_frame.response.rect;
                            let br_rect = if let Some(ir) = input_rect {
                                egui::Rect::from_min_max(
                                    egui::pos2(ir.min.x, mr.min.y),
                                    egui::pos2(ir.max.x, mr.max.y),
                                )
                            } else {
                                mr
                            };
                            let outer_r = (3.0 * scale_factor).round();
                            let bw = border_width;
                            let center_r = (outer_r - bw * 0.5).max(0.0);
                            let bl = egui::pos2(br_rect.min.x + bw * 0.5, br_rect.max.y);
                            let tl_mid = egui::pos2(br_rect.min.x + bw * 0.5, br_rect.min.y + outer_r);
                            let br = egui::pos2(br_rect.max.x - bw * 0.5, br_rect.max.y);
                            let mut points = Vec::new();
                            points.push(bl);
                            points.push(tl_mid);
                            let segs = 8;
                            for i in 1..=segs {
                                let t = i as f32 / segs as f32;
                                let angle = std::f32::consts::PI
                                    + t * std::f32::consts::FRAC_PI_2;
                                let x = br_rect.min.x + outer_r + center_r * angle.cos();
                                let y = br_rect.min.y + outer_r + center_r * angle.sin();
                                points.push(egui::pos2(x, y));
                            }
                            let p_top_right =
                                egui::pos2(br_rect.max.x - outer_r, br_rect.min.y + bw * 0.5);
                            points.push(p_top_right);
                            for i in 1..=segs {
                                let t = i as f32 / segs as f32;
                                let angle = 3.0 * std::f32::consts::FRAC_PI_2
                                    + t * std::f32::consts::FRAC_PI_2;
                                let x = br_rect.max.x - outer_r + center_r * angle.cos();
                                let y = br_rect.min.y + outer_r + center_r * angle.sin();
                                points.push(egui::pos2(x, y));
                            }
                            points.push(br);
                            ui.painter()
                                .add(egui::Shape::line(points, egui::Stroke::new(bw, border_color)));
                        }
                    });
                });
        });
}
