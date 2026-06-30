use bevy::prelude::*;
use bevy_egui::egui;
use bevy::pbr::ExtendedMaterial;
use crate::common::components::Brick;

pub fn draw_properties(
    ui: &mut egui::Ui,
    selected_entity: Entity,
    properties_query: &mut Query<(
        Entity,
        &mut Transform,
        &mut Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        &GlobalTransform,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>>,
    ), Without<Camera3d>>,
    materials: &mut Assets<StandardMaterial>,
    studs_materials: &mut Assets<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>,
) {
    let Ok((_, mut transform, mut name, _, _, Some(_brick), _, _, mat_opt, studs_mat_opt)) = properties_query.get_mut(selected_entity) else {
        return;
    };

    let mut current_color = None;
    let mut is_extended = false;

    if let Some(studs_mat_handle) = studs_mat_opt {
        if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
            current_color = Some(mat.base.base_color.to_srgba());
            is_extended = true;
        }
    } else if let Some(mat_handle) = mat_opt {
        if let Some(mat) = materials.get(&mat_handle.0) {
            current_color = Some(mat.base_color.to_srgba());
            is_extended = false;
        }
    }

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
                    let mut name_str = ui.data_mut(|d| d.get_temp::<String>(name_id).unwrap_or_else(|| name.to_string()));
                    let res = ui.text_edit_singleline(&mut name_str);
                    if res.changed() {
                        ui.data_mut(|d| d.insert_temp(name_id, name_str.clone()));
                        *name = Name::new(name_str.clone());
                    } else if !res.has_focus() {
                        let current_name = name.to_string();
                        if name_str != current_name {
                            name_str = current_name;
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
                    let mut pos_studs = transform.translation / 0.28;
                    let mut pos_changed = false;
                    ui.horizontal(|ui| {
                        ui.label("X");
                        pos_changed |= ui.add(egui::DragValue::new(&mut pos_studs.x).speed(0.1)).changed();
                        ui.label("Y");
                        pos_changed |= ui.add(egui::DragValue::new(&mut pos_studs.y).speed(0.1)).changed();
                        ui.label("Z");
                        pos_changed |= ui.add(egui::DragValue::new(&mut pos_studs.z).speed(0.1)).changed();
                    });
                    if pos_changed {
                        transform.translation = pos_studs * 0.28;
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Size").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                    let mut size_studs = transform.scale * Vec3::new(4.0, 1.0, 2.0);
                    let mut size_changed = false;
                    ui.horizontal(|ui| {
                        ui.label("X");
                        size_changed |= ui.add(egui::DragValue::new(&mut size_studs.x).speed(0.1).range(0.1..=1000.0)).changed();
                        ui.label("Y");
                        size_changed |= ui.add(egui::DragValue::new(&mut size_studs.y).speed(0.1).range(0.1..=1000.0)).changed();
                        size_changed |= ui.add(egui::DragValue::new(&mut size_studs.z).speed(0.1).range(0.1..=1000.0)).changed();
                    });
                    if size_changed {
                        size_studs = size_studs.max(Vec3::splat(0.1));
                        transform.scale = size_studs / Vec3::new(4.0, 1.0, 2.0);
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Rotation").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                    let (rx, ry, rz) = transform.rotation.to_euler(EulerRot::XYZ);
                    let mut rot_deg = Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees());
                    let mut rot_changed = false;
                    ui.horizontal(|ui| {
                        ui.label("X");
                        rot_changed |= ui.add(egui::DragValue::new(&mut rot_deg.x).speed(1.0)).changed();
                        ui.label("Y");
                        rot_changed |= ui.add(egui::DragValue::new(&mut rot_deg.y).speed(1.0)).changed();
                        ui.label("Z");
                        rot_changed |= ui.add(egui::DragValue::new(&mut rot_deg.z).speed(1.0)).changed();
                    });
                    if rot_changed {
                        transform.rotation = Quat::from_euler(
                            EulerRot::XYZ,
                            rot_deg.x.to_radians(),
                            rot_deg.y.to_radians(),
                            rot_deg.z.to_radians(),
                        );
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
                    if let Some(srgba) = current_color {
                        ui.label(egui::RichText::new("Color").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                        let mut color_array = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                        if ui.color_edit_button_rgba_unmultiplied(&mut color_array).changed() {
                            let new_color = Color::Srgba(Srgba::new(color_array[0], color_array[1], color_array[2], color_array[3]));
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
                        }
                        ui.end_row();
                    }
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