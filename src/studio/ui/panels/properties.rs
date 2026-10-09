use crate::common::game::bricks::components::Brick;
use avian3d::prelude::{CollisionLayers, Friction, GravityScale, Mass, Restitution, RigidBody};
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy_egui::egui;

fn current_brick_color(
    entity: Entity,
    brick_colors: &Query<&mut crate::common::game::bricks::components::BrickColor>,
    materials: &Assets<
        ExtendedMaterial<
            StandardMaterial,
            crate::common::game::bricks::studs::ShadowOpacityExtension,
        >,
    >,
    studs_materials: &Assets<
        ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>,
    >,
    mat_opt: Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::ShadowOpacityExtension>>>,
    studs_mat_opt: Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
) -> Color {
    if let Ok(bc) = brick_colors.get(entity) {
        bc.color
    } else if let Some(studs_mat_handle) = studs_mat_opt {
        studs_materials
            .get(&studs_mat_handle.0)
            .map(|mat| mat.base.base_color)
            .unwrap_or(Color::srgb(0.84, 0.24, 0.16))
    } else if let Some(mat_handle) = mat_opt {
        materials
            .get(&mat_handle.0)
            .map(|mat| mat.base.base_color)
            .unwrap_or(Color::srgb(0.84, 0.24, 0.16))
    } else {
        Color::srgb(0.84, 0.24, 0.16)
    }
}

fn write_brick_color(
    commands: &mut Commands,
    brick_colors: &mut Query<&mut crate::common::game::bricks::components::BrickColor>,
    entity: Entity,
    color: Color,
) {
    if let Ok(mut bc) = brick_colors.get_mut(entity) {
        bc.color = color;
    } else {
        commands
            .entity(entity)
            .insert(crate::common::game::bricks::components::BrickColor { color });
    }
}

fn draw_coord_edit(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut f32,
    all_same: bool,
    unique_key: &str,
) -> Option<f32> {
    if !label.is_empty() {
        ui.label(label);
    }
    let id = ui.make_persistent_id(unique_key);
    let mut text = ui.data_mut(|d| {
        d.get_temp::<String>(id).unwrap_or_else(|| {
            if all_same {
                format!("{:.2}", val)
            } else {
                "—".to_string()
            }
        })
    });

    let res = ui.add(egui::TextEdit::singleline(&mut text).desired_width(45.0));
    if res.changed() {
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
        if let Ok(parsed) = text.parse::<f32>() {
            return Some(parsed);
        }
    } else if !res.has_focus() {
        let expected = if all_same {
            format!("{:.2}", val)
        } else {
            "—".to_string()
        };
        if text != expected {
            ui.data_mut(|d| d.insert_temp(id, expected));
        }
    }
    None
}

fn asset_status_notice(
    asset_id: u32,
    asset_status: &crate::common::game::assets::status::AssetStatusCache,
    current_uid: Option<i32>,
    kind: &str,
) -> Option<(String, egui::Color32, f32)> {
    if asset_id == 0 {
        return None;
    }
    match asset_status.statuses.get(&asset_id) {
        Some(crate::common::game::assets::status::AssetStatus::Pending) => {
            let is_owner = match (asset_status.owner_uids.get(&asset_id), current_uid) {
                (Some(owner), Some(current)) => *owner == current,
                _ => false,
            };
            if is_owner {
                Some((
                    "This asset is pending, but because you are the creator, you are able to see it."
                        .to_string(),
                    egui::Color32::from_rgb(160, 110, 0),
                    11.0,
                ))
            } else {
                Some((
                    "The asset is pending and has not been approved yet.".to_string(),
                    egui::Color32::from_rgb(160, 110, 0),
                    12.0,
                ))
            }
        }
        Some(crate::common::game::assets::status::AssetStatus::Rejected) => Some((
            "The asset has been declined and cannot be used.".to_string(),
            egui::Color32::from_rgb(180, 60, 60),
            12.0,
        )),
        Some(crate::common::game::assets::status::AssetStatus::NotFound) => Some((
            format!("{kind} not found."),
            egui::Color32::from_rgb(180, 60, 60),
            12.0,
        )),
        _ => None,
    }
}

fn draw_asset_status_row(
    ui: &mut egui::Ui,
    asset_id: u32,
    asset_status: &crate::common::game::assets::status::AssetStatusCache,
    current_uid: Option<i32>,
    kind: &str,
) {
    if let Some((text, color, size)) = asset_status_notice(asset_id, asset_status, current_uid, kind)
    {
        ui.label("");
        ui.label(egui::RichText::new(text).color(color).size(size));
        ui.end_row();
    }
}

