use bevy::prelude::*;
use bevy_egui::egui;
use bevy::pbr::ExtendedMaterial;
use crate::common::game::bricks::components::Brick;

fn draw_coord_edit(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut f32,
    all_same: bool,
) -> Option<f32> {
    if !label.is_empty() {
        ui.label(label);
    }
    let id = ui.make_persistent_id(format!("coord_edit_{}", label));
    let mut text = ui.data_mut(|d| d.get_temp::<String>(id).unwrap_or_else(|| {
        if all_same { format!("{:.2}", val) } else { "—".to_string() }
    }));
    
    let res = ui.add(egui::TextEdit::singleline(&mut text).desired_width(45.0));
    if res.changed() {
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
        if let Ok(parsed) = text.parse::<f32>() {
            return Some(parsed);
        }
    } else if !res.has_focus() {
        let expected = if all_same { format!("{:.2}", val) } else { "—".to_string() };
        if text != expected {
            ui.data_mut(|d| d.insert_temp(id, expected));
        }
    }
    None
}

pub fn draw_properties(
    ui: &mut egui::Ui,
    selected_entities: &[Entity],
    properties_query: &mut Query<(
        Entity,
        &mut Transform,
        &mut Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&mut crate::common::game::bricks::components::BrickShapeComponent>,
        &GlobalTransform,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
        Option<&mut crate::common::game::bricks::components::BrickPhysics>,
    ), Without<Camera3d>>,
    brick_colors: &mut Query<&mut crate::common::game::bricks::components::BrickColor>,
    materials: &mut Assets<StandardMaterial>,
    studs_materials: &mut Assets<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>,
) {
    if selected_entities.is_empty() {
        return;
    }

    let first_entity = selected_entities[0];
    let (
        first_transform_val,
        first_name_str,
        first_shape_opt_val,
        first_color,
        is_extended,
        first_phys_enabled,
        first_bounciness,
        has_phys_opt,
    ) = {
        let Ok((_, first_transform, first_name, _, _, _, first_shape_opt, _, _, first_mat_opt, first_studs_mat_opt, first_phys_opt)) = properties_query.get(first_entity) else {
            return;
        };

        let first_name_str = first_name.to_string();
        let first_shape_opt_val = first_shape_opt.as_ref().map(|s| s.shape);

        let mut first_color = Color::srgb(0.84, 0.24, 0.16);
        let mut is_extended = false;
        if let Some(studs_mat_handle) = first_studs_mat_opt {
            if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                first_color = mat.base.base_color;
                is_extended = true;
            }
        } else if let Some(mat_handle) = first_mat_opt {
            if let Some(mat) = materials.get(&mat_handle.0) {
                first_color = mat.base_color;
            }
        }

        let (first_phys_enabled, first_bounciness) = if let Some(phys) = first_phys_opt {
            (phys.enabled, phys.bounciness)
        } else {
            (true, 0.3)
        };

        (
            *first_transform,
            first_name_str,
            first_shape_opt_val,
            first_color,
            is_extended,
            first_phys_enabled,
            first_bounciness,
            first_phys_opt.is_some(),
        )
    };

    let first_pos = first_transform_val.translation / 0.28;
    let first_scale = first_transform_val.scale;
    let first_rot = first_transform_val.rotation;
    let first_shape = first_shape_opt_val.unwrap_or(crate::common::game::bricks::components::BrickShape::Block);

    let mut all_names_same = true;
    let mut all_pos_x_same = true;
    let mut all_pos_y_same = true;
    let mut all_pos_z_same = true;
    let mut all_scale_x_same = true;
    let mut all_scale_y_same = true;
    let mut all_scale_z_same = true;
    let mut all_rot_same = true;
    let mut all_color_same = true;
    let mut all_shape_same = true;
    let mut all_phys_enabled_same = true;
    let mut all_bounciness_same = true;

    for &entity in &selected_entities[1..] {
        if let Ok((_, transform, name, _, _, _, shape_opt, _, _, mat_opt, studs_mat_opt, phys_opt)) = properties_query.get(entity) {
            if name.to_string() != first_name_str { all_names_same = false; }
            let pos = transform.translation / 0.28;
            if (pos.x - first_pos.x).abs() > 0.001 { all_pos_x_same = false; }
            if (pos.y - first_pos.y).abs() > 0.001 { all_pos_y_same = false; }
            if (pos.z - first_pos.z).abs() > 0.001 { all_pos_z_same = false; }
            
            let scale = transform.scale;
            if (scale.x - first_scale.x).abs() > 0.001 { all_scale_x_same = false; }
            if (scale.y - first_scale.y).abs() > 0.001 { all_scale_y_same = false; }
            if (scale.z - first_scale.z).abs() > 0.001 { all_scale_z_same = false; }
            
            let rot = transform.rotation;
            if rot.dot(first_rot).abs() < 0.999 { all_rot_same = false; }
            
            let shape = shape_opt.map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
            if shape != first_shape { all_shape_same = false; }
            
            let mut color = Color::srgb(0.84, 0.24, 0.16);
            if let Some(studs_mat_handle) = studs_mat_opt {
                if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                    color = mat.base.base_color;
                }
            } else if let Some(mat_handle) = mat_opt {
                if let Some(mat) = materials.get(&mat_handle.0) {
                    color = mat.base_color;
                }
            }
            if color != first_color { all_color_same = false; }
            
            let (phys_enabled, bounciness) = if let Some(phys) = phys_opt {
                (phys.enabled, phys.bounciness)
            } else {
                (true, 0.3)
            };
            if phys_enabled != first_phys_enabled { all_phys_enabled_same = false; }
            if (bounciness - first_bounciness).abs() > 0.001 { all_bounciness_same = false; }
        }
    }

    let mut color_array = [
        first_color.to_srgba().red,
        first_color.to_srgba().green,
        first_color.to_srgba().blue,
        first_color.to_srgba().alpha,
    ];

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Properties").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(16.0));
        });

        ui.add_space(8.0);
        let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
        ui.add_space(8.0);

        egui::CollapsingHeader::new(egui::RichText::new("Information").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("properties_info_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Name").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        let name_id = ui.make_persistent_id("properties_name_input");
                        let mut name_str = ui.data_mut(|d| d.get_temp::<String>(name_id).unwrap_or_else(|| {
                            if all_names_same { first_name_str.clone() } else { "".to_string() }
                        }));
                        let placeholder = if all_names_same { "" } else { "Mixed" };
                        let res = ui.add(egui::TextEdit::singleline(&mut name_str).hint_text(placeholder));
                        if res.changed() {
                            ui.data_mut(|d| d.insert_temp(name_id, name_str.clone()));
                            for &entity in selected_entities {
                                if let Ok((_, _, mut name, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    *name = Name::new(name_str.clone());
                                }
                            }
                        } else if !res.has_focus() {
                            let current_expected = if all_names_same { first_name_str.clone() } else { "".to_string() };
                            if name_str != current_expected {
                                name_str = current_expected;
                                ui.data_mut(|d| d.insert_temp(name_id, name_str.clone()));
                            }
                        }
                        ui.end_row();
                    });
            });

        ui.add_space(8.0);

        egui::CollapsingHeader::new(egui::RichText::new("Transform").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("properties_transform_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Position").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        let mut pos_studs = first_pos;
                        let mut new_px = None;
                        let mut new_py = None;
                        let mut new_pz = None;
                        ui.horizontal(|ui| {
                            new_px = draw_coord_edit(ui, "X", &mut pos_studs.x, all_pos_x_same);
                            new_py = draw_coord_edit(ui, "Y", &mut pos_studs.y, all_pos_y_same);
                            new_pz = draw_coord_edit(ui, "Z", &mut pos_studs.z, all_pos_z_same);
                        });
                        if new_px.is_some() || new_py.is_some() || new_pz.is_some() {
                            for &entity in selected_entities {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    if let Some(x) = new_px { transform.translation.x = x * 0.28; }
                                    if let Some(y) = new_py { transform.translation.y = y * 0.28; }
                                    if let Some(z) = new_pz { transform.translation.z = z * 0.28; }
                                }
                            }
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Size").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        let mut scale_val = first_scale;
                        let mut new_sx = None;
                        let mut new_sy = None;
                        let mut new_sz = None;
                        ui.horizontal(|ui| {
                            new_sx = draw_coord_edit(ui, "X", &mut scale_val.x, all_scale_x_same);
                            new_sy = draw_coord_edit(ui, "Y", &mut scale_val.y, all_scale_y_same);
                            new_sz = draw_coord_edit(ui, "Z", &mut scale_val.z, all_scale_z_same);
                        });
                        if new_sx.is_some() || new_sy.is_some() || new_sz.is_some() {
                            for &entity in selected_entities {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    if let Some(x) = new_sx { transform.scale.x = x.max(0.01); }
                                    if let Some(y) = new_sy { transform.scale.y = y.max(0.01); }
                                    if let Some(z) = new_sz { transform.scale.z = z.max(0.01); }
                                }
                            }
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Rotation").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        let (rx, ry, rz) = first_rot.to_euler(EulerRot::XYZ);
                        let mut rot_deg = Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees());
                        let mut new_rx = None;
                        let mut new_ry = None;
                        let mut new_rz = None;
                        ui.horizontal(|ui| {
                            new_rx = draw_coord_edit(ui, "X", &mut rot_deg.x, all_rot_same);
                            new_ry = draw_coord_edit(ui, "Y", &mut rot_deg.y, all_rot_same);
                            new_rz = draw_coord_edit(ui, "Z", &mut rot_deg.z, all_rot_same);
                        });
                        if new_rx.is_some() || new_ry.is_some() || new_rz.is_some() {
                            let rx_val = new_rx.unwrap_or(rot_deg.x).to_radians();
                            let ry_val = new_ry.unwrap_or(rot_deg.y).to_radians();
                            let rz_val = new_rz.unwrap_or(rot_deg.z).to_radians();
                            for &entity in selected_entities {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    transform.rotation = Quat::from_euler(EulerRot::XYZ, rx_val, ry_val, rz_val);
                                }
                            }
                        }
                        ui.end_row();
                    });
            });

        ui.add_space(8.0);

        egui::CollapsingHeader::new(egui::RichText::new("Appearance").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("properties_appearance_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Color").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        ui.horizontal(|ui| {
                            let color_btn = ui.color_edit_button_rgba_unmultiplied(&mut color_array);
                            if !all_color_same {
                                ui.label(egui::RichText::new("Mixed").italics().color(egui::Color32::from_rgb(120, 120, 120)));
                            }
                            if color_btn.changed() {
                                let new_color = Color::Srgba(Srgba::new(color_array[0], color_array[1], color_array[2], color_array[3]));
                                for &entity in selected_entities {
                                    if let Ok((_, _, _, _, _, _, _, _, _, mat_opt, studs_mat_opt, _)) = properties_query.get_mut(entity) {
                                        if is_extended {
                                            if let Some(studs_mat_handle) = studs_mat_opt {
                                                if let Some(mut mat) = studs_materials.get_mut(&studs_mat_handle.0) {
                                                    mat.base.base_color = new_color;
                                                }
                                            }
                                        } else {
                                            if let Some(mat_handle) = mat_opt {
                                                if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                                                    mat.base_color = new_color;
                                                }
                                            }
                                        }
                                        if let Ok(mut bc) = brick_colors.get_mut(entity) {
                                            bc.color = new_color;
                                        }
                                    }
                                }
                            }
                        });
                        ui.end_row();

                        if first_shape_opt_val.is_some() {
                            ui.label(egui::RichText::new("Shape").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut current_shape = first_shape;
                            let combo_label = if all_shape_same { format!("{:?}", current_shape) } else { "Mixed".to_string() };
                            let mut selection_changed = false;
                            egui::ComboBox::from_id_salt("brick_shape_select")
                                .selected_text(combo_label)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_value(&mut current_shape, crate::common::game::bricks::components::BrickShape::Block, "Block").clicked() {
                                        selection_changed = true;
                                    }
                                    if ui.selectable_value(&mut current_shape, crate::common::game::bricks::components::BrickShape::Sphere, "Sphere").clicked() {
                                        selection_changed = true;
                                    }
                                });
                            if selection_changed {
                                for &entity in selected_entities {
                                    if let Ok((_, _, _, _, _, _, Some(mut shape_comp), _, _, _, _, _)) = properties_query.get_mut(entity) {
                                        shape_comp.shape = current_shape;
                                    }
                                }
                            }
                            ui.end_row();
                        }
                    });
            });

        ui.add_space(8.0);

        egui::CollapsingHeader::new(egui::RichText::new("Physics").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("properties_physics_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        if has_phys_opt {
                            ui.label(egui::RichText::new("Physics Enabled").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            
                            let mut enabled = first_phys_enabled;
                            let checkbox_res = if all_phys_enabled_same {
                                Some(ui.checkbox(&mut enabled, ""))
                            } else {
                                let mut clicked = false;
                                ui.horizontal(|ui| {
                                    if ui.button("Mixed (Click to set)").clicked() {
                                        clicked = true;
                                        enabled = true;
                                    }
                                });
                                if clicked {
                                    for &entity in selected_entities {
                                        if let Ok((_, _, _, _, _, _, _, _, _, _, _, Some(mut phys))) = properties_query.get_mut(entity) {
                                            phys.enabled = enabled;
                                        }
                                    }
                                }
                                None
                            };
                            if let Some(res) = checkbox_res {
                                if res.changed() {
                                    for &entity in selected_entities {
                                        if let Ok((_, _, _, _, _, _, _, _, _, _, _, Some(mut phys))) = properties_query.get_mut(entity) {
                                            phys.enabled = enabled;
                                        }
                                    }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Bounciness").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut bounciness_val = first_bounciness;
                            let mut new_bounciness = None;
                            ui.horizontal(|ui| {
                                new_bounciness = draw_coord_edit(ui, "", &mut bounciness_val, all_bounciness_same);
                            });
                            
                            if let Some(new_val) = new_bounciness {
                                for &entity in selected_entities {
                                    if let Ok((_, _, _, _, _, _, _, _, _, _, _, Some(mut phys))) = properties_query.get_mut(entity) {
                                        phys.bounciness = new_val.clamp(0.0, 1.0);
                                    }
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
    });
}

pub fn draw_workspace_properties(
    ui: &mut egui::Ui,
    gravity: &mut Option<ResMut<'_, avian3d::prelude::Gravity>>,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Properties").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(16.0));
    });

    ui.add_space(8.0);
    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
    ui.add_space(8.0);

    egui::CollapsingHeader::new(egui::RichText::new("Physics").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("properties_workspace_physics_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("World Gravity").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                    if let Some(g) = gravity {
                        let mut gravity_studs = -g.0.y / 0.28;
                        if ui.add(egui::DragValue::new(&mut gravity_studs).speed(1.0).range(0.0..=10000.0).suffix(" studs/s²")).changed() {
                            g.0 = Vec3::new(0.0, -gravity_studs * 0.28, 0.0);
                        }
                    } else {
                        ui.label(egui::RichText::new("Gravity resource not found").color(egui::Color32::from_rgb(180, 60, 60)).size(13.0));
                    }
                    ui.end_row();
                });
        });
}

pub fn draw_players_properties(
    ui: &mut egui::Ui,
    players_service: &mut Option<ResMut<'_, crate::studio::tools::PlayersService>>,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Properties").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(16.0));
    });

    ui.add_space(8.0);
    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
    ui.add_space(8.0);

    egui::CollapsingHeader::new(egui::RichText::new("Basic").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("properties_players_basic_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Player Speed").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                    if let Some(service) = players_service {
                        let mut speed_studs = service.speed / 0.28;
                        if ui.add(egui::DragValue::new(&mut speed_studs).speed(0.5).range(0.0..=1000.0).suffix(" studs/s")).changed() {
                            service.speed = speed_studs * 0.28;
                        }
                    } else {
                        ui.label(egui::RichText::new("Players resource not found").color(egui::Color32::from_rgb(180, 60, 60)).size(13.0));
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Player Jump Power").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                    if let Some(service) = players_service {
                        let mut jump_power_studs = service.jump_power / 0.28;
                        if ui.add(egui::DragValue::new(&mut jump_power_studs).speed(1.0).range(0.0..=1000.0).suffix(" studs/s")).changed() {
                            service.jump_power = jump_power_studs * 0.28;
                        }
                    } else {
                        ui.label(egui::RichText::new("Players resource not found").color(egui::Color32::from_rgb(180, 60, 60)).size(13.0));
                    }
                    ui.end_row();
                });
        });
}