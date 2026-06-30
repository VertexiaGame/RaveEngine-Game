use bevy::prelude::*;
use bevy_egui::egui;
use crate::studio::tools::{ToolState, SnapConfig};
use crate::studio::bricks::spawn_brick;
use crate::studio::studs::StudsAssets;
use crate::studio::bricks::BrickSpawnerCount;
use bevy::pbr::ExtendedMaterial;

#[allow(deprecated)]
pub fn draw_top_bar(
    ui: &mut egui::Ui,
    next_tool: &mut ResMut<NextState<ToolState>>,
    current_tool: &Res<State<ToolState>>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    studs_materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>>,
    studs_assets: &StudsAssets,
    count: &mut ResMut<BrickSpawnerCount>,
    snap_config: &mut ResMut<SnapConfig>,
    move_tex: egui::TextureId,
    rotate_tex: egui::TextureId,
    scale_tex: egui::TextureId,
    add_tex: egui::TextureId,
    diagnostics: &Res<bevy::diagnostic::DiagnosticsStore>,
    camera_transform: Option<&Transform>,
    _action_writer: &mut MessageWriter<crate::studio::tools::UndoRedoAction>,
    history: &mut ResMut<crate::studio::tools::UndoRedoHistory>,
    physics_state: crate::common::physics::PhysicsSimulationState,
    physics_action_writer: &mut MessageWriter<crate::common::physics::PhysicsSimulationAction>,
    settings_window: &mut ResMut<crate::studio::ui::SettingsWindow>,
) {
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

                let settings_id = ui.make_persistent_id("settings_menu_btn");
                let is_hovered = ui.data(|d| d.get_temp::<bool>(settings_id)).unwrap_or(false);
                let text_color = if is_hovered {
                    egui::Color32::from_rgb(80, 160, 240)
                } else {
                    egui::Color32::from_rgb(60, 60, 60)
                };

                let settings_btn = ui.add(egui::Label::new(egui::RichText::new("Settings").color(text_color).size(13.0)).sense(egui::Sense::click()));
                ui.data_mut(|d| d.insert_temp(settings_id, settings_btn.hovered()));

                if settings_btn.clicked() {
                    settings_window.open = !settings_window.open;
                }

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
                if ribbonbutton(ui, Some(move_tex), "Move", is_move).clicked() {
                    if is_move {
                        next_tool.set(ToolState::None);
                    } else {
                        next_tool.set(ToolState::Move);
                    }
                }

                let is_rotate = *current_tool.get() == ToolState::Rotate;
                if ribbonbutton(ui, Some(rotate_tex), "Rotate", is_rotate).clicked() {
                    if is_rotate {
                        next_tool.set(ToolState::None);
                    } else {
                        next_tool.set(ToolState::Rotate);
                    }
                }

                let is_scale = *current_tool.get() == ToolState::Size;
                if ribbonbutton(ui, Some(scale_tex), "Scale", is_scale).clicked() {
                    if is_scale {
                        next_tool.set(ToolState::None);
                    } else {
                        next_tool.set(ToolState::Size);
                    }
                }

                ui.add_space(8.0);
                let (sep_rect_phys, _) = ui.allocate_exact_size(egui::vec2(1.0, 56.0), egui::Sense::hover());
                ui.painter().rect_filled(sep_rect_phys, 0.0, egui::Color32::from_rgb(212, 212, 212));
                ui.add_space(8.0);

                match physics_state {
                    crate::common::physics::PhysicsSimulationState::Stopped => {
                        if ribbonbutton(ui, None, "Play", false).clicked() {
                            physics_action_writer.write(crate::common::physics::PhysicsSimulationAction::Play);
                        }
                    }
                    crate::common::physics::PhysicsSimulationState::Running => {
                        if ribbonbutton(ui, None, "Stop", false).clicked() {
                            physics_action_writer.write(crate::common::physics::PhysicsSimulationAction::Stop);
                        }
                        if ribbonbutton(ui, None, "Replay", false).clicked() {
                            physics_action_writer.write(crate::common::physics::PhysicsSimulationAction::Replay);
                        }
                    }
                }

                ui.add_space(8.0);
                let (sep_rect_snap, _) = ui.allocate_exact_size(egui::vec2(1.0, 56.0), egui::Sense::hover());
                ui.painter().rect_filled(sep_rect_snap, 0.0, egui::Color32::from_rgb(212, 212, 212));
                ui.add_space(8.0);

                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    let mut enabled = snap_config.enabled;
                    if ui.checkbox(&mut enabled, "Snap").changed() {
                        snap_config.enabled = enabled;
                        if enabled {
                            snap_config.distance = 1.0;
                        }
                    }
                    if snap_config.enabled {
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut snap_config.distance)
                                .speed(0.1)
                                .range(0.01..=1000.0)
                                .suffix(" stud")
                            );
                        });
                    }
                });

                ui.add_space(8.0);
                let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 56.0), egui::Sense::hover());
                ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
                ui.add_space(8.0);

                let add_btn = ribbonbutton(ui, Some(add_tex), "Add", false);
                let popup_id = ui.make_persistent_id("add_part_popup");
                if add_btn.clicked() {
                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                }

                let mut search_query = ui.data_mut(|d| d.get_temp::<String>(popup_id).unwrap_or_default());

                let original_window_fill = ui.visuals().window_fill;
                let original_window_stroke = ui.visuals().window_stroke;

                ui.visuals_mut().window_fill = egui::Color32::from_rgb(255, 255, 255);
                ui.visuals_mut().window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(212, 212, 212));

                let _: Option<()> = egui::popup_below_widget(
                    ui,
                    popup_id,
                    &add_btn,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui: &mut egui::Ui| {
                        ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(224, 238, 249);
                        ui.visuals_mut().widgets.active.bg_fill = egui::Color32::from_rgb(204, 232, 255);
                        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(255, 255, 255);
                        ui.visuals_mut().widgets.noninteractive.bg_fill = egui::Color32::from_rgb(255, 255, 255);

                        ui.set_min_width(150.0);
                        ui.horizontal(|ui| {
                            ui.label("🔍"); //KEEP THIS!! It looks kinda good ngl
                            let text_edit_res = ui.text_edit_singleline(&mut search_query);
                            if text_edit_res.changed() {
                                ui.data_mut(|d| d.insert_temp(popup_id, search_query.clone()));
                            }
                        });
                        ui.separator();

                        let items = ["Part"];
                        for item in items {
                            if item.to_lowercase().contains(&search_query.to_lowercase()) {
                                if ui.button(item).clicked() {
                                    let mut spawn_pos = Vec3::new(0.0, 0.14, 0.0);
                                    if let Some(cam_t) = camera_transform {
                                        let camera_pos = cam_t.translation;
                                        let camera_forward = cam_t.forward();

                                        let mut found_hit = false;
                                        if camera_forward.y.abs() > 0.001 {
                                            let resting_y = 0.5 * 0.28;
                                            let t = (resting_y - camera_pos.y) / camera_forward.y;
                                            if t > 0.0 && t < 100.0 {
                                                let hit_pos = camera_pos + camera_forward * t;
                                                if hit_pos.x.abs() <= 25.0 && hit_pos.z.abs() <= 25.0 {
                                                    spawn_pos = hit_pos;
                                                    found_hit = true;
                                                }
                                            }
                                        }
                                        if !found_hit {
                                            spawn_pos = camera_pos + camera_forward * (10.0 * 0.28);
                                        }
                                    }

                                    if snap_config.enabled && snap_config.distance > 0.0 {
                                        let snap_interval = snap_config.distance * 0.28;
                                        spawn_pos.x = (spawn_pos.x / snap_interval).round() * snap_interval;
                                        spawn_pos.z = (spawn_pos.z / snap_interval).round() * snap_interval;
                                        spawn_pos.y = (spawn_pos.y / snap_interval).round() * snap_interval;
                                        if spawn_pos.y < (0.5 * 0.28) {
                                            spawn_pos.y = 0.5 * 0.28;
                                        }
                                    }

                                    let new_entity = spawn_brick(commands, meshes, studs_materials, studs_assets, count, spawn_pos);

                                    let data = crate::studio::tools::BrickData {
                                        transform: Transform::from_translation(spawn_pos),
                                        name: format!("Part{}", count.count - 1),
                                        is_brick: true,
                                        mesh: Some(Mesh3d(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)))),
                                        standard_material: None,
                                        studs_material: Some(MeshMaterial3d(studs_materials.add(ExtendedMaterial {
                                            base: StandardMaterial {
                                                base_color: Color::srgb(0.84, 0.24, 0.16),
                                                perceptual_roughness: 0.9,
                                                ..default()
                                            },
                                            extension: crate::studio::studs::StudsExtension {
                                                stud_texture: studs_assets.stud.clone(),
                                                inlet_texture: studs_assets.inlet.clone(),
                                            },
                                        }))),
                                        parent: None,
                                    };

                                    history.push_command(crate::studio::tools::UndoCommand::Spawn {
                                        entity: new_entity,
                                        data,
                                    });

                                    ui.memory_mut(|mem| mem.close_popup(popup_id));
                                }
                            }
                        }
                    },
                );

                ui.visuals_mut().window_fill = original_window_fill;
                ui.visuals_mut().window_stroke = original_window_stroke;
            });
        });

    let (bottom_sep, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(bottom_sep, 0.0, egui::Color32::from_rgb(180, 180, 180));
}

#[allow(deprecated)]
fn ribbonbutton(
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
            if let Some(texture_id) = icon {
                ui.add_space(7.0);
                ui.add(egui::Image::new((texture_id, egui::vec2(24.0, 24.0))));
                ui.add_space(3.0);
            } else {
                ui.add_space(18.0);
            }
            let text_color = egui::Color32::from_rgb(20, 20, 20);
            ui.label(egui::RichText::new(label).color(text_color).size(11.5));
        });
    });

    response
}