pub fn draw_properties(
    ui: &mut egui::Ui,
    selected_entities: &[Entity],
    commands: &mut Commands,
    properties_query: &mut Query<
        (
            Entity,
            &mut Transform,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&Brick>,
            Option<&mut crate::common::game::bricks::components::BrickShapeComponent>,
            &GlobalTransform,
            Option<&Mesh3d>,
            Option<
                &MeshMaterial3d<
                    ExtendedMaterial<
                        StandardMaterial,
                        crate::common::game::bricks::studs::ShadowOpacityExtension,
                    >,
                >,
            >,
            Option<
                &MeshMaterial3d<
                    ExtendedMaterial<
                        StandardMaterial,
                        crate::common::game::bricks::studs::StudsExtension,
                    >,
                >,
            >,
            Option<&mut crate::common::game::bricks::components::BrickPhysics>,
        ),
        Without<Camera3d>,
    >,
    brick_colors: &mut Query<&mut crate::common::game::bricks::components::BrickColor>,
    materials: &mut Assets<
        ExtendedMaterial<
            StandardMaterial,
            crate::common::game::bricks::studs::ShadowOpacityExtension,
        >,
    >,
    studs_materials: &mut Assets<
        ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>,
    >,
    material_cache: &mut crate::common::game::bricks::BrickMaterialCache,
    studs_assets: &crate::common::game::bricks::studs::StudsAssets,
    explorer_query: &Query<
        (
            Entity,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&Brick>,
            Option<&crate::scripting::ecs::ServerScript>,
            Option<&crate::scripting::ecs::LocalScript>,
            Option<&crate::scripting::ecs::ModuleScript>,
            Option<&crate::common::game::assets::components::Image>,
            Option<&crate::common::game::assets::components::Texture>,
            Option<&crate::common::game::assets::components::Mesh>,
            Option<&crate::common::game::assets::components::Sound>,
        ),
        Without<Camera3d>,
    >,
    texture_assets: &Query<&crate::common::game::assets::components::Texture>,
    active_editor: &mut ResMut<crate::studio::ui::resources::ActiveScriptEditor>,
    studs_query: &Query<&crate::common::game::bricks::components::BrickStuds>,
    workspace_studs: &crate::common::game::bricks::WorkspaceShowStuds,
    asset_status: &crate::common::game::assets::status::AssetStatusCache,
    current_uid: Option<i32>,
) {
    if selected_entities.is_empty() {
        return;
    }

    let is_logged_in = current_uid.is_some();

    let mut script_entity = None;
    let ent = selected_entities[0];
    if let Ok((_, _, _, _, _, s, l, m, _, _, _, _)) = explorer_query.get(ent) {
        if s.is_some() || l.is_some() || m.is_some() {
            script_entity = Some(ent);
        }
    }

    if let Some(entity) = script_entity {
        let name_str = explorer_query
            .get(entity)
            .map(|(_, n, _, _, _, _, _, _, _, _, _, _)| n.as_str().to_string())
            .unwrap_or_else(|_| "Script".to_string());
        let (code, script_type, mut enabled) =
            if let Ok((_, _, _, _, _, s, l, m, _, _, _, _)) = explorer_query.get(entity) {
                if let Some(ref script) = s {
                    (script.code.clone(), "Script", script.enabled)
                } else if let Some(ref script) = l {
                    (script.code.clone(), "LocalScript", script.enabled)
                } else if let Some(ref script) = m {
                    (script.code.clone(), "ModuleScript", true)
                } else {
                    ("".to_string(), "Script", true)
                }
            } else {
                ("".to_string(), "Script", true)
            };

        let parent_name_str = if let Ok((_, _, Some(child_of), _, _, _, _, _, _, _, _, _)) =
            explorer_query.get(entity)
        {
            explorer_query
                .get(child_of.parent())
                .map(|(_, name, _, _, _, _, _, _, _, _, _, _)| name.as_str().to_string())
                .unwrap_or_else(|_| "None".to_string())
        } else {
            "Workspace".to_string()
        };

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Properties")
                        .color(egui::Color32::from_rgb(0, 0, 0))
                        .strong()
                        .size(16.0),
                );
            });

            ui.add_space(8.0);
            let (sep_rect, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
            ui.add_space(8.0);

            egui::CollapsingHeader::new(
                egui::RichText::new("Information")
                    .color(egui::Color32::from_rgb(0, 0, 0))
                    .strong()
                    .size(14.0),
            )
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("properties_script_info_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Name")
                                .color(egui::Color32::from_rgb(60, 60, 60))
                                .size(13.0),
                        );
                        let name_id = ui.make_persistent_id("properties_script_name_input");
                        let mut name_edit = ui.data_mut(|d| {
                            d.get_temp::<String>(name_id)
                                .unwrap_or_else(|| name_str.clone())
                        });
                        let res = ui.add(egui::TextEdit::singleline(&mut name_edit));
                        if res.changed() {
                            ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                            commands.entity(entity).insert(Name::new(name_edit.clone()));
                        } else if !res.has_focus() {
                            if name_edit != name_str {
                                name_edit = name_str.clone();
                                ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                            }
                        }
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Class Name")
                                .color(egui::Color32::from_rgb(60, 60, 60))
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(script_type)
                                .color(egui::Color32::BLACK)
                                .size(13.0),
                        );
                        ui.end_row();

                        if script_type != "ModuleScript" {
                            ui.label(
                                egui::RichText::new("Enabled")
                                    .color(egui::Color32::from_rgb(60, 60, 60))
                                    .size(13.0),
                            );
                            if ui.checkbox(&mut enabled, "").changed() {
                                if let Ok((_, _, _, _, _, s, l, _, _, _, _, _)) =
                                    explorer_query.get(entity)
                                {
                                    if s.is_some() {
                                        commands.entity(entity).insert(
                                            crate::scripting::ecs::ServerScript {
                                                code: code.clone(),
                                                enabled,
                                                started: false,
                                                running_code: String::new(),
                                            },
                                        );
                                    } else if l.is_some() {
                                        commands.entity(entity).insert(
                                            crate::scripting::ecs::LocalScript {
                                                code: code.clone(),
                                                enabled,
                                                started: false,
                                                running_code: String::new(),
                                            },
                                        );
                                    }
                                }
                            }
                            ui.end_row();
                        }

                        ui.label(
                            egui::RichText::new("Parent")
                                .color(egui::Color32::from_rgb(60, 60, 60))
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(&parent_name_str)
                                .color(egui::Color32::BLACK)
                                .size(13.0),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Lines")
                                .color(egui::Color32::from_rgb(60, 60, 60))
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", code.lines().count()))
                                .color(egui::Color32::BLACK)
                                .size(13.0),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Characters")
                                .color(egui::Color32::from_rgb(60, 60, 60))
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", code.len()))
                                .color(egui::Color32::BLACK)
                                .size(13.0),
                        );
                        ui.end_row();
                    });
            });

            ui.add_space(12.0);

            ui.vertical_centered_justified(|ui| {
                if ui
                    .button(egui::RichText::new("Open in Editor").strong())
                    .clicked()
                {
                    if !active_editor.open_entities.contains(&entity) {
                        active_editor.open_entities.push(entity);
                    }
                    active_editor.entity = Some(entity);
                }
            });
        });
        return;
    }

    let mut image_entity = None;
    let ent = selected_entities[0];
    if let Ok((_, _, _, _, _, _, _, _, image_opt, _, _, _)) = explorer_query.get(ent) {
        if image_opt.is_some() {
            image_entity = Some(ent);
        }
    }

    if let Some(entity) = image_entity {
        let name_str = explorer_query
            .get(entity)
            .map(|(_, n, _, _, _, _, _, _, _, _, _, _)| n.as_str().to_string())
            .unwrap_or_else(|_| "Image".to_string());
        let parent_name_str = if let Ok((_, _, Some(child_of), _, _, _, _, _, _, _, _, _)) =
            explorer_query.get(entity)
        {
            explorer_query
                .get(child_of.parent())
                .map(|(_, name, _, _, _, _, _, _, _, _, _, _)| name.as_str().to_string())
                .unwrap_or_else(|_| "None".to_string())
        } else {
            "Workspace".to_string()
        };
        let mut current_id = 0u32;
        let mut current_face = "front".to_string();
        let mut is_parented = false;
        if let Ok((_, _, _, _, _, _, _, _, image_opt, _, _, _)) = explorer_query.get(entity) {
            if let Some(image) = image_opt {
                current_id = image.asset_id;
                current_face = image
                    .face
                    .map(|f| f.as_str().to_string())
                    .unwrap_or_else(|| "front".to_string());
            }
        }
        if let Ok((_, _, child_of_opt, _, _, _, _, _, _, _, _, _)) = explorer_query.get(entity) {
            is_parented = child_of_opt.is_some();
        }

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
                    egui::Grid::new("properties_image_info_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Name").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let name_id = ui.make_persistent_id("properties_image_name_input");
                            let mut name_edit = ui.data_mut(|d| d.get_temp::<String>(name_id).unwrap_or_else(|| name_str.clone()));
                            let res = ui.add(egui::TextEdit::singleline(&mut name_edit));
                            if res.changed() {
                                ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                                commands.entity(entity).insert(Name::new(name_edit.clone()));
                            } else if !res.has_focus() {
                                if name_edit != name_str {
                                    name_edit = name_str.clone();
                                    ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Class Name").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            ui.label(egui::RichText::new("Image").color(egui::Color32::BLACK).size(13.0));
                            ui.end_row();

                            ui.label(egui::RichText::new("ID").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let id_id = ui.make_persistent_id("properties_image_id_input");
                            let mut id_text = ui.data_mut(|d| d.get_temp::<String>(id_id).unwrap_or_else(|| current_id.to_string()));
                            let id_res = ui.add_enabled(is_logged_in, egui::TextEdit::singleline(&mut id_text).desired_width(60.0)).on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                            if id_res.changed() {
                                ui.data_mut(|d| d.insert_temp(id_id, id_text.clone()));
                                if let Ok(parsed) = id_text.parse::<u32>() {
                                    if let Ok((_, _, _, _, _, _, _, _, image_opt, _, _, _)) = explorer_query.get(entity) {
                                        if let Some(image) = image_opt {
                                            commands.entity(entity).insert(crate::common::game::assets::components::Image {
                                                asset_id: parsed,
                                                face: image.face,
                                            });
                                        }
                                    }
                                }
                            } else if !id_res.has_focus() {
                                let expected = current_id.to_string();
                                if id_text != expected {
                                    id_text = expected;
                                    ui.data_mut(|d| d.insert_temp(id_id, id_text.clone()));
                                }
                            }
                            ui.end_row();

                            draw_asset_status_row(ui, current_id, asset_status, current_uid, "Image");

                            ui.label(egui::RichText::new("Face").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            if is_parented {
                                let face_res = ui.add_enabled_ui(is_logged_in, |ui| {
                                    egui::ComboBox::from_id_salt("properties_image_face_combo")
                                        .selected_text(&current_face)
                                        .show_ui(ui, |ui| {
                                            for face in ["top", "bottom", "left", "right", "front", "back"] {
                                                if ui.selectable_label(current_face == face, face).clicked() {
                                                    current_face = face.to_string();
                                                    if let Ok((_, _, _, _, _, _, _, _, image_opt, _, _, _)) = explorer_query.get(entity) {
                                                        if let Some(image) = image_opt {
                                                            commands.entity(entity).insert(crate::common::game::assets::components::Image {
                                                                asset_id: image.asset_id,
                                                                face: Some(crate::common::game::assets::components::ImageFace::from_str(face).unwrap_or(crate::common::game::assets::components::ImageFace::Front)),
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                });
                                if !is_logged_in {
                                    let _ = face_res.response.on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                                }
                            } else {
                                ui.label(egui::RichText::new("None").color(egui::Color32::BLACK).size(13.0));
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Parent").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            ui.label(egui::RichText::new(&parent_name_str).color(egui::Color32::BLACK).size(13.0));
                            ui.end_row();
                        });
                });

            ui.add_space(8.0);

            let (image_pos_studs, image_scale, image_rot_deg) = match properties_query.get(entity) {
                Ok((_, transform, _, _, _, _, _, _, _, _, _, _)) => {
                    let (rx, ry, rz) = transform.rotation.to_euler(EulerRot::XYZ);
                    (
                        transform.translation / 0.28,
                        transform.scale,
                        Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()),
                    )
                }
                Err(_) => (Vec3::ZERO, Vec3::ONE, Vec3::ZERO),
            };

            egui::CollapsingHeader::new(egui::RichText::new("Transform").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("properties_image_transform_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Position").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut pos_studs = image_pos_studs;
                            let mut new_px = None;
                            let mut new_py = None;
                            let mut new_pz = None;
                            ui.horizontal(|ui| {
                                new_px = draw_coord_edit(ui, "X", &mut pos_studs.x, true, "image_pos_x");
                                new_py = draw_coord_edit(ui, "Y", &mut pos_studs.y, true, "image_pos_y");
                                new_pz = draw_coord_edit(ui, "Z", &mut pos_studs.z, true, "image_pos_z");
                            });
                            if new_px.is_some() || new_py.is_some() || new_pz.is_some() {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    if let Some(x) = new_px { transform.translation.x = x * 0.28; }
                                    if let Some(y) = new_py { transform.translation.y = y * 0.28; }
                                    if let Some(z) = new_pz { transform.translation.z = z * 0.28; }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Size").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut scale_val = image_scale;
                            let mut new_sx = None;
                            let mut new_sy = None;
                            let mut new_sz = None;
                            ui.horizontal(|ui| {
                                new_sx = draw_coord_edit(ui, "X", &mut scale_val.x, true, "image_size_x");
                                new_sy = draw_coord_edit(ui, "Y", &mut scale_val.y, true, "image_size_y");
                                new_sz = draw_coord_edit(ui, "Z", &mut scale_val.z, true, "image_size_z");
                            });
                            if new_sx.is_some() || new_sy.is_some() || new_sz.is_some() {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    if let Some(x) = new_sx { transform.scale.x = x.max(0.01); }
                                    if let Some(y) = new_sy { transform.scale.y = y.max(0.01); }
                                    if let Some(z) = new_sz { transform.scale.z = z.max(0.01); }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Rotation").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut rot_deg = image_rot_deg;
                            let mut new_rx = None;
                            let mut new_ry = None;
                            let mut new_rz = None;
                            ui.horizontal(|ui| {
                                new_rx = draw_coord_edit(ui, "X", &mut rot_deg.x, true, "image_rot_x");
                                new_ry = draw_coord_edit(ui, "Y", &mut rot_deg.y, true, "image_rot_y");
                                new_rz = draw_coord_edit(ui, "Z", &mut rot_deg.z, true, "image_rot_z");
                            });
                            if new_rx.is_some() || new_ry.is_some() || new_rz.is_some() {
                                let rx_val = new_rx.unwrap_or(rot_deg.x).to_radians();
                                let ry_val = new_ry.unwrap_or(rot_deg.y).to_radians();
                                let rz_val = new_rz.unwrap_or(rot_deg.z).to_radians();
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    transform.rotation = Quat::from_euler(EulerRot::XYZ, rx_val, ry_val, rz_val);
                                }
                            }
                            ui.end_row();
                        });
                });
        });
        return;
    }

    let mut texture_entity = None;
    let ent = selected_entities[0];
    if let Ok((_, _, _, _, _, _, _, _, _, texture_opt, _, _)) = explorer_query.get(ent) {
        if texture_opt.is_some() {
            texture_entity = Some(ent);
        }
    }

    if let Some(entity) = texture_entity {
        let name_str = explorer_query
            .get(entity)
            .map(|(_, n, _, _, _, _, _, _, _, _, _, _)| n.as_str().to_string())
            .unwrap_or_else(|_| "Texture".to_string());
        let parent_name_str = if let Ok((_, _, Some(child_of), _, _, _, _, _, _, _, _, _)) =
            explorer_query.get(entity)
        {
            explorer_query
                .get(child_of.parent())
                .map(|(_, name, _, _, _, _, _, _, _, _, _, _)| name.as_str().to_string())
                .unwrap_or_else(|_| "None".to_string())
        } else {
            "Workspace".to_string()
        };
        let mut current_id_text = "mesh/0".to_string();
        if let Ok(texture) = texture_assets.get(entity) {
            current_id_text = texture.as_content_id();
        }

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Properties")
                        .color(egui::Color32::from_rgb(0, 0, 0))
                        .strong()
                        .size(16.0),
                );
            });

            ui.add_space(8.0);
            let (sep_rect, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
            ui.add_space(8.0);

            egui::CollapsingHeader::new(
                egui::RichText::new("Information")
                    .color(egui::Color32::from_rgb(0, 0, 0))
                    .strong()
                    .size(14.0),
            )
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("properties_texture_info_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Name")
                                .color(egui::Color32::from_rgb(60, 60, 60))
                                .size(13.0),
                        );
                        let name_id = ui.make_persistent_id("properties_texture_name_input");
                        let mut name_edit = ui.data_mut(|d| {
                            d.get_temp::<String>(name_id)
                                .unwrap_or_else(|| name_str.clone())
                        });
                        let res = ui.add(egui::TextEdit::singleline(&mut name_edit));
                        if res.changed() {
                            ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                            commands.entity(entity).insert(Name::new(name_edit.clone()));
                        } else if !res.has_focus() {
                            if name_edit != name_str {
                                name_edit = name_str.clone();
                                ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                            }
                        }
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Class Name")
                                .color(egui::Color32::from_rgb(60, 60, 60))
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new("Texture")
                                .color(egui::Color32::BLACK)
                                .size(13.0),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("ID")
                                .color(egui::Color32::from_rgb(60, 60, 60))
                                .size(13.0),
                        );
                        let id_id = ui.make_persistent_id("properties_texture_id_input");
                        let mut id_text = ui.data_mut(|d| {
                            d.get_temp::<String>(id_id)
                                .unwrap_or_else(|| current_id_text.clone())
                        });
                        let id_res =
                            ui.add(egui::TextEdit::singleline(&mut id_text).desired_width(90.0));
                        if id_res.changed() {
                            ui.data_mut(|d| d.insert_temp(id_id, id_text.clone()));
                            if let Some((asset_id, is_decal)) =
                                crate::common::game::assets::components::Texture::parse_content_id(
                                    &id_text,
                                )
                            {
                                commands.entity(entity).insert(
                                    crate::common::game::assets::components::Texture {
                                        asset_id,
                                        is_decal,
                                    },
                                );
                            }
                        } else if !id_res.has_focus() {
                            if id_text != current_id_text {
                                id_text = current_id_text.clone();
                                ui.data_mut(|d| d.insert_temp(id_id, id_text.clone()));
                            }
                        }
                        ui.end_row();

                        if let Ok(texture) = texture_assets.get(entity) {
                            let kind = if texture.is_decal { "Image" } else { "Mesh" };
                            draw_asset_status_row(
                                ui,
                                texture.asset_id,
                                asset_status,
                                current_uid,
                                kind,
                            );
                        }

                        ui.label(
                            egui::RichText::new("Parent")
                                .color(egui::Color32::from_rgb(60, 60, 60))
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(&parent_name_str)
                                .color(egui::Color32::BLACK)
                                .size(13.0),
                        );
                        ui.end_row();
                    });
            });
        });
        return;
    }

    let mut mesh_entity = None;
    let ent = selected_entities[0];
    if let Ok((_, _, _, _, _, _, _, _, _, _, mesh_opt, _)) = explorer_query.get(ent) {
        if mesh_opt.is_some() {
            mesh_entity = Some(ent);
        }
    }

    if let Some(entity) = mesh_entity {
        let name_str = explorer_query
            .get(entity)
            .map(|(_, n, _, _, _, _, _, _, _, _, _, _)| n.as_str().to_string())
            .unwrap_or_else(|_| "Mesh".to_string());
        let parent_name_str = if let Ok((_, _, Some(child_of), _, _, _, _, _, _, _, _, _)) =
            explorer_query.get(entity)
        {
            explorer_query
                .get(child_of.parent())
                .map(|(_, name, _, _, _, _, _, _, _, _, _, _)| name.as_str().to_string())
                .unwrap_or_else(|_| "None".to_string())
        } else {
            "Workspace".to_string()
        };
        let mut current_id = 0u32;
        let mut current_normalize = false;
        if let Ok((_, _, _, _, _, _, _, _, _, _, mesh_opt, _)) = explorer_query.get(entity) {
            if let Some(mesh) = mesh_opt {
                current_id = mesh.asset_id;
                current_normalize = mesh.normalize;
            }
        }
        let mut mesh_phys = properties_query
            .get(entity)
            .ok()
            .and_then(|(_, _, _, _, _, _, _, _, _, _, _, p)| p.map(|p| *p))
            .unwrap_or_default();

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
                    egui::Grid::new("properties_mesh_info_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Name").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let name_id = ui.make_persistent_id("properties_mesh_name_input");
                            let mut name_edit = ui.data_mut(|d| d.get_temp::<String>(name_id).unwrap_or_else(|| name_str.clone()));
                            let res = ui.add(egui::TextEdit::singleline(&mut name_edit));
                            if res.changed() {
                                ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                                commands.entity(entity).insert(Name::new(name_edit.clone()));
                            } else if !res.has_focus() {
                                if name_edit != name_str {
                                    name_edit = name_str.clone();
                                    ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Class Name").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            ui.label(egui::RichText::new("Mesh").color(egui::Color32::BLACK).size(13.0));
                            ui.end_row();

                            ui.label(egui::RichText::new("ID").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let id_id = ui.make_persistent_id("properties_mesh_id_input");
                            let mut id_text = ui.data_mut(|d| d.get_temp::<String>(id_id).unwrap_or_else(|| current_id.to_string()));
                            let id_res = ui.add_enabled(is_logged_in, egui::TextEdit::singleline(&mut id_text).desired_width(60.0)).on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                            if id_res.changed() {
                                ui.data_mut(|d| d.insert_temp(id_id, id_text.clone()));
                                if let Ok(parsed) = id_text.parse::<u32>() {
                                    if let Ok((_, _, _, children_opt, _, _, _, _, _, _, mesh_opt, _)) = explorer_query.get(entity) {
                                        if let Some(mesh) = mesh_opt {
                                            let old_id = mesh.asset_id;
                                            if parsed != old_id {
                                                if let Some(children) = children_opt {
                                                    for child in children.iter() {
                                                        if let Ok(texture) = texture_assets.get(child) {
                                                            if !texture.is_decal && texture.asset_id == old_id {
                                                                commands.entity(child).insert(crate::common::game::assets::components::Texture {
                                                                    asset_id: parsed,
                                                                    is_decal: false,
                                                                });
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            commands.entity(entity).insert(crate::common::game::assets::components::Mesh {
                                                asset_id: parsed,
                                                normalize: mesh.normalize,
                                            });
                                            let has_texture_child = children_opt.is_some_and(|children| {
                                                children.iter().any(|c| texture_assets.get(c).is_ok())
                                            });
                                            if !has_texture_child {
                                                let texture_entity = commands
                                                    .spawn((
                                                        Name::new("Texture"),
                                                        Transform::default(),
                                                        crate::common::game::assets::components::Texture {
                                                            asset_id: parsed,
                                                            is_decal: false,
                                                        },
                                                        lightyear::prelude::Replicate::default(),
                                                    ))
                                                    .id();
                                                commands.entity(entity).add_child(texture_entity);
                                            }
                                        }
                                    }
                                }
                            } else if !id_res.has_focus() {
                                let expected = current_id.to_string();
                                if id_text != expected {
                                    id_text = expected;
                                    ui.data_mut(|d| d.insert_temp(id_id, id_text.clone()));
                                }
                            }
                            ui.end_row();

                            draw_asset_status_row(ui, current_id, asset_status, current_uid, "Mesh");

                            ui.label(egui::RichText::new("Normalize").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let normalize_response = ui.checkbox(&mut current_normalize, "").on_hover_text(
                                "Scales the mesh so its largest dimension is exactly 1 stud, regardless of the model's original size.",
                            );
                            if normalize_response.changed() {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    transform.scale = Vec3::ONE;
                                }
                                if let Ok((_, _, _, _, _, _, _, _, _, _, mesh_opt, _)) = explorer_query.get(entity) {
                                    if let Some(mesh) = mesh_opt {
                                        commands.entity(entity).insert(crate::common::game::assets::components::Mesh {
                                            asset_id: mesh.asset_id,
                                            normalize: current_normalize,
                                        });
                                    }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Parent").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            ui.label(egui::RichText::new(&parent_name_str).color(egui::Color32::BLACK).size(13.0));
                            ui.end_row();
                        });
                });

            ui.add_space(8.0);

            let (mesh_pos_studs, mesh_scale, mesh_rot_deg) = match properties_query.get(entity) {
                Ok((_, transform, _, _, _, _, _, _, _, _, _, _)) => {
                    let (rx, ry, rz) = transform.rotation.to_euler(EulerRot::XYZ);
                    (
                        transform.translation / 0.28,
                        transform.scale,
                        Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()),
                    )
                }
                Err(_) => (Vec3::ZERO, Vec3::ONE, Vec3::ZERO),
            };

            egui::CollapsingHeader::new(egui::RichText::new("Transform").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("properties_mesh_transform_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Position").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut pos_studs = mesh_pos_studs;
                            let mut new_px = None;
                            let mut new_py = None;
                            let mut new_pz = None;
                            ui.horizontal(|ui| {
                                new_px = draw_coord_edit(ui, "X", &mut pos_studs.x, true, "mesh_pos_x");
                                new_py = draw_coord_edit(ui, "Y", &mut pos_studs.y, true, "mesh_pos_y");
                                new_pz = draw_coord_edit(ui, "Z", &mut pos_studs.z, true, "mesh_pos_z");
                            });
                            if new_px.is_some() || new_py.is_some() || new_pz.is_some() {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    if let Some(x) = new_px { transform.translation.x = x * 0.28; }
                                    if let Some(y) = new_py { transform.translation.y = y * 0.28; }
                                    if let Some(z) = new_pz { transform.translation.z = z * 0.28; }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Size").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut scale_val = mesh_scale;
                            let mut new_sx = None;
                            let mut new_sy = None;
                            let mut new_sz = None;
                            ui.horizontal(|ui| {
                                new_sx = draw_coord_edit(ui, "X", &mut scale_val.x, true, "mesh_size_x");
                                new_sy = draw_coord_edit(ui, "Y", &mut scale_val.y, true, "mesh_size_y");
                                new_sz = draw_coord_edit(ui, "Z", &mut scale_val.z, true, "mesh_size_z");
                            });
                            if new_sx.is_some() || new_sy.is_some() || new_sz.is_some() {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    if let Some(x) = new_sx { transform.scale.x = x.max(0.01); }
                                    if let Some(y) = new_sy { transform.scale.y = y.max(0.01); }
                                    if let Some(z) = new_sz { transform.scale.z = z.max(0.01); }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Rotation").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut rot_deg = mesh_rot_deg;
                            let mut new_rx = None;
                            let mut new_ry = None;
                            let mut new_rz = None;
                            ui.horizontal(|ui| {
                                new_rx = draw_coord_edit(ui, "X", &mut rot_deg.x, true, "mesh_rot_x");
                                new_ry = draw_coord_edit(ui, "Y", &mut rot_deg.y, true, "mesh_rot_y");
                                new_rz = draw_coord_edit(ui, "Z", &mut rot_deg.z, true, "mesh_rot_z");
                            });
                            if new_rx.is_some() || new_ry.is_some() || new_rz.is_some() {
                                let rx_val = new_rx.unwrap_or(rot_deg.x).to_radians();
                                let ry_val = new_ry.unwrap_or(rot_deg.y).to_radians();
                                let rz_val = new_rz.unwrap_or(rot_deg.z).to_radians();
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    transform.rotation = Quat::from_euler(EulerRot::XYZ, rx_val, ry_val, rz_val);
                                }
                            }
                            ui.end_row();
                        });
                });

            ui.add_space(8.0);

            egui::CollapsingHeader::new(egui::RichText::new("Physics").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("properties_mesh_physics_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            let mut phys_changed = false;

                            ui.label(egui::RichText::new("Physics Enabled").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            if ui.checkbox(&mut mesh_phys.enabled, "").changed() {
                                phys_changed = true;
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Player can collide").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            if ui.checkbox(&mut mesh_phys.player_can_collide, "").changed() {
                                phys_changed = true;
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Friction").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut friction = mesh_phys.friction;
                            if let Some(new_val) = draw_coord_edit(ui, "", &mut friction, true, "mesh_friction") {
                                mesh_phys.friction = new_val.clamp(0.0, 1.0);
                                phys_changed = true;
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Gravity Scale").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut gravity_scale = mesh_phys.gravity_scale;
                            if let Some(new_val) = draw_coord_edit(ui, "", &mut gravity_scale, true, "mesh_gravity_scale") {
                                mesh_phys.gravity_scale = new_val;
                                phys_changed = true;
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Mass").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut mass = mesh_phys.mass;
                            if let Some(new_val) = draw_coord_edit(ui, "", &mut mass, true, "mesh_mass") {
                                mesh_phys.mass = new_val.max(0.001);
                                phys_changed = true;
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Bounciness").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut bounciness = mesh_phys.bounciness;
                            if let Some(new_val) = draw_coord_edit(ui, "", &mut bounciness, true, "mesh_bounciness") {
                                mesh_phys.bounciness = new_val.clamp(0.0, 1.0);
                                phys_changed = true;
                            }
                            ui.end_row();

                            if phys_changed {
                                let layers = if mesh_phys.player_can_collide {
                                    CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                                } else {
                                    CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                                };
                                commands.entity(entity).insert((
                                    mesh_phys.clone(),
                                    Friction::new(mesh_phys.friction),
                                    Restitution::new(mesh_phys.bounciness),
                                    GravityScale(mesh_phys.gravity_scale),
                                    Mass(mesh_phys.mass),
                                    if mesh_phys.enabled { RigidBody::Dynamic } else { RigidBody::Static },
                                    layers,
                                ));
                            }
                        });
                });
        });
        return;
    }

    let mut sound_entity = None;
    let ent = selected_entities[0];
    if let Ok((_, _, _, _, _, _, _, _, _, _, _, sound_opt)) = explorer_query.get(ent) {
        if sound_opt.is_some() {
            sound_entity = Some(ent);
        }
    }

    if let Some(entity) = sound_entity {
        let name_str = explorer_query
            .get(entity)
            .map(|(_, n, _, _, _, _, _, _, _, _, _, _)| n.as_str().to_string())
            .unwrap_or_else(|_| "Sound".to_string());
        let parent_name_str = if let Ok((_, _, Some(child_of), _, _, _, _, _, _, _, _, _)) =
            explorer_query.get(entity)
        {
            explorer_query
                .get(child_of.parent())
                .map(|(_, name, _, _, _, _, _, _, _, _, _, _)| name.as_str().to_string())
                .unwrap_or_else(|_| "None".to_string())
        } else {
            "Workspace".to_string()
        };
        let mut current_id = 0u32;
        let mut current_volume = crate::common::game::assets::components::Sound::default().volume;
        let mut current_speed = 1.0f32;
        let mut current_looped = false;
        let mut current_replicate_time = false;
        let mut current_playing = false;
        let mut current_spatial = false;
        if let Ok((_, _, _, _, _, _, _, _, _, _, _, sound_opt)) = explorer_query.get(entity) {
            if let Some(sound) = sound_opt {
                current_id = sound.asset_id;
                current_volume = sound.volume;
                current_speed = sound.speed;
                current_looped = sound.looped;
                current_replicate_time = sound.replicate_time;
                current_playing = sound.playing;
                current_spatial = sound.spatial;
            }
        }

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
                    egui::Grid::new("properties_sound_info_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Name").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let name_id = ui.make_persistent_id("properties_sound_name_input");
                            let mut name_edit = ui.data_mut(|d| d.get_temp::<String>(name_id).unwrap_or_else(|| name_str.clone()));
                            let res = ui.add(egui::TextEdit::singleline(&mut name_edit));
                            if res.changed() {
                                ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                                commands.entity(entity).insert(Name::new(name_edit.clone()));
                            } else if !res.has_focus() {
                                if name_edit != name_str {
                                    name_edit = name_str.clone();
                                    ui.data_mut(|d| d.insert_temp(name_id, name_edit.clone()));
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Class Name").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            ui.label(egui::RichText::new("Sound").color(egui::Color32::BLACK).size(13.0));
                            ui.end_row();

                            ui.label(egui::RichText::new("ID").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let id_id = ui.make_persistent_id("properties_sound_id_input");
                            let mut id_text = ui.data_mut(|d| d.get_temp::<String>(id_id).unwrap_or_else(|| current_id.to_string()));
                            let id_res = ui.add_enabled(is_logged_in, egui::TextEdit::singleline(&mut id_text).desired_width(60.0)).on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                            if id_res.changed() {
                                ui.data_mut(|d| d.insert_temp(id_id, id_text.clone()));
                                if let Ok(parsed) = id_text.parse::<u32>() {
                                    if let Ok((_, _, _, _, _, _, _, _, _, _, _, sound_opt)) = explorer_query.get(entity) {
                                        if let Some(sound) = sound_opt {
                                            let mut updated = *sound;
                                            updated.asset_id = parsed;
                                            commands.entity(entity).insert(updated);
                                        }
                                    }
                                }
                            } else if !id_res.has_focus() {
                                let expected = current_id.to_string();
                                if id_text != expected {
                                    id_text = expected;
                                    ui.data_mut(|d| d.insert_temp(id_id, id_text.clone()));
                                }
                            }
                            ui.end_row();

                            draw_asset_status_row(ui, current_id, asset_status, current_uid, "Audio");

                            ui.label(egui::RichText::new("Volume").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut volume_edit = current_volume;
                            let volume_res = ui.add_enabled(is_logged_in, egui::Slider::new(&mut volume_edit, 0.0..=crate::common::game::assets::components::Sound::MAX_VOLUME).show_value(true)).on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                            if volume_res.changed() {
                                if let Ok((_, _, _, _, _, _, _, _, _, _, _, sound_opt)) = explorer_query.get(entity) {
                                    if let Some(sound) = sound_opt {
                                        let mut updated = *sound;
                                        updated.volume = crate::common::game::assets::components::Sound::clamp_volume(volume_edit);
                                        commands.entity(entity).insert(updated);
                                    }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Speed").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let speed_id = ui.make_persistent_id("properties_sound_speed_input");
                            let mut speed_text = ui.data_mut(|d| d.get_temp::<String>(speed_id).unwrap_or_else(|| format!("{:.2}", current_speed)));
                            let speed_res = ui.add_enabled(is_logged_in, egui::TextEdit::singleline(&mut speed_text).desired_width(60.0)).on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                            if speed_res.changed() {
                                ui.data_mut(|d| d.insert_temp(speed_id, speed_text.clone()));
                                if let Ok(parsed) = speed_text.parse::<f32>() {
                                    if let Ok((_, _, _, _, _, _, _, _, _, _, _, sound_opt)) = explorer_query.get(entity) {
                                        if let Some(sound) = sound_opt {
                                            let mut updated = *sound;
                                            updated.speed = crate::common::game::assets::components::Sound::clamp_speed(parsed);
                                            commands.entity(entity).insert(updated);
                                        }
                                    }
                                }
                            } else if !speed_res.has_focus() {
                                let expected = format!("{:.2}", current_speed);
                                if speed_text != expected {
                                    speed_text = expected;
                                    ui.data_mut(|d| d.insert_temp(speed_id, speed_text.clone()));
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Looped").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut looped_edit = current_looped;
                            let looped_res = ui.add_enabled(is_logged_in, egui::Checkbox::new(&mut looped_edit, "")).on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                            if looped_res.changed() {
                                if let Ok((_, _, _, _, _, _, _, _, _, _, _, sound_opt)) = explorer_query.get(entity) {
                                    if let Some(sound) = sound_opt {
                                        let mut updated = *sound;
                                        updated.looped = looped_edit;
                                        commands.entity(entity).insert(updated);
                                    }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Replicate Time To Clients").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut replicate_edit = current_replicate_time;
                            let replicate_res = ui.add_enabled(is_logged_in, egui::Checkbox::new(&mut replicate_edit, "")).on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                            if replicate_res.changed() {
                                if let Ok((_, _, _, _, _, _, _, _, _, _, _, sound_opt)) = explorer_query.get(entity) {
                                    if let Some(sound) = sound_opt {
                                        let mut updated = *sound;
                                        updated.replicate_time = replicate_edit;
                                        commands.entity(entity).insert(updated);
                                    }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Playing").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut playing_edit = current_playing;
                            let playing_res = ui.add_enabled(is_logged_in, egui::Checkbox::new(&mut playing_edit, "")).on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                            if playing_res.changed() {
                                if let Ok((_, _, _, _, _, _, _, _, _, _, _, sound_opt)) = explorer_query.get(entity) {
                                    if let Some(sound) = sound_opt {
                                        let mut updated = *sound;
                                        updated.playing = playing_edit;
                                        commands.entity(entity).insert(updated);
                                    }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Spatial").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut spatial_edit = current_spatial;
                            let spatial_res = ui.add_enabled(is_logged_in, egui::Checkbox::new(&mut spatial_edit, "")).on_disabled_hover_text(crate::studio::auth::LOGIN_REQUIRED_TOOLTIP);
                            if spatial_res.changed() {
                                if let Ok((_, _, _, _, _, _, _, _, _, _, _, sound_opt)) = explorer_query.get(entity) {
                                    if let Some(sound) = sound_opt {
                                        let mut updated = *sound;
                                        updated.spatial = spatial_edit;
                                        commands.entity(entity).insert(updated);
                                    }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Parent").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            ui.label(egui::RichText::new(&parent_name_str).color(egui::Color32::BLACK).size(13.0));
                            ui.end_row();
                        });
                });

            ui.add_space(8.0);

            let (sound_pos_studs, sound_scale, sound_rot_deg) = match properties_query.get(entity) {
                Ok((_, transform, _, _, _, _, _, _, _, _, _, _)) => {
                    let (rx, ry, rz) = transform.rotation.to_euler(EulerRot::XYZ);
                    (
                        transform.translation / 0.28,
                        transform.scale,
                        Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()),
                    )
                }
                Err(_) => (Vec3::ZERO, Vec3::ONE, Vec3::ZERO),
            };

            egui::CollapsingHeader::new(egui::RichText::new("Transform").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("properties_sound_transform_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Position").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut pos_studs = sound_pos_studs;
                            let mut new_px = None;
                            let mut new_py = None;
                            let mut new_pz = None;
                            ui.horizontal(|ui| {
                                new_px = draw_coord_edit(ui, "X", &mut pos_studs.x, true, "sound_pos_x");
                                new_py = draw_coord_edit(ui, "Y", &mut pos_studs.y, true, "sound_pos_y");
                                new_pz = draw_coord_edit(ui, "Z", &mut pos_studs.z, true, "sound_pos_z");
                            });
                            if new_px.is_some() || new_py.is_some() || new_pz.is_some() {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    if let Some(x) = new_px { transform.translation.x = x * 0.28; }
                                    if let Some(y) = new_py { transform.translation.y = y * 0.28; }
                                    if let Some(z) = new_pz { transform.translation.z = z * 0.28; }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Size").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut scale_val = sound_scale;
                            let mut new_sx = None;
                            let mut new_sy = None;
                            let mut new_sz = None;
                            ui.horizontal(|ui| {
                                new_sx = draw_coord_edit(ui, "X", &mut scale_val.x, true, "sound_size_x");
                                new_sy = draw_coord_edit(ui, "Y", &mut scale_val.y, true, "sound_size_y");
                                new_sz = draw_coord_edit(ui, "Z", &mut scale_val.z, true, "sound_size_z");
                            });
                            if new_sx.is_some() || new_sy.is_some() || new_sz.is_some() {
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    if let Some(x) = new_sx { transform.scale.x = x.max(0.01); }
                                    if let Some(y) = new_sy { transform.scale.y = y.max(0.01); }
                                    if let Some(z) = new_sz { transform.scale.z = z.max(0.01); }
                                }
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("Rotation").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            let mut rot_deg = sound_rot_deg;
                            let mut new_rx = None;
                            let mut new_ry = None;
                            let mut new_rz = None;
                            ui.horizontal(|ui| {
                                new_rx = draw_coord_edit(ui, "X", &mut rot_deg.x, true, "sound_rot_x");
                                new_ry = draw_coord_edit(ui, "Y", &mut rot_deg.y, true, "sound_rot_y");
                                new_rz = draw_coord_edit(ui, "Z", &mut rot_deg.z, true, "sound_rot_z");
                            });
                            if new_rx.is_some() || new_ry.is_some() || new_rz.is_some() {
                                let rx_val = new_rx.unwrap_or(rot_deg.x).to_radians();
                                let ry_val = new_ry.unwrap_or(rot_deg.y).to_radians();
                                let rz_val = new_rz.unwrap_or(rot_deg.z).to_radians();
                                if let Ok((_, mut transform, _, _, _, _, _, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                    transform.rotation = Quat::from_euler(EulerRot::XYZ, rx_val, ry_val, rz_val);
                                }
                            }
                            ui.end_row();
                        });
                });
        });
        return;
    }

    let first_entity = selected_entities[0];
    let selection_salt = first_entity.to_bits();

    let (
        first_transform_val,
        first_name_str,
        first_shape_opt_val,
        first_color,
        first_phys_enabled,
        first_bounciness,
        first_player_can_collide,
        first_friction,
        first_gravity_scale,
        first_mass,
        has_phys_opt,
    ) = {
        let Ok((
            _,
            first_transform,
            first_name,
            _,
            _,
            _,
            first_shape_opt,
            _,
            _,
            first_mat_opt,
            first_studs_mat_opt,
            first_phys_opt,
        )) = properties_query.get(first_entity)
        else {
            return;
        };

        let first_name_str = first_name.to_string();
        let first_shape_opt_val = first_shape_opt.as_ref().map(|s| s.shape);

        let mut first_color = Color::srgb(0.84, 0.24, 0.16);
        if let Ok(bc) = brick_colors.get(first_entity) {
            first_color = bc.color;
        } else if let Some(studs_mat_handle) = first_studs_mat_opt {
            if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                first_color = mat.base.base_color;
            }
        } else if let Some(mat_handle) = first_mat_opt {
            if let Some(mat) = materials.get(&mat_handle.0) {
                first_color = mat.base.base_color;
            }
        }

        let (
            first_phys_enabled,
            first_bounciness,
            first_player_can_collide,
            first_friction,
            first_gravity_scale,
            first_mass,
        ) = if let Some(phys) = first_phys_opt {
            (
                phys.enabled,
                phys.bounciness,
                phys.player_can_collide,
                phys.friction,
                phys.gravity_scale,
                phys.mass,
            )
        } else {
            (true, 0.3, true, 0.3, 1.0, 1.0)
        };

        (
            *first_transform,
            first_name_str,
            first_shape_opt_val,
            first_color,
            first_phys_enabled,
            first_bounciness,
            first_player_can_collide,
            first_friction,
            first_gravity_scale,
            first_mass,
            first_phys_opt.is_some(),
        )
    };

    let first_pos = first_transform_val.translation / 0.28;
    let first_shape =
        first_shape_opt_val.unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
    let first_size = first_transform_val.scale * first_shape.base_size_studs();
    let first_rot = first_transform_val.rotation;
    let first_alpha = first_color.to_srgba().alpha;
    let first_show_studs = studs_query
        .get(first_entity)
        .map(|s| s.enabled)
        .unwrap_or(true);

    let mut all_names_same = true;
    let mut all_pos_x_same = true;
    let mut all_pos_y_same = true;
    let mut all_pos_z_same = true;
    let mut all_scale_x_same = true;
    let mut all_scale_y_same = true;
    let mut all_scale_z_same = true;
    let mut all_rot_same = true;
    let mut all_color_same = true;
    let mut all_transparency_same = true;
    let mut all_shape_same = true;
    let mut all_show_studs_same = true;
    let mut all_phys_enabled_same = true;
    let mut all_bounciness_same = true;
    let mut all_player_can_collide_same = true;
    let mut all_friction_same = true;
    let mut all_gravity_scale_same = true;
    let mut all_mass_same = true;

    for &entity in &selected_entities[1..] {
        if let Ok((
            _,
            transform,
            name,
            _,
            _,
            _,
            shape_opt,
            _,
            _,
            mat_opt,
            studs_mat_opt,
            phys_opt,
        )) = properties_query.get(entity)
        {
            if name.to_string() != first_name_str {
                all_names_same = false;
            }
            let pos = transform.translation / 0.28;
            if (pos.x - first_pos.x).abs() > 0.001 {
                all_pos_x_same = false;
            }
            if (pos.y - first_pos.y).abs() > 0.001 {
                all_pos_y_same = false;
            }
            if (pos.z - first_pos.z).abs() > 0.001 {
                all_pos_z_same = false;
            }

            let shape = shape_opt
                .map(|s| s.shape)
                .unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
            let size = transform.scale * shape.base_size_studs();
            if (size.x - first_size.x).abs() > 0.001 {
                all_scale_x_same = false;
            }
            if (size.y - first_size.y).abs() > 0.001 {
                all_scale_y_same = false;
            }
            if (size.z - first_size.z).abs() > 0.001 {
                all_scale_z_same = false;
            }

            let rot = transform.rotation;
            if rot.dot(first_rot).abs() < 0.999 {
                all_rot_same = false;
            }

            if shape != first_shape {
                all_shape_same = false;
            }

            if studs_query.get(entity).map(|s| s.enabled).unwrap_or(true) != first_show_studs {
                all_show_studs_same = false;
            }

            let mut color = Color::srgb(0.84, 0.24, 0.16);
            if let Ok(bc) = brick_colors.get(entity) {
                color = bc.color;
            } else if let Some(studs_mat_handle) = studs_mat_opt {
                if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                    color = mat.base.base_color;
                }
            } else if let Some(mat_handle) = mat_opt {
                if let Some(mat) = materials.get(&mat_handle.0) {
                    color = mat.base.base_color;
                }
            }
            if color != first_color {
                all_color_same = false;
            }
            if (color.to_srgba().alpha - first_alpha).abs() > 0.001 {
                all_transparency_same = false;
            }

            let (phys_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) =
                if let Some(phys) = phys_opt {
                    (
                        phys.enabled,
                        phys.bounciness,
                        phys.player_can_collide,
                        phys.friction,
                        phys.gravity_scale,
                        phys.mass,
                    )
                } else {
                    (true, 0.3, true, 0.3, 1.0, 1.0)
                };
            if phys_enabled != first_phys_enabled {
                all_phys_enabled_same = false;
            }
            if (bounciness - first_bounciness).abs() > 0.001 {
                all_bounciness_same = false;
            }
            if player_can_collide != first_player_can_collide {
                all_player_can_collide_same = false;
            }
            if (friction - first_friction).abs() > 0.001 {
                all_friction_same = false;
            }
            if (gravity_scale - first_gravity_scale).abs() > 0.001 {
                all_gravity_scale_same = false;
            }
            if (mass - first_mass).abs() > 0.001 {
                all_mass_same = false;
            }
        }
    }

    let first_srgba = first_color.to_srgba();
    let mut color_array = [
        (first_srgba.red.clamp(0.0, 1.0) * 255.0).round() as u8,
        (first_srgba.green.clamp(0.0, 1.0) * 255.0).round() as u8,
        (first_srgba.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
    ];

    ui.push_id(selection_salt, |ui| {
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
                                    commands.entity(entity).insert(Name::new(name_str.clone()));
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
                                new_px = draw_coord_edit(ui, "X", &mut pos_studs.x, all_pos_x_same, "pos_x");
                                new_py = draw_coord_edit(ui, "Y", &mut pos_studs.y, all_pos_y_same, "pos_y");
                                new_pz = draw_coord_edit(ui, "Z", &mut pos_studs.z, all_pos_z_same, "pos_z");
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
                            let mut scale_val = first_size;
                            let mut new_sx = None;
                            let mut new_sy = None;
                            let mut new_sz = None;
                            ui.horizontal(|ui| {
                                new_sx = draw_coord_edit(ui, "X", &mut scale_val.x, all_scale_x_same, "size_x");
                                new_sy = draw_coord_edit(ui, "Y", &mut scale_val.y, all_scale_y_same, "size_y");
                                new_sz = draw_coord_edit(ui, "Z", &mut scale_val.z, all_scale_z_same, "size_z");
                            });
                            if new_sx.is_some() || new_sy.is_some() || new_sz.is_some() {
                                for &entity in selected_entities {
                                    if let Ok((_, mut transform, _, _, _, _, shape_opt, _, _, _, _, _)) = properties_query.get_mut(entity) {
                                        let base = shape_opt.as_ref().map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block).base_size_studs();
                                        if let Some(x) = new_sx { transform.scale.x = (x / base.x).max(0.01); }
                                        if let Some(y) = new_sy { transform.scale.y = (y / base.y).max(0.01); }
                                        if let Some(z) = new_sz { transform.scale.z = (z / base.z).max(0.01); }
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
                                new_rx = draw_coord_edit(ui, "X", &mut rot_deg.x, all_rot_same, "rot_x");
                                new_ry = draw_coord_edit(ui, "Y", &mut rot_deg.y, all_rot_same, "rot_y");
                                new_rz = draw_coord_edit(ui, "Z", &mut rot_deg.z, all_rot_same, "rot_z");
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
                                let color_btn = ui
                                    .push_id("brick_color_btn", |ui| {
                                        ui.color_edit_button_srgb(&mut color_array)
                                    })
                                    .inner;
                                if !all_color_same {
                                    ui.label(egui::RichText::new("Mixed").italics().color(egui::Color32::from_rgb(120, 120, 120)));
                                }
                                if color_btn.changed() {
                                    let (nr, ng, nb) = (color_array[0], color_array[1], color_array[2]);
                                    for &entity in selected_entities {
                                        if let Ok((_, _, _, _, _, _, _, _, _, mat_opt, studs_mat_opt, _)) = properties_query.get(entity) {
                                            let alpha = current_brick_color(entity, brick_colors, materials, studs_materials, mat_opt, studs_mat_opt).to_srgba().alpha;
                                            let new_color = Color::Srgba(Srgba::new(
                                                nr as f32 / 255.0,
                                                ng as f32 / 255.0,
                                                nb as f32 / 255.0,
                                                alpha,
                                            ));
                                            crate::common::game::bricks::swap_brick_material(
                                                commands,
                                                entity,
                                                studs_mat_opt.is_some(),
                                                material_cache,
                                                studs_materials,
                                                materials,
                                                studs_assets,
                                                new_color,
                                            );
                                            write_brick_color(commands, brick_colors, entity, new_color);
                                        }
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label(egui::RichText::new("Transparency").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            ui.horizontal(|ui| {
                                let mut transparency = 1.0 - first_color.to_srgba().alpha;
                                if all_transparency_same {
                                    let slider_res = ui.add(egui::Slider::new(&mut transparency, 0.0..=1.0).step_by(0.01));
                                    if slider_res.changed() {
                                        for &entity in selected_entities {
                                            if let Ok((_, _, _, _, _, _, _, _, _, mat_opt, studs_mat_opt, _)) = properties_query.get(entity) {
                                                let current_color = current_brick_color(entity, brick_colors, materials, studs_materials, mat_opt, studs_mat_opt);
                                                let mut srgba = current_color.to_srgba();
                                                srgba.alpha = 1.0 - transparency;
                                                let new_color = Color::Srgba(srgba);
                                                crate::common::game::bricks::swap_brick_material(
                                                    commands,
                                                    entity,
                                                    studs_mat_opt.is_some(),
                                                    material_cache,
                                                    studs_materials,
                                                    materials,
                                                    studs_assets,
                                                    new_color,
                                                );
                                                write_brick_color(commands, brick_colors, entity, new_color);
                                            }
                                        }
                                    }
                                } else {
                                    let mut clicked = false;
                                    if ui.button("Mixed (Click to set)").clicked() {
                                        clicked = true;
                                        transparency = 0.0;
                                    }
                                    if clicked {
                                        for &entity in selected_entities {
                                            if let Ok((_, _, _, _, _, _, _, _, _, mat_opt, studs_mat_opt, _)) = properties_query.get(entity) {
                                                let current_color = current_brick_color(entity, brick_colors, materials, studs_materials, mat_opt, studs_mat_opt);
                                                let mut srgba = current_color.to_srgba();
                                                srgba.alpha = 1.0 - transparency;
                                                let new_color = Color::Srgba(srgba);
                                                crate::common::game::bricks::swap_brick_material(
                                                    commands,
                                                    entity,
                                                    studs_mat_opt.is_some(),
                                                    material_cache,
                                                    studs_materials,
                                                    materials,
                                                    studs_assets,
                                                    new_color,
                                                );
                                                write_brick_color(commands, brick_colors, entity, new_color);
                                            }
                                        }
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label(egui::RichText::new("Show Studs").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                            ui.add_enabled_ui(workspace_studs.enabled, |ui| {
                                if all_show_studs_same {
                                    let mut show_studs = first_show_studs;
                                    if ui.checkbox(&mut show_studs, "").changed() {
                                        for &entity in selected_entities {
                                            commands.entity(entity).insert(crate::common::game::bricks::components::BrickStuds { enabled: show_studs });
                                        }
                                    }
                                } else {
                                    let mut clicked = false;
                                    ui.horizontal(|ui| {
                                        if ui.button("Mixed (Click to set)").clicked() {
                                            clicked = true;
                                        }
                                    });
                                    if clicked {
                                        for &entity in selected_entities {
                                            commands.entity(entity).insert(crate::common::game::bricks::components::BrickStuds { enabled: true });
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
                                        if ui.selectable_value(&mut current_shape, crate::common::game::bricks::components::BrickShape::Cylinder, "Cylinder").clicked() {
                                            selection_changed = true;
                                        }
                                        if ui.selectable_value(&mut current_shape, crate::common::game::bricks::components::BrickShape::Wedge, "Wedge").clicked() {
                                            selection_changed = true;
                                        }
                                        if ui.selectable_value(&mut current_shape, crate::common::game::bricks::components::BrickShape::CornerWedge, "CornerWedge").clicked() {
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

                                ui.label(egui::RichText::new("Player can collide").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                                let mut player_can_collide = first_player_can_collide;
                                let can_collide_checkbox_res = if all_player_can_collide_same {
                                    Some(ui.checkbox(&mut player_can_collide, ""))
                                } else {
                                    let mut clicked = false;
                                    ui.horizontal(|ui| {
                                        if ui.button("Mixed (Click to set)").clicked() {
                                            clicked = true;
                                            player_can_collide = true;
                                        }
                                    });
                                    if clicked {
                                        for &entity in selected_entities {
                                            if let Ok((_, _, _, _, _, _, _, _, _, _, _, Some(mut phys))) = properties_query.get_mut(entity) {
                                                phys.player_can_collide = player_can_collide;
                                                let layers = if player_can_collide {
                                                    CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                                                } else {
                                                    CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                                                };
                                                commands.entity(entity).insert(layers);
                                            }
                                        }
                                    }
                                    None
                                };
                                if let Some(res) = can_collide_checkbox_res {
                                    if res.changed() {
                                        for &entity in selected_entities {
                                            if let Ok((_, _, _, _, _, _, _, _, _, _, _, Some(mut phys))) = properties_query.get_mut(entity) {
                                                phys.player_can_collide = player_can_collide;
                                                let layers = if player_can_collide {
                                                    CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                                                } else {
                                                    CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                                                };
                                                commands.entity(entity).insert(layers);
                                            }
                                        }
                                    }
                                }
                                ui.end_row();

                                ui.label(egui::RichText::new("Friction").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                                let mut friction = first_friction;
                                let mut new_friction = None;
                                ui.horizontal(|ui| {
                                    new_friction = draw_coord_edit(ui, "", &mut friction, all_friction_same, "friction");
                                });
                                if let Some(new_val) = new_friction {
                                    for &entity in selected_entities {
                                        if let Ok((_, _, _, _, _, _, _, _, _, _, _, Some(mut phys))) = properties_query.get_mut(entity) {
                                            phys.friction = new_val.clamp(0.0, 1.0);
                                            commands.entity(entity).insert(Friction::new(phys.friction));
                                        }
                                    }
                                }
                                ui.end_row();

                                ui.label(egui::RichText::new("Gravity Scale").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                                let mut gravity_scale = first_gravity_scale;
                                let mut new_gravity_scale = None;
                                ui.horizontal(|ui| {
                                    new_gravity_scale = draw_coord_edit(ui, "", &mut gravity_scale, all_gravity_scale_same, "gravity_scale");
                                });
                                if let Some(new_val) = new_gravity_scale {
                                    for &entity in selected_entities {
                                        if let Ok((_, _, _, _, _, _, _, _, _, _, _, Some(mut phys))) = properties_query.get_mut(entity) {
                                            phys.gravity_scale = new_val;
                                            commands.entity(entity).insert(GravityScale(phys.gravity_scale));
                                        }
                                    }
                                }
                                ui.end_row();

                                ui.label(egui::RichText::new("Mass").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                                let mut mass = first_mass;
                                let mut new_mass = None;
                                ui.horizontal(|ui| {
                                    new_mass = draw_coord_edit(ui, "", &mut mass, all_mass_same, "mass");
                                });
                                if let Some(new_val) = new_mass {
                                    for &entity in selected_entities {
                                        if let Ok((_, _, _, _, _, _, _, _, _, _, _, Some(mut phys))) = properties_query.get_mut(entity) {
                                            phys.mass = new_val.max(0.001);
                                            commands.entity(entity).insert(Mass(phys.mass));
                                        }
                                    }
                                }
                                ui.end_row();

                                ui.label(egui::RichText::new("Bounciness").color(egui::Color32::from_rgb(60, 60, 60)).size(13.0));
                                let mut bounciness_val = first_bounciness;
                                let mut new_bounciness = None;
                                ui.horizontal(|ui| {
                                    new_bounciness = draw_coord_edit(ui, "", &mut bounciness_val, all_bounciness_same, "bounciness");
                                });
                                
                                if let Some(new_val) = new_bounciness {
                                    for &entity in selected_entities {
                                        if let Ok((_, _, _, _, _, _, _, _, _, _, _, Some(mut phys))) = properties_query.get_mut(entity) {
                                            phys.bounciness = new_val.clamp(0.0, 1.0);
                                            commands.entity(entity).insert(Restitution::new(phys.bounciness));
                                        }
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    });
}

pub fn draw_workspace_properties(
    ui: &mut egui::Ui,
    gravity: &mut Option<ResMut<'_, avian3d::prelude::Gravity>>,
    workspace_studs: &mut ResMut<'_, crate::common::game::bricks::WorkspaceShowStuds>,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Properties")
                .color(egui::Color32::from_rgb(0, 0, 0))
                .strong()
                .size(16.0),
        );
    });

    ui.add_space(8.0);
    let (sep_rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
    ui.add_space(8.0);

    egui::CollapsingHeader::new(
        egui::RichText::new("Display")
            .color(egui::Color32::from_rgb(0, 0, 0))
            .strong()
            .size(14.0),
    )
    .default_open(true)
    .show(ui, |ui| {
        egui::Grid::new("properties_workspace_display_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Show Studs")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.checkbox(&mut workspace_studs.enabled, "");
                ui.end_row();
            });
    });

    ui.add_space(8.0);

    egui::CollapsingHeader::new(
        egui::RichText::new("Physics")
            .color(egui::Color32::from_rgb(0, 0, 0))
            .strong()
            .size(14.0),
    )
    .default_open(true)
    .show(ui, |ui| {
        egui::Grid::new("properties_workspace_physics_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("World Gravity")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                if let Some(g) = gravity {
                    let mut gravity_studs = -g.0.y / 0.28;
                    if ui
                        .add(
                            egui::DragValue::new(&mut gravity_studs)
                                .speed(1.0)
                                .range(0.0..=10000.0)
                                .suffix(" studs/s²"),
                        )
                        .changed()
                    {
                        g.0 = Vec3::new(0.0, -gravity_studs * 0.28, 0.0);
                    }
                } else {
                    ui.label(
                        egui::RichText::new("Gravity resource not found")
                            .color(egui::Color32::from_rgb(180, 60, 60))
                            .size(13.0),
                    );
                }
                ui.end_row();
            });
    });
}

pub fn draw_lighting_properties(
    ui: &mut egui::Ui,
    lighting_config: &mut ResMut<'_, crate::client::sky::LightingConfig>,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Properties")
                .color(egui::Color32::from_rgb(0, 0, 0))
                .strong()
                .size(16.0),
        );
    });

    ui.add_space(8.0);
    let (sep_rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
    ui.add_space(8.0);

    egui::CollapsingHeader::new(
        egui::RichText::new("Lighting")
            .color(egui::Color32::from_rgb(0, 0, 0))
            .strong()
            .size(14.0),
    )
    .default_open(true)
    .show(ui, |ui| {
        egui::Grid::new("properties_lighting_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Time of Day")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.time_of_day, 0.0..=24.0)
                        .step_by(0.05)
                        .custom_formatter(|val, _| {
                            let h = (val.floor() as u32) % 24;
                            let m = ((val - val.floor()) * 60.0).round() as u32;
                            format!("{:02}:{:02} ({:.2}h)", h, m, val)
                        }),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Latitude")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(egui::Slider::new(&mut lighting_config.latitude, -90.0..=90.0).step_by(0.5));
                ui.end_row();

                ui.label(
                    egui::RichText::new("Sun Angular Radius")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.sun_angular_radius, 0.005..=0.1)
                        .step_by(0.001),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Moon Angular Radius")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.moon_angular_radius, 0.005..=0.1)
                        .step_by(0.001),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Star Density")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.star_density, 0.0..=1.0).step_by(0.01),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Night Ambient")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                let night_ambient_srgba = lighting_config.night_ambient.to_srgba();
                let mut ambient_rgba = [
                    (night_ambient_srgba.red.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (night_ambient_srgba.green.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (night_ambient_srgba.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (night_ambient_srgba.alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
                ];
                if ui
                    .color_edit_button_srgba_unmultiplied(&mut ambient_rgba)
                    .changed()
                {
                    lighting_config.night_ambient = Color::Srgba(Srgba::new(
                        ambient_rgba[0] as f32 / 255.0,
                        ambient_rgba[1] as f32 / 255.0,
                        ambient_rgba[2] as f32 / 255.0,
                        ambient_rgba[3] as f32 / 255.0,
                    ));
                }
                ui.end_row();

                ui.label(
                    egui::RichText::new("Sun Brightness")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.sun_illuminance, 0.0..=50000.0)
                        .step_by(500.0),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Moon Brightness")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.moon_illuminance, 0.0..=1000.0)
                        .step_by(10.0),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Ambient Brightness")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.ambient_brightness, 0.0..=5.0)
                        .step_by(0.05),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Fog Density")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.fog_density, 0.0..=5.0).step_by(0.05),
                );
                ui.end_row();
            });
    });

    egui::CollapsingHeader::new(
        egui::RichText::new("Procedural Clouds")
            .color(egui::Color32::from_rgb(0, 0, 0))
            .strong()
            .size(14.0),
    )
    .default_open(true)
    .show(ui, |ui| {
        egui::Grid::new("properties_lighting_clouds_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Volumetric Clouds")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.checkbox(&mut lighting_config.volumetric_clouds, "");
                ui.end_row();

                ui.label(
                    egui::RichText::new("Render Scale")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_render_scale, 0.25..=1.0)
                        .step_by(0.05),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Raymarch Steps")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(egui::Slider::new(
                    &mut lighting_config.cloud_raymarch_steps,
                    48..=100,
                ));
                ui.end_row();

                ui.label(
                    egui::RichText::new("Shadow Steps")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(egui::Slider::new(
                    &mut lighting_config.cloud_shadow_steps,
                    1..=50,
                ));
                ui.end_row();

                ui.label(
                    egui::RichText::new("Planet Radius")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(egui::Slider::new(
                    &mut lighting_config.planet_radius,
                    5e4..=1e7,
                ));
                ui.end_row();

                ui.label(
                    egui::RichText::new("Cloud Bottom Height")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_bottom_height, 1.0..=5e3)
                        .step_by(10.0),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Cloud Top Height")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_top_height, 1.0..=5e3)
                        .step_by(10.0),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Coverage")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_coverage, 0.0..=1.0).step_by(0.01),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Density")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_density, 0.001..=1.0)
                        .step_by(0.001),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Detail Strength")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_detail_strength, 0.0..=1.0)
                        .step_by(0.01),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Base Edge Softness")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_base_edge_softness, 0.0..=1.0)
                        .step_by(0.01),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Bottom Softness")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_bottom_softness, 0.01..=10.0)
                        .step_by(0.05),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Base Scale")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_base_scale, 0.1..=100.0)
                        .step_by(0.1),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Detail Scale")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_detail_scale, 1.0..=100.0)
                        .step_by(1.0),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Shadow Step Size")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_shadow_step_size, 1.0..=100.0)
                        .step_by(1.0),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Shadow Step Multiply")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_shadow_step_multiply, 0.1..=10.0)
                        .step_by(0.1),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Forward Scattering G")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(
                        &mut lighting_config.cloud_forward_scattering_g,
                        -10.0..=10.0,
                    )
                    .step_by(0.1),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Backward Scattering G")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(
                        &mut lighting_config.cloud_backward_scattering_g,
                        -10.0..=10.0,
                    )
                    .step_by(0.1),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Scattering Lerp")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_scattering_lerp, 0.01..=100.0)
                        .step_by(0.1),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Min Transmittance")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_min_transmittance, 0.01..=100.0)
                        .step_by(0.01),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Reprojection Strength")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_reprojection_strength, 0.0..=1.0)
                        .step_by(0.01),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Wind Velocity X")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_wind_velocity.x, -100.0..=100.0)
                        .step_by(0.1),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Wind Velocity Y")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_wind_velocity.y, -100.0..=100.0)
                        .step_by(0.1),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Wind Velocity Z")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                ui.add(
                    egui::Slider::new(&mut lighting_config.cloud_wind_velocity.z, -100.0..=100.0)
                        .step_by(0.1),
                );
                ui.end_row();
            });
    });
}

pub fn draw_players_properties(
    ui: &mut egui::Ui,
    players_service: &mut Option<ResMut<'_, crate::studio::tools::PlayersService>>,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Properties")
                .color(egui::Color32::from_rgb(0, 0, 0))
                .strong()
                .size(16.0),
        );
    });

    ui.add_space(8.0);
    let (sep_rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
    ui.add_space(8.0);

    egui::CollapsingHeader::new(
        egui::RichText::new("Basic")
            .color(egui::Color32::from_rgb(0, 0, 0))
            .strong()
            .size(14.0),
    )
    .default_open(true)
    .show(ui, |ui| {
        egui::Grid::new("properties_players_basic_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Player Speed")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                if let Some(service) = players_service {
                    let mut speed_studs = service.speed / 0.28;
                    if ui
                        .add(
                            egui::DragValue::new(&mut speed_studs)
                                .speed(0.5)
                                .range(0.0..=1000.0)
                                .suffix(" studs/s"),
                        )
                        .changed()
                    {
                        service.speed = speed_studs * 0.28;
                        if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write()
                        {
                            shared.speed = service.speed;
                        }
                    }
                } else {
                    ui.label(
                        egui::RichText::new("Players resource not found")
                            .color(egui::Color32::from_rgb(180, 60, 60))
                            .size(13.0),
                    );
                }
                ui.end_row();

                ui.label(
                    egui::RichText::new("Player Jump Power")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                if let Some(service) = players_service {
                    let mut jump_power_studs = service.jump_power / 0.28;
                    if ui
                        .add(
                            egui::DragValue::new(&mut jump_power_studs)
                                .speed(1.0)
                                .range(0.0..=1000.0)
                                .suffix(" studs/s"),
                        )
                        .changed()
                    {
                        service.jump_power = jump_power_studs * 0.28;
                        if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write()
                        {
                            shared.jump_power = service.jump_power;
                        }
                    }
                } else {
                    ui.label(
                        egui::RichText::new("Players resource not found")
                            .color(egui::Color32::from_rgb(180, 60, 60))
                            .size(13.0),
                    );
                }
                ui.end_row();

                ui.label(
                    egui::RichText::new("Player Gravity")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                if let Some(service) = players_service {
                    let mut gravity_studs = service.gravity / 0.28;
                    if ui
                        .add(
                            egui::DragValue::new(&mut gravity_studs)
                                .speed(1.0)
                                .range(0.0..=10000.0)
                                .suffix(" studs/s²"),
                        )
                        .changed()
                    {
                        service.gravity = gravity_studs * 0.28;
                        if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write()
                        {
                            shared.gravity = service.gravity;
                        }
                    }
                } else {
                    ui.label(
                        egui::RichText::new("Players resource not found")
                            .color(egui::Color32::from_rgb(180, 60, 60))
                            .size(13.0),
                    );
                }
                ui.end_row();

                ui.label(
                    egui::RichText::new("Speed Response")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                if let Some(service) = players_service {
                    let mut response = service.speed_response;
                    egui::ComboBox::from_id_salt("players_speed_response")
                        .selected_text(match response {
                            crate::common::game::movement::SpeedResponse::Linear => "Linear",
                            crate::common::game::movement::SpeedResponse::Exponential => {
                                "Exponential"
                            }
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut response,
                                crate::common::game::movement::SpeedResponse::Linear,
                                "Linear",
                            );
                            ui.selectable_value(
                                &mut response,
                                crate::common::game::movement::SpeedResponse::Exponential,
                                "Exponential",
                            );
                        });
                    if response != service.speed_response {
                        service.speed_response = response;
                        if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write()
                        {
                            shared.speed_response = response;
                        }
                    }
                } else {
                    ui.label(
                        egui::RichText::new("Players resource not found")
                            .color(egui::Color32::from_rgb(180, 60, 60))
                            .size(13.0),
                    );
                }
                ui.end_row();

                ui.label(
                    egui::RichText::new("Player Friction")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                if let Some(service) = players_service {
                    if ui
                        .add(
                            egui::DragValue::new(&mut service.friction)
                                .speed(0.05)
                                .range(0.0..=1.0),
                        )
                        .changed()
                    {
                        if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write()
                        {
                            shared.friction = service.friction;
                        }
                    }
                } else {
                    ui.label(
                        egui::RichText::new("Players resource not found")
                            .color(egui::Color32::from_rgb(180, 60, 60))
                            .size(13.0),
                    );
                }
                ui.end_row();

                ui.label(
                    egui::RichText::new("Player Bounciness")
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .size(13.0),
                );
                if let Some(service) = players_service {
                    if ui
                        .add(
                            egui::DragValue::new(&mut service.bounciness)
                                .speed(0.05)
                                .range(0.0..=1.0),
                        )
                        .changed()
                    {
                        if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write()
                        {
                            shared.bounciness = service.bounciness;
                        }
                    }
                } else {
                    ui.label(
                        egui::RichText::new("Players resource not found")
                            .color(egui::Color32::from_rgb(180, 60, 60))
                            .size(13.0),
                    );
                }
                ui.end_row();
            });
    });
}
