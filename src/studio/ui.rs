use bevy::prelude::*;
use bevy::image::{ImageType, CompressedImageFormats, ImageSampler};
use bevy::asset::RenderAssetUsages;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle};
use crate::studio::tools::{ToolState, Selection};
use crate::common::components::Brick;

#[derive(Resource)]
pub struct StudioUiAssets {
    pub move_icon: Handle<Image>,
    pub rotate_icon: Handle<Image>,
    pub scale_icon: Handle<Image>,
    pub add_icon: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct StudioUiTextureIds {
    pub move_tex: Option<egui::TextureId>,
    pub rotate_tex: Option<egui::TextureId>,
    pub scale_tex: Option<egui::TextureId>,
    pub add_tex: Option<egui::TextureId>,
}

#[derive(Resource, Default)]
pub struct CameraSpeedIndicator {
    pub visible_timer: f32,
    pub current_speed: f32,
}

pub fn update_camera_speed_indicator(
    mut indicator: ResMut<CameraSpeedIndicator>,
    camera_query: Query<(
        &bevy::camera_controller::free_camera::FreeCamera,
        &bevy::camera_controller::free_camera::FreeCameraState,
    )>,
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    time: Res<Time>,
) {
    let mut scrolled = false;
    for _ in scroll_events.read() {
        scrolled = true;
    }

    if scrolled {
        if let Some((free_camera, free_camera_state)) = camera_query.iter().next() {
            indicator.current_speed = free_camera.walk_speed * free_camera_state.speed_multiplier;
            indicator.visible_timer = 2.0;
        }
    } else if indicator.visible_timer > 0.0 {
        indicator.visible_timer -= time.delta_secs();
        if let Some((free_camera, free_camera_state)) = camera_query.iter().next() {
            indicator.current_speed = free_camera.walk_speed * free_camera_state.speed_multiplier;
        }
    }
}

fn load_icon_image(path: &str, images: &mut Assets<Image>) -> Handle<Image> {
    let bytes = std::fs::read(path).unwrap_or_else(|_| {
        std::fs::read(format!("assets/{}", path)).unwrap_or_default()
    });
    if bytes.is_empty() {
        return Handle::default();
    }

    let mut image = Image::from_buffer(
        &bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::all(),
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    ).ok();

    if image.is_none() {
        image = Image::from_buffer(
            &bytes,
            ImageType::Extension("jpg"),
            CompressedImageFormats::all(),
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        ).ok();
    }

    let final_image = image.unwrap_or_else(|| Image::default());
    images.add(final_image)
}

pub fn setup_ui_assets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let move_icon = load_icon_image("content/studio/icons/Tools/Move.png", &mut images);
    let rotate_icon = load_icon_image("content/studio/icons/Tools/Rotate.png", &mut images);
    let scale_icon = load_icon_image("content/studio/icons/Tools/Scale.png", &mut images);
    let add_icon = load_icon_image("content/studio/icons/Tools/Add.png", &mut images);

    commands.insert_resource(StudioUiAssets {
        move_icon,
        rotate_icon,
        scale_icon,
        add_icon,
    });
}

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

#[allow(deprecated)]
pub fn studio_ui(
    mut contexts: EguiContexts,
    mut next_tool: ResMut<NextState<ToolState>>,
    current_tool: Res<State<ToolState>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut count: ResMut<crate::studio::camera::BrickSpawnerCount>,
    ui_assets: Option<Res<StudioUiAssets>>,
    mut texture_ids: ResMut<StudioUiTextureIds>,
    mut camera_indicator: ResMut<CameraSpeedIndicator>,
    mut camera_query: Query<(
        &bevy::camera_controller::free_camera::FreeCamera,
        &mut bevy::camera_controller::free_camera::FreeCameraState,
    )>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut selection: ResMut<Selection>,
    entities_query: Query<(Entity, &Name, Option<&Brick>)>,
) {
    let Some(assets) = ui_assets else { return; };

    let move_tex = *texture_ids.move_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.move_icon.clone()))
    });
    let rotate_tex = *texture_ids.rotate_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.rotate_icon.clone()))
    });
    let scale_tex = *texture_ids.scale_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.scale_icon.clone()))
    });
    let add_tex = *texture_ids.add_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.add_icon.clone()))
    });

    let Ok(ctx) = contexts.ctx_mut() else { return; };

    let frame = egui::Frame::NONE
        .fill(egui::Color32::from_rgb(245, 246, 247))
        .inner_margin(egui::Margin::same(0));

    egui::Panel::top("topbar")
        .frame(frame)
        .show(ctx, |ui| {
            ui.style_mut().interaction.selectable_labels = false;

            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(12, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(16.0, 0.0);

                        ui.label(egui::RichText::new("File").color(egui::Color32::from_rgb(0, 0, 0)).size(13.0));
                        ui.label(egui::RichText::new("Edit").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        ui.label(egui::RichText::new("Insert").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        ui.label(egui::RichText::new("View").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        ui.label(egui::RichText::new("Test").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        ui.label(egui::RichText::new("Settings").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let fps = if let Some(diag) = diagnostics.get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS) {
                                diag.smoothed().unwrap_or_default()
                            } else {
                                0.0
                            };
                            ui.label(
                                egui::RichText::new(format!("FPS: {:.0}", fps))
                                    .color(egui::Color32::from_rgb(100, 100, 100))
                                    .size(13.0)
                            );
                        });
                    });
                });

            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(212, 212, 212));

            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

                        let is_move = *current_tool.get() == ToolState::Move;
                        if ribbon_button(ui, Some(move_tex), "Move", is_move).clicked() {
                            next_tool.set(ToolState::Move);
                        }

                        let is_rotate = *current_tool.get() == ToolState::Rotate;
                        if ribbon_button(ui, Some(rotate_tex), "Rotate", is_rotate).clicked() {
                            next_tool.set(ToolState::Rotate);
                        }

                        let is_scale = *current_tool.get() == ToolState::Size;
                        if ribbon_button(ui, Some(scale_tex), "Scale", is_scale).clicked() {
                            next_tool.set(ToolState::Size);
                        }

                        ui.add_space(8.0);
                        let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 56.0), egui::Sense::hover());
                        ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
                        ui.add_space(8.0);

                        if ribbon_button(ui, Some(add_tex), "Add", false).clicked() {
                            crate::studio::camera::spawn_brick(&mut commands, &mut meshes, &mut materials, &mut count);
                        }
                    });
                });

            let (bottom_sep, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().rect_filled(bottom_sep, 0.0, egui::Color32::from_rgb(180, 180, 180));
        });

    egui::SidePanel::left("explorer")
        .frame(egui::Frame::none()
            .fill(egui::Color32::from_rgb(245, 246, 247))
            .inner_margin(egui::Margin::symmetric(12, 12))
        )
        .default_width(220.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Explorer").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(16.0));
            });

            ui.add_space(8.0);
            let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
            ui.add_space(8.0);

            let mut baseplate_item = None;
            let mut parts_items = Vec::new();

            for (entity, name, brick_opt) in &entities_query {
                let name_str = name.as_str();
                if name_str == "Baseplate" {
                    baseplate_item = Some((entity, name_str.to_string()));
                } else if brick_opt.is_some() {
                    parts_items.push((entity, name_str.to_string()));
                }
            }

            parts_items.sort_by(|a, b| a.1.cmp(&b.1));

            egui::CollapsingHeader::new(egui::RichText::new("Workspace").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
                .default_open(true)
                .show(ui, |ui| {
                    if let Some((entity, name_str)) = baseplate_item {
                        let is_selected = selection.entity == Some(entity);
                        if custom_explorer_label(ui, is_selected, &name_str).clicked() {
                            selection.entity = Some(entity);
                        }
                    }

                    for (entity, name_str) in parts_items {
                        let is_selected = selection.entity == Some(entity);
                        if custom_explorer_label(ui, is_selected, &name_str).clicked() {
                            selection.entity = Some(entity);
                        }
                    }
                });

            let right_x = ui.max_rect().right() + 12.0;
            let top_y = ui.max_rect().top() - 12.0;
            let bottom_y = ui.max_rect().bottom() + 12.0;
            ui.painter().line_segment(
                [egui::pos2(right_x, top_y), egui::pos2(right_x, bottom_y)],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 180))
            );
        });

    if camera_indicator.visible_timer > 0.0 {
        let alpha_factor = if camera_indicator.visible_timer < 1.0 {
            camera_indicator.visible_timer.clamp(0.0, 1.0)
        } else {
            1.0
        };

        let mut inner_hovered = false;
        let mut slider_active = false;
        let area_response = egui::Area::new(egui::Id::new("camera_speed_indicator"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -40.0))
            .show(ctx, |ui| {
                ui.set_opacity(alpha_factor);
                let frame_res = egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(240, 240, 240, 230))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 180)))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut speed = camera_indicator.current_speed;
                            let slider_res = ui.add(
                                egui::Slider::new(&mut speed, 0.1..=100.0)
                                    .text("Camera Speed")
                            );
                            if slider_res.changed() {
                                if let Some((free_camera, mut free_camera_state)) = camera_query.iter_mut().next() {
                                    free_camera_state.speed_multiplier = speed / free_camera.walk_speed;
                                    camera_indicator.current_speed = speed;
                                }
                            }
                            slider_active = slider_res.dragged() || slider_res.has_focus() || slider_res.hovered();
                        });
                    });
                
                if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    if frame_res.response.rect.contains(pos) {
                        inner_hovered = true;
                    }
                }
            });

        if area_response.response.hovered() || inner_hovered || slider_active {
            camera_indicator.visible_timer = 2.0;
        }
    }
}

#[allow(deprecated)]
fn ribbon_button(
    ui: &mut egui::Ui,
    icon: Option<egui::TextureId>,
    label: &str,
    selected: bool,
) -> egui::Response {
    let size = egui::vec2(56.0, 56.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if selected {
        ui.painter().rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgb(204, 232, 255),
        );
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(153, 209, 255)),
            egui::StrokeKind::Inside,
        );
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgb(224, 238, 249),
        );
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 220, 240)),
            egui::StrokeKind::Inside,
        );
    }

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(7.0);
            if let Some(texture_id) = icon {
                ui.add(egui::Image::new((texture_id, egui::vec2(24.0, 24.0))));
            }
            ui.add_space(3.0);
            let text_color = egui::Color32::from_rgb(20, 20, 20);
            ui.label(egui::RichText::new(label).color(text_color).size(11.5));
        });
    });

    response
}

fn custom_explorer_label(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
) -> egui::Response {
    let size = egui::vec2(ui.available_width(), 20.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if selected {
        ui.painter().rect_filled(
            rect,
            2.0,
            egui::Color32::from_rgb(204, 232, 255),
        );
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(153, 209, 255)),
            egui::StrokeKind::Inside,
        );
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            2.0,
            egui::Color32::from_rgb(224, 238, 249),
        );
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 220, 240)),
            egui::StrokeKind::Inside,
        );
    }

    let text_color = if selected {
        egui::Color32::from_rgb(0, 0, 0)
    } else if response.hovered() {
        egui::Color32::from_rgb(20, 20, 20)
    } else {
        egui::Color32::from_rgb(60, 60, 60)
    };

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(label).color(text_color).size(13.5));
        });
    });

    response
}