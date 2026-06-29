use bevy::prelude::*;
use bevy_egui::egui;
use crate::studio::tools::{ToolState, Selection};
use crate::common::components::Brick;
use crate::studio::ui::CopiedEntityBuffer;

#[allow(deprecated)]
pub fn draw_top_bar(
    ui: &mut egui::Ui,
    next_tool: &mut ResMut<NextState<ToolState>>,
    current_tool: &Res<State<ToolState>>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    count: &mut ResMut<crate::studio::camera::BrickSpawnerCount>,
    move_tex: egui::TextureId,
    rotate_tex: egui::TextureId,
    scale_tex: egui::TextureId,
    add_tex: egui::TextureId,
    diagnostics: &Res<bevy::diagnostic::DiagnosticsStore>,
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
                            ui.label("🔍");
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
                                    crate::studio::camera::spawn_brick(commands, meshes, materials, count);
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

pub fn draw_explorer(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    selection: &mut ResMut<Selection>,
    entitiesquery: &Query<(Entity, &Name, Option<&Brick>)>,
    copiedbuffer: &mut CopiedEntityBuffer,
    fullentityquery: &Query<(
        &Transform,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Name,
        Option<&Brick>,
    )>,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Explorer").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(16.0));
    });

    ui.add_space(8.0);
    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
    ui.add_space(8.0);

    let mut baseplateitem = None;
    let mut partsitems = Vec::new();

    for (entity, name, brick_opt) in entitiesquery {
        let name_str = name.as_str();
        if name_str == "Baseplate" {
            baseplateitem = Some((entity, name_str.to_string()));
        } else if brick_opt.is_some() {
            partsitems.push((entity, name_str.to_string()));
        }
    }

    partsitems.sort_by(|a, b| a.1.cmp(&b.1));

    egui::CollapsingHeader::new(egui::RichText::new("Workspace").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
        .default_open(true)
        .show(ui, |ui| {
            if let Some((entity, name_str)) = baseplateitem {
                let is_selected = selection.entity == Some(entity);
                let labelres = explorerlabel(ui, is_selected, &name_str);
                if labelres.clicked() {
                    selection.entity = Some(entity);
                }
                labelres.context_menu(|ui| {
                    if ui.button("Copy").clicked() {
                        if let Ok((transform, mesh, material, name, brick_opt)) = fullentityquery.get(entity) {
                            copiedbuffer.transform = Some(*transform);
                            copiedbuffer.mesh = Some(mesh.clone());
                            copiedbuffer.material = Some(material.clone());
                            copiedbuffer.name = Some(name.to_string());
                            copiedbuffer.is_brick = brick_opt.is_some();
                        }
                        ui.close();
                    }
                    if copiedbuffer.transform.is_some() {
                        if ui.button("Paste").clicked() {
                            let transform = copiedbuffer.transform.unwrap();
                            let mesh = copiedbuffer.mesh.clone().unwrap();
                            let material = copiedbuffer.material.clone().unwrap();
                            let name = copiedbuffer.name.clone().unwrap();
                            let mut newtransform = transform;
                            newtransform.translation += Vec3::new(2.0, 0.0, 2.0);
                            let mut spawned = commands.spawn((
                                newtransform,
                                mesh,
                                material,
                                Name::new(format!("{} - Copy", name)),
                                Pickable::default(),
                            ));
                            if copiedbuffer.is_brick {
                                spawned.insert(Brick);
                            }
                            ui.close();
                        }
                    }
                    if ui.button("Duplicate").clicked() {
                        if let Ok((transform, mesh, material, name, brick_opt)) = fullentityquery.get(entity) {
                            let newtransform = *transform;
                            let mut spawned = commands.spawn((
                                newtransform,
                                mesh.clone(),
                                material.clone(),
                                Name::new(format!("{} - Copy", name.as_str())),
                                Pickable::default(),
                            ));
                            if brick_opt.is_some() {
                                spawned.insert(Brick);
                            }
                            ui.close();
                        }
                    }
                    if ui.button("Delete").clicked() {
                        commands.entity(entity).despawn();
                        if selection.entity == Some(entity) {
                            selection.entity = None;
                        }
                        ui.close();
                    }
                });
            }

            for (entity, name_str) in partsitems {
                let is_selected = selection.entity == Some(entity);
                let labelres = explorerlabel(ui, is_selected, &name_str);
                if labelres.clicked() {
                    selection.entity = Some(entity);
                }
                labelres.context_menu(|ui| {
                    if ui.button("Copy").clicked() {
                        if let Ok((transform, mesh, material, name, brick_opt)) = fullentityquery.get(entity) {
                            copiedbuffer.transform = Some(*transform);
                            copiedbuffer.mesh = Some(mesh.clone());
                            copiedbuffer.material = Some(material.clone());
                            copiedbuffer.name = Some(name.to_string());
                            copiedbuffer.is_brick = brick_opt.is_some();
                        }
                        ui.close();
                    }
                    if copiedbuffer.transform.is_some() {
                        if ui.button("Paste").clicked() {
                            let transform = copiedbuffer.transform.unwrap();
                            let mesh = copiedbuffer.mesh.clone().unwrap();
                            let material = copiedbuffer.material.clone().unwrap();
                            let name = copiedbuffer.name.clone().unwrap();
                            let mut newtransform = transform;
                            newtransform.translation += Vec3::new(2.0, 0.0, 2.0);
                            let mut spawned = commands.spawn((
                                newtransform,
                                mesh,
                                material,
                                Name::new(format!("{} - Copy", name)),
                                Pickable::default(),
                            ));
                            if copiedbuffer.is_brick {
                                spawned.insert(Brick);
                            }
                            ui.close();
                        }
                    }
                    if ui.button("Duplicate").clicked() {
                        if let Ok((transform, mesh, material, name, brick_opt)) = fullentityquery.get(entity) {
                            let newtransform = *transform;
                            let mut spawned = commands.spawn((
                                newtransform,
                                mesh.clone(),
                                material.clone(),
                                Name::new(format!("{} - Copy", name.as_str())),
                                Pickable::default(),
                            ));
                            if brick_opt.is_some() {
                                spawned.insert(Brick);
                            }
                            ui.close();
                        }
                    }
                    if ui.button("Delete").clicked() {
                        commands.entity(entity).despawn();
                        if selection.entity == Some(entity) {
                            selection.entity = None;
                        }
                        ui.close();
                    }
                });
            }
        });

    let right_x = ui.max_rect().right() + 12.0;
    let top_y = ui.max_rect().top() - 12.0;
    let bottom_y = ui.max_rect().bottom() + 12.0;
    ui.painter().line_segment(
        [egui::pos2(right_x, top_y), egui::pos2(right_x, bottom_y)],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 180))
    );
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

fn explorerlabel(
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
            ui.add(egui::Label::new(egui::RichText::new(label).color(text_color).size(13.5)).selectable(false));
        });
    });

    response
}