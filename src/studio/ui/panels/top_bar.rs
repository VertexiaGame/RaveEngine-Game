use crate::common::game::bricks::data::BrickSpawnerCount;
use crate::common::game::bricks::data::spawn_brick;
use crate::common::game::bricks::studs::StudsAssets;
use crate::common::game::bricks::BRICK_METALLIC;
use crate::common::game::bricks::BRICK_PERCEPTUAL_ROUGHNESS;
use crate::common::game::bricks::BRICK_REFLECTANCE;
use crate::studio::tools::{Selection, SnapConfig, ToolState};
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy_egui::egui;

#[allow(deprecated)]
pub fn draw_top_bar(
    ui: &mut egui::Ui,
    next_tool: &mut NextState<ToolState>,
    current_tool: &State<ToolState>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<
        Assets<
            ExtendedMaterial<
                StandardMaterial,
                crate::common::game::bricks::studs::ShadowOpacityExtension,
            >,
        >,
    >,
    studs_materials: &mut ResMut<
        Assets<
            ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>,
        >,
    >,
    studs_assets: &StudsAssets,
    count: &mut ResMut<BrickSpawnerCount>,
    snap_config: &mut ResMut<SnapConfig>,
    move_tex: egui::TextureId,
    rotate_tex: egui::TextureId,
    scale_tex: egui::TextureId,
    add_tex: egui::TextureId,
    play_tex: egui::TextureId,
    playc_tex: egui::TextureId,
    stopp_tex: egui::TextureId,
    image_tex: egui::TextureId,
    mesh_tex: egui::TextureId,
    brick_tex: egui::TextureId,
    script_tex: egui::TextureId,
    localscript_tex: egui::TextureId,
    modulescript_tex: egui::TextureId,
    diagnostics: &Res<bevy::diagnostic::DiagnosticsStore>,
    camera_transform: Option<&Transform>,
    action_writer: &mut MessageWriter<crate::studio::tools::UndoRedoAction>,
    history: &mut ResMut<crate::studio::tools::UndoRedoHistory>,
    physics_state: crate::common::game::physics::PhysicsSimulationState,
    physics_action_writer: &mut MessageWriter<
        crate::common::game::physics::PhysicsSimulationAction,
    >,
    settings_window: &mut ResMut<crate::studio::ui::SettingsWindow>,
    graphics_settings: &mut crate::studio::ui::GraphicsSettings,
    lighting_config: &mut ResMut<crate::client::sky::LightingConfig>,
    gravity: &mut Option<ResMut<avian3d::prelude::Gravity>>,
    camera_transform_query: &mut Query<&mut Transform, With<Camera3d>>,
    entities_query: &mut Query<
        (
            Entity,
            &mut Transform,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&crate::common::game::bricks::components::Brick>,
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
    onboarding_data: &mut crate::studio::ui::panels::onboarding::OnboardingData,
    _play_processes: &mut ResMut<crate::studio::ui::resources::PlayInClientProcesses>,
    playtest_state: &mut ResMut<crate::client::PlaytestState>,
    playtest_backup: &mut ResMut<crate::studio::ui::resources::PlaytestBackup>,
    playtest_client_query: &Query<
        Entity,
        With<crate::studio::ui::resources::InEditorPlaytestClient>,
    >,
    selection: &mut Selection,
    explorer_query: &Query<
        (
            Entity,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::scripting::ecs::ServerScript>,
            Option<&crate::scripting::ecs::LocalScript>,
            Option<&crate::scripting::ecs::ModuleScript>,
            Option<&crate::common::game::assets::components::Image>,
            Option<&crate::common::game::assets::components::Texture>,
            Option<&crate::common::game::assets::components::Mesh>,
        ),
        Without<Camera3d>,
    >,
    onboarding_active: bool,
    players_service: &mut Option<ResMut<crate::studio::tools::PlayersService>>,
    file_dialog_state: &crate::studio::ui::resources::FileDialogState,
    studs_query: &Query<&crate::common::game::bricks::components::BrickStuds>,
    brick_colors: &Query<&mut crate::common::game::bricks::components::BrickColor>,
    mesh_assets: &Query<&crate::common::game::assets::components::Mesh>,
    workspace_studs: &mut ResMut<crate::common::game::bricks::WorkspaceShowStuds>,
    images_query: &Query<
        (
            Entity,
            &Name,
            Option<&ChildOf>,
            Option<&crate::common::game::assets::components::Image>,
        ),
        Without<Camera3d>,
    >,
    replicated_images: &Query<
        Entity,
        (
            With<crate::common::game::assets::components::Image>,
            With<lightyear::prelude::Replicate>,
        ),
    >,
    copied_buffer: &mut ResMut<crate::studio::ui::CopiedEntityBuffer>,
    texture_assets: &Query<&crate::common::game::assets::components::Texture>,
) {
    ui.style_mut().interaction.selectable_labels = false;

    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(16.0, 0.0);

                let file_id = ui.make_persistent_id("file_menu_btn");
                let is_file_hovered = ui.data(|d| d.get_temp::<bool>(file_id)).unwrap_or(false);
                let file_text_color = if is_file_hovered {
                    egui::Color32::from_rgb(80, 160, 240)
                } else {
                    egui::Color32::from_rgb(60, 60, 60)
                };

                let mut is_hovered = false;
                ui.scope(|ui| {
                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;

                    ui.style_mut().visuals.widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

                    let file_button_res = egui::menu::menu_button(ui, egui::RichText::new("File").color(file_text_color).size(13.0), |ui| {
                        let save_enabled = !onboarding_data.save_path.is_empty();
                        if ui.add_enabled(save_enabled, egui::Button::new("Save")).clicked() {
                            let mut bricks_data = Vec::new();
                            for (entity, transform, name, _, _, brick_opt, shape_opt, _, _, mat_opt, studs_mat_opt, phys_opt) in entities_query.iter() {
                                if brick_opt.is_some() {
                                    let shape = shape_opt.as_ref().map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                                    let mut current_color = Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0));
                                    if let Some(studs_mat_handle) = studs_mat_opt {
                                        if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                                            current_color = mat.base.base_color;
                                        }
                                    } else if let Some(mat_handle) = mat_opt {
                                        if let Some(mat) = materials.get(&mat_handle.0) {
                                            current_color = mat.base.base_color;
                                        }
                                    }
                                    let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = if let Some(phys) = phys_opt {
                                        (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass)
                                    } else {
                                        (true, 0.3, true, 0.3, 1.0, 1.0)
                                    };
                                    bricks_data.push(crate::common::core::vrtx::VrtxBrick {
                                        name: name.to_string(),
                                        transform: *transform,
                                        shape,
                                        color: current_color,
                                        physics_enabled,
                                        bounciness,
                                        player_can_collide,
                                        friction,
                                        gravity_scale,
                                        mass,
                                        show_studs: studs_query.get(entity).map(|s| s.enabled).unwrap_or(true),
                                    });
                                }
                            }

                            let mut scripts_data = Vec::new();
                            for (_entity, name, child_of_opt, _, _, s_opt, l_opt, m_opt, _, _, _) in explorer_query.iter() {
                                let mut script_type_opt = None;
                                let mut code = String::new();
                                let mut enabled = true;
                                if let Some(s) = s_opt {
                                    script_type_opt = Some(0);
                                    code = s.code.clone();
                                    enabled = s.enabled;
                                } else if let Some(l) = l_opt {
                                    script_type_opt = Some(1);
                                    code = l.code.clone();
                                    enabled = l.enabled;
                                } else if let Some(m) = m_opt {
                                    script_type_opt = Some(2);
                                    code = m.code.clone();
                                }
                                if let Some(script_type) = script_type_opt {
                                let mut parent_name = None;
                                if let Some(child_of) = child_of_opt {
                                    if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) {
                                        parent_name = Some(p_name.to_string());
                                    }
                                }
                                scripts_data.push(crate::common::core::vrtx::VrtxScript {
                                        name: name.to_string(),
                                        script_type,
                                        code,
                                        parent_name,
                                        enabled,
                                    });
                                }
                            }

                            let mut images_data = Vec::new();
                            for (_entity, name, child_of_opt, image_opt) in images_query.iter() {
                                if let Some(image) = image_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt {
                                        if let Ok((_, p_name, _, _)) = images_query.get(child_of.parent()) {
                                            parent_name = Some(p_name.to_string());
                                        }
                                    }
                                    let transform = entities_query
                                        .get(_entity)
                                        .map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t)
                                        .unwrap_or_default();
                                    images_data.push(crate::common::core::vrtx::VrtxImage {
                                        name: name.to_string(),
                                        asset_id: image.asset_id,
                                        face: image.face.as_ref().map(|f| f.as_str().to_string()),
                                        parent_name,
                                        transform,
                                    });
                                }
                            }

                            let mut meshes_data = Vec::new();
                            for (_entity, name, child_of_opt, _, _, _, _, _, _, _, mesh_opt) in explorer_query.iter() {
                                if let Some(mesh) = mesh_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt {
                                        if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) {
                                            parent_name = Some(p_name.to_string());
                                        }
                                    }
                                    let transform = entities_query
                                        .get(_entity)
                                        .map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t)
                                        .unwrap_or_default();
                                    let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = entities_query
                                        .get(_entity)
                                        .map(|(_, _, _, _, _, _, _, _, _, _, _, phys_opt)| {
                                            if let Some(phys) = phys_opt {
                                                (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass)
                                            } else {
                                                (true, 0.3, true, 0.3, 1.0, 1.0)
                                            }
                                        })
                                        .unwrap_or((true, 0.3, true, 0.3, 1.0, 1.0));
                                    meshes_data.push(crate::common::core::vrtx::VrtxMesh {
                                        name: name.to_string(),
                                        asset_id: mesh.asset_id,
                                        normalize: mesh.normalize,
                                        parent_name,
                                        transform,
                                        physics_enabled,
                                        bounciness,
                                        player_can_collide,
                                        friction,
                                        gravity_scale,
                                        mass,
                                    });
                                }
                            }

                            let mut textures_data = Vec::new();
                            for (_entity, name, child_of_opt, _, _, _, _, _, _, texture_opt, _) in explorer_query.iter() {
                                if let Some(texture) = texture_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt {
                                        if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) {
                                            parent_name = Some(p_name.to_string());
                                        }
                                    }
                                    textures_data.push(crate::common::core::vrtx::VrtxTexture {
                                        name: name.to_string(),
                                        id_string: texture.as_content_id(),
                                        parent_name,
                                    });
                                }
                            }

                            let gravity_val = if let Some(g) = gravity.as_ref() {
                                g.0
                            } else {
                                Vec3::new(0.0, -186.9 * 0.28, 0.0)
                            };
                            let cam_transform = if let Some(cam_t) = camera_transform_query.iter().next() {
                                *cam_t
                            } else {
                                Transform::IDENTITY
                            };
                            let players = if let Some(ps) = players_service.as_ref() {
                                crate::common::core::vrtx::VrtxPlayers {
                                    speed: ps.speed,
                                    jump_power: ps.jump_power,
                                    gravity: ps.gravity,
                                    speed_response: ps.speed_response,
                                    friction: ps.friction,
                                    bounciness: ps.bounciness,
                                }
                            } else {
                                crate::common::core::vrtx::VrtxPlayers::default()
                            };
                            let state = crate::common::core::vrtx::VrtxFileState {
                                version: crate::common::core::vrtx::FORMAT_VERSION,
                                gravity: gravity_val,
                                settings: crate::common::core::vrtx::VrtxSettings {
                                    ssao: graphics_settings.ssao,
                                    contact_shadows: graphics_settings.contact_shadows,
                                    bloom: graphics_settings.bloom,
                                },
                                lighting: crate::common::core::vrtx::VrtxLighting::from(&**lighting_config),
                                players,
                                camera_transform: cam_transform,
                                bricks: bricks_data,
                                scripts: scripts_data,
                                images: images_data,
                                meshes: meshes_data,
                                textures: textures_data,
                            };
                            let _ = state.save_to_file(&onboarding_data.save_path);
                            ui.close_menu();
                        }
                        
                        let is_open = file_dialog_state.is_open.load(std::sync::atomic::Ordering::Relaxed);
                        if ui.add_enabled(!is_open, egui::Button::new("Save As...")).clicked() {
                            file_dialog_state.is_open.store(true, std::sync::atomic::Ordering::Relaxed);
                            let tx = file_dialog_state.tx.clone();
                            std::thread::spawn(move || {
                                if let Some(path) = crate::studio::ui::resources::save_file_dialog("Rave Project") {
                                    let _ = tx.send(crate::studio::ui::resources::FileDialogResult::SaveAs(path));
                                } else {
                                    let _ = tx.send(crate::studio::ui::resources::FileDialogResult::Cancel);
                                }
                            });
                            ui.close_menu();
                        }
                        if ui.add_enabled(!is_open, egui::Button::new("Open...")).clicked() {
                            file_dialog_state.is_open.store(true, std::sync::atomic::Ordering::Relaxed);
                            let tx = file_dialog_state.tx.clone();
                            std::thread::spawn(move || {
                                if let Some(path) = crate::studio::ui::resources::pick_file_dialog("Rave Project") {
                                    let _ = tx.send(crate::studio::ui::resources::FileDialogResult::OpenFile(path));
                                } else {
                                    let _ = tx.send(crate::studio::ui::resources::FileDialogResult::Cancel);
                                }
                            });
                            ui.close_menu();
                        }
                    });
                    is_hovered = file_button_res.response.hovered();
                });
                ui.data_mut(|d| d.insert_temp(file_id, is_hovered));

                let edit_id = ui.make_persistent_id("edit_menu_btn");
                let is_edit_hovered = ui.data(|d| d.get_temp::<bool>(edit_id)).unwrap_or(false);
                let edit_text_color = if is_edit_hovered {
                    egui::Color32::from_rgb(80, 160, 240)
                } else {
                    egui::Color32::from_rgb(60, 60, 60)
                };
                let mut is_edit_hovered_flag = false;
                ui.scope(|ui| {
                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                    ui.style_mut().visuals.widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                    let edit_button_res = egui::menu::menu_button(ui, egui::RichText::new("Edit").color(edit_text_color).size(13.0), |ui| {
                        let can_undo = !history.undo_stack.is_empty();
                        if ui.add_enabled(can_undo, egui::Button::new("Undo  Ctrl+Z")).clicked() {
                            action_writer.write(crate::studio::tools::UndoRedoAction::Undo);
                            ui.close_menu();
                        }
                        let can_redo = !history.redo_stack.is_empty();
                        if ui.add_enabled(can_redo, egui::Button::new("Redo  Ctrl+Y")).clicked() {
                            action_writer.write(crate::studio::tools::UndoRedoAction::Redo);
                            ui.close_menu();
                        }
                        ui.separator();
                        let has_selection = !selection.entities.is_empty();
                        if ui.add_enabled(has_selection, egui::Button::new("Cut  Ctrl+X")).clicked() {
                            if let Some(primary) = selection.entity {
                                if let Ok((_, transform, name, _, _, brick_opt, shape_opt, _, mesh_opt, mat_opt, studs_mat_opt, phys_opt)) = entities_query.get(primary) {
                                    copied_buffer.transform = Some(*transform);
                                    copied_buffer.mesh = mesh_opt.cloned();
                                    copied_buffer.mesh_asset = mesh_assets.get(primary).ok().copied();
                                    copied_buffer.texture = texture_assets.get(primary).ok().copied();
                                    copied_buffer.material = mat_opt.cloned();
                                    copied_buffer.studs_material = studs_mat_opt.cloned();
                                    copied_buffer.name = Some(name.to_string());
                                    copied_buffer.is_brick = brick_opt.is_some();
                                    copied_buffer.shape = shape_opt.as_ref().map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                                    copied_buffer.physics = phys_opt.cloned();
                                    copied_buffer.show_studs = studs_query.get(primary).map(|s| s.enabled).unwrap_or(true);
                                    copied_buffer.color = brick_colors.get(primary).ok().map(|bc| bc.color);
                                }
                            }
                            for entity in selection.entities.drain(..).collect::<Vec<_>>() {
                                if let Some(data) = crate::common::game::bricks::data::capture_brick_data(entity, entities_query, studs_query, brick_colors, mesh_assets) {
                                    history.push_command(crate::studio::tools::UndoCommand::Delete { entity, data });
                                }
                                commands.entity(entity).try_despawn();
                            }
                            selection.entity = None;
                            selection.workspace_selected = false;
                            selection.players_selected = false;
                            selection.lighting_selected = false;
                            ui.close_menu();
                        }
                        if ui.add_enabled(has_selection, egui::Button::new("Copy  Ctrl+C")).clicked() {
                            if let Some(primary) = selection.entity {
                                if let Ok((_, transform, name, _, _, brick_opt, shape_opt, _, mesh_opt, mat_opt, studs_mat_opt, phys_opt)) = entities_query.get(primary) {
                                    copied_buffer.transform = Some(*transform);
                                    copied_buffer.mesh = mesh_opt.cloned();
                                    copied_buffer.mesh_asset = mesh_assets.get(primary).ok().copied();
                                    copied_buffer.texture = texture_assets.get(primary).ok().copied();
                                    copied_buffer.material = mat_opt.cloned();
                                    copied_buffer.studs_material = studs_mat_opt.cloned();
                                    copied_buffer.name = Some(name.to_string());
                                    copied_buffer.is_brick = brick_opt.is_some();
                                    copied_buffer.shape = shape_opt.as_ref().map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                                    copied_buffer.physics = phys_opt.cloned();
                                    copied_buffer.show_studs = studs_query.get(primary).map(|s| s.enabled).unwrap_or(true);
                                    copied_buffer.color = brick_colors.get(primary).ok().map(|bc| bc.color);
                                }
                            }
                            ui.close_menu();
                        }
                        let can_paste = copied_buffer.transform.is_some();
                        if ui.add_enabled(can_paste, egui::Button::new("Paste  Ctrl+V")).clicked() {
                            if let Some(transform) = copied_buffer.transform {
                                let name = copied_buffer.name.clone().unwrap_or_else(|| "Part".to_string());
                                let mut new_transform = transform;
                                new_transform.translation += Vec3::new(2.0 * 0.28, 0.0, 2.0 * 0.28);
                                let new_entity = commands.spawn((new_transform, Name::new(format!("{} - Copy", name)), Pickable::default())).id();
                                if let Some(ref mesh) = copied_buffer.mesh { commands.entity(new_entity).insert(mesh.clone()); }
                                if let Some(ref mesh_asset) = copied_buffer.mesh_asset { commands.entity(new_entity).insert(*mesh_asset); }
                                if let Some(texture) = copied_buffer.texture {
                                    let tex_entity = commands.spawn((Name::new("Texture"), Transform::default(), texture)).id();
                                    commands.entity(new_entity).add_child(tex_entity);
                                }
                                if let Some(ref mat) = copied_buffer.material { commands.entity(new_entity).insert(mat.clone()); }
                                if let Some(ref studs_mat) = copied_buffer.studs_material { commands.entity(new_entity).insert(studs_mat.clone()); }
                                if copied_buffer.is_brick {
                                    commands.entity(new_entity).insert((
                                        crate::common::game::bricks::components::Brick,
                                        crate::common::game::bricks::components::BrickShapeComponent { shape: copied_buffer.shape },
                                        crate::common::game::bricks::components::BrickStuds { enabled: copied_buffer.show_studs },
                                        crate::common::game::bricks::components::BrickColor { color: copied_buffer.color.unwrap_or(Color::srgb(0.84, 0.24, 0.16)) },
                                    ));
                                }
                                if let Some(phys) = copied_buffer.physics {
                                    let layers = if phys.player_can_collide { avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF) } else { avian3d::prelude::CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD) };
                                    commands.entity(new_entity).insert((phys, layers));
                                } else if copied_buffer.is_brick {
                                    commands.entity(new_entity).insert((crate::common::game::bricks::components::BrickPhysics::default(), avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)));
                                }
                                let data = crate::common::game::bricks::data::BrickData {
                                    transform: new_transform,
                                    name: format!("{} - Copy", name),
                                    is_brick: copied_buffer.is_brick,
                                    shape: copied_buffer.shape,
                                    mesh: copied_buffer.mesh.clone(),
                                    mesh_asset: copied_buffer.mesh_asset,
                                    standard_material: copied_buffer.material.clone(),
                                    studs_material: copied_buffer.studs_material.clone(),
                                    parent: None,
                                    physics: copied_buffer.physics.clone(),
                                    studs: copied_buffer.show_studs,
                                    color: copied_buffer.color,
                                };
                                history.push_command(crate::studio::tools::UndoCommand::Spawn { entity: new_entity, data });
                                selection.entity = Some(new_entity);
                                selection.entities = vec![new_entity];
                                selection.workspace_selected = false;
                                selection.players_selected = false;
                                selection.lighting_selected = false;
                            }
                            ui.close_menu();
                        }
                        if ui.add_enabled(has_selection, egui::Button::new("Duplicate  Ctrl+D")).clicked() {
                            let to_duplicate: Vec<Entity> = selection.entities.clone();
                            let mut new_entities = Vec::new();
                            for entity in to_duplicate {
                                if let Some(mut data) = crate::common::game::bricks::data::capture_brick_data(entity, entities_query, studs_query, brick_colors, mesh_assets) {
                                    data.transform.translation += Vec3::new(2.0 * 0.28, 0.0, 2.0 * 0.28);
                                    let new_entity = crate::common::game::bricks::data::spawn_from_data(commands, &data);
                                    history.push_command(crate::studio::tools::UndoCommand::Spawn { entity: new_entity, data });
                                    new_entities.push(new_entity);
                                }
                            }
                            if !new_entities.is_empty() {
                                selection.entities = new_entities.clone();
                                selection.entity = Some(new_entities[0]);
                                selection.workspace_selected = false;
                                selection.players_selected = false;
                                selection.lighting_selected = false;
                            }
                            ui.close_menu();
                        }
                        if ui.add_enabled(has_selection, egui::Button::new("Delete  Del")).clicked() {
                            for entity in selection.entities.drain(..).collect::<Vec<_>>() {
                                if let Some(data) = crate::common::game::bricks::data::capture_brick_data(entity, entities_query, studs_query, brick_colors, mesh_assets) {
                                    history.push_command(crate::studio::tools::UndoCommand::Delete { entity, data });
                                }
                                commands.entity(entity).try_despawn();
                            }
                            selection.entity = None;
                            selection.workspace_selected = false;
                            selection.players_selected = false;
                            selection.lighting_selected = false;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Select All  Ctrl+A").clicked() {
                            let mut all = Vec::new();
                            for (entity, _, _, _, _, _, _, _, _, _, _, _) in entities_query.iter() {
                                all.push(entity);
                            }
                            for (entity, _, _, _, _, _, _, _, _, _, _) in explorer_query.iter() {
                                if !all.contains(&entity) { all.push(entity); }
                            }
                            selection.entities = all.clone();
                            selection.entity = all.first().copied();
                            selection.workspace_selected = false;
                            selection.players_selected = false;
                            selection.lighting_selected = false;
                            ui.close_menu();
                        }
                        if ui.add_enabled(has_selection, egui::Button::new("Clear Selection")).clicked() {
                            selection.entities.clear();
                            selection.entity = None;
                            selection.workspace_selected = false;
                            selection.players_selected = false;
                            selection.lighting_selected = false;
                            ui.close_menu();
                        }
                    });
                    is_edit_hovered_flag = edit_button_res.response.hovered();
                });
                ui.data_mut(|d| d.insert_temp(edit_id, is_edit_hovered_flag));

                let insert_id = ui.make_persistent_id("insert_menu_btn");
                let is_insert_hovered = ui.data(|d| d.get_temp::<bool>(insert_id)).unwrap_or(false);
                let insert_text_color = if is_insert_hovered {
                    egui::Color32::from_rgb(80, 160, 240)
                } else {
                    egui::Color32::from_rgb(60, 60, 60)
                };
                let mut is_insert_hovered_flag = false;
                ui.scope(|ui| {
                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                    ui.style_mut().visuals.widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                    let insert_button_res = egui::menu::menu_button(ui, egui::RichText::new("Insert").color(insert_text_color).size(13.0), |ui| {
                        ui.set_min_width(160.0);
                        if ui.add(egui::Button::image_and_text((brick_tex, egui::vec2(16.0, 16.0)), "Block")).clicked() {
                            let shape = crate::common::game::bricks::components::BrickShape::Block;
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
                                let half_ext = Vec3::new(2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28);
                                spawn_pos.x = ((spawn_pos.x - half_ext.x) / snap_interval).round() * snap_interval + half_ext.x;
                                spawn_pos.z = ((spawn_pos.z - half_ext.z) / snap_interval).round() * snap_interval + half_ext.x;
                                spawn_pos.y = ((spawn_pos.y - half_ext.y) / snap_interval).round() * snap_interval + half_ext.y;
                                if spawn_pos.y < half_ext.y { spawn_pos.y = half_ext.y; }
                            }
                            let new_entity = spawn_brick(commands, meshes, studs_materials, studs_assets, count, spawn_pos, shape);
                            commands.entity(new_entity).insert(crate::common::game::bricks::components::BrickStuds { enabled: workspace_studs.enabled });
                            let default_name = format!("Part{}", count.count - 1);
                            let default_mesh = Some(Mesh3d(meshes.add(crate::common::game::bricks::block_brick_mesh(Vec3::ONE))));
                            let data = crate::common::game::bricks::data::BrickData {
                                transform: Transform::from_translation(spawn_pos),
                                name: default_name,
                                is_brick: true,
                                shape,
                                mesh: default_mesh,
                                mesh_asset: None,
                                standard_material: None,
                                studs_material: Some(MeshMaterial3d(studs_materials.add(ExtendedMaterial {
                                    base: StandardMaterial { base_color: Color::srgb(0.84, 0.24, 0.16), perceptual_roughness: BRICK_PERCEPTUAL_ROUGHNESS, reflectance: BRICK_REFLECTANCE, metallic: BRICK_METALLIC, ..default() },
                                    extension: crate::common::game::bricks::studs::StudsExtension {
                                        stud_texture: studs_assets.stud.clone(),
                                        stud_ambient_texture: studs_assets.stud_ambient.clone(),
                                        stud_height_texture: studs_assets.stud_height.clone(),
                                        inlet_ambient_texture: studs_assets.inlet_ambient.clone(),
                                        inlet_height_texture: studs_assets.inlet_height.clone(),
                                    },
                                }))),
                                parent: None,
                                physics: Some(crate::common::game::bricks::components::BrickPhysics::default()),
                                studs: workspace_studs.enabled,
                                color: None,
                            };
                            history.push_command(crate::studio::tools::UndoCommand::Spawn { entity: new_entity, data });
                            ui.close_menu();
                        }
                        if ui.add(egui::Button::image_and_text((brick_tex, egui::vec2(16.0, 16.0)), "Sphere")).clicked() {
                            let shape = crate::common::game::bricks::components::BrickShape::Sphere;
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
                                let half_ext = Vec3::new(1.0 * 0.28, 1.0 * 0.28, 1.0 * 0.28);
                                spawn_pos.x = ((spawn_pos.x - half_ext.x) / snap_interval).round() * snap_interval + half_ext.x;
                                spawn_pos.z = ((spawn_pos.z - half_ext.z) / snap_interval).round() * snap_interval + half_ext.x;
                                spawn_pos.y = ((spawn_pos.y - half_ext.y) / snap_interval).round() * snap_interval + half_ext.y;
                                if spawn_pos.y < half_ext.y { spawn_pos.y = half_ext.y; }
                            }
                            let new_entity = spawn_brick(commands, meshes, studs_materials, studs_assets, count, spawn_pos, shape);
                            commands.entity(new_entity).insert(crate::common::game::bricks::components::BrickStuds { enabled: workspace_studs.enabled });
                            let default_name = format!("Sphere{}", count.count - 1);
                            let default_mesh = Some(Mesh3d(meshes.add(Sphere::new(1.0 * 0.28))));
                            let data = crate::common::game::bricks::data::BrickData {
                                transform: Transform::from_translation(spawn_pos),
                                name: default_name,
                                is_brick: true,
                                shape,
                                mesh: default_mesh,
                                mesh_asset: None,
                                standard_material: None,
                                studs_material: Some(MeshMaterial3d(studs_materials.add(ExtendedMaterial {
                                    base: StandardMaterial { base_color: Color::srgb(0.84, 0.24, 0.16), perceptual_roughness: BRICK_PERCEPTUAL_ROUGHNESS, reflectance: BRICK_REFLECTANCE, metallic: BRICK_METALLIC, ..default() },
                                    extension: crate::common::game::bricks::studs::StudsExtension {
                                        stud_texture: studs_assets.stud.clone(),
                                        stud_ambient_texture: studs_assets.stud_ambient.clone(),
                                        stud_height_texture: studs_assets.stud_height.clone(),
                                        inlet_ambient_texture: studs_assets.inlet_ambient.clone(),
                                        inlet_height_texture: studs_assets.inlet_height.clone(),
                                    },
                                }))),
                                parent: None,
                                physics: Some(crate::common::game::bricks::components::BrickPhysics::default()),
                                studs: workspace_studs.enabled,
                                color: None,
                            };
                            history.push_command(crate::studio::tools::UndoCommand::Spawn { entity: new_entity, data });
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.add(egui::Button::image_and_text((script_tex, egui::vec2(16.0, 16.0)), "Script")).clicked() {
                            let new_entity = commands.spawn((Name::new("Script"), crate::scripting::ecs::ServerScript { code: "print(\"Hello World from Server!\")\n".to_string(), enabled: true, started: false, running_code: String::new() })).id();
                            if let Some(parent) = selection.entity { commands.entity(parent).add_child(new_entity); }
                            ui.close_menu();
                        }
                        if ui.add(egui::Button::image_and_text((localscript_tex, egui::vec2(16.0, 16.0)), "LocalScript")).clicked() {
                            let new_entity = commands.spawn((Name::new("LocalScript"), crate::scripting::ecs::LocalScript { code: "print(\"Hello World from Local!\")\n".to_string(), enabled: true, started: false, running_code: String::new() }, lightyear::prelude::Replicate::default())).id();
                            if let Some(parent) = selection.entity { commands.entity(parent).add_child(new_entity); }
                            ui.close_menu();
                        }
                        if ui.add(egui::Button::image_and_text((modulescript_tex, egui::vec2(16.0, 16.0)), "ModuleScript")).clicked() {
                            let new_entity = commands.spawn((Name::new("ModuleScript"), crate::scripting::ecs::ModuleScript { code: "local module = {}\nreturn module\n".to_string() }, lightyear::prelude::Replicate::default())).id();
                            if let Some(parent) = selection.entity { commands.entity(parent).add_child(new_entity); }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.add(egui::Button::image_and_text((image_tex, egui::vec2(16.0, 16.0)), "Image")).clicked() {
                            let mut spawn_pos = Vec3::new(0.0, 0.001, 0.0);
                            let mut spawn_rotation = Quat::IDENTITY;
                            if let Some(cam_t) = camera_transform {
                                let camera_pos = cam_t.translation;
                                let camera_forward = cam_t.forward();
                                let mut found_hit = false;
                                if camera_forward.y.abs() > 0.001 {
                                    let resting_y = 0.001;
                                    let t = (resting_y - camera_pos.y) / camera_forward.y;
                                    if t > 0.0 && t < 100.0 {
                                        let hit_pos = camera_pos + camera_forward * t;
                                        if hit_pos.x.abs() <= 25.0 && hit_pos.z.abs() <= 25.0 { spawn_pos = hit_pos; found_hit = true; }
                                    }
                                }
                                if !found_hit { spawn_pos = camera_pos + camera_forward * (10.0 * 0.28); }
                                let yaw = f32::atan2(camera_forward.x, camera_forward.z);
                                spawn_rotation = Quat::from_rotation_y(yaw);
                            }
                            let cmd = commands.spawn((Transform::from_translation(spawn_pos).with_rotation(spawn_rotation).with_scale(Vec3::new(2.0 * 0.28, 2.0 * 0.28, 0.001)), Name::new("Image"), crate::common::game::assets::components::Image { asset_id: 0, face: None }, Pickable::default(), Visibility::Visible));
                            let new_entity = cmd.id();
                            if let Some(parent) = selection.entity { commands.entity(parent).add_child(new_entity); }
                            ui.close_menu();
                        }
                        if ui.add(egui::Button::image_and_text((mesh_tex, egui::vec2(16.0, 16.0)), "Mesh")).clicked() {
                            let mut spawn_pos = Vec3::new(0.0, 0.001, 0.0);
                            let mut spawn_rotation = Quat::IDENTITY;
                            if let Some(cam_t) = camera_transform {
                                let camera_pos = cam_t.translation;
                                let camera_forward = cam_t.forward();
                                let mut found_hit = false;
                                if camera_forward.y.abs() > 0.001 {
                                    let resting_y = 0.001;
                                    let t = (resting_y - camera_pos.y) / camera_forward.y;
                                    if t > 0.0 && t < 100.0 {
                                        let hit_pos = camera_pos + camera_forward * t;
                                        if hit_pos.x.abs() <= 25.0 && hit_pos.z.abs() <= 25.0 { spawn_pos = hit_pos; found_hit = true; }
                                    }
                                }
                                if !found_hit { spawn_pos = camera_pos + camera_forward * (10.0 * 0.28); }
                                let yaw = f32::atan2(camera_forward.x, camera_forward.z);
                                spawn_rotation = Quat::from_rotation_y(yaw);
                            }
                            let cmd = commands.spawn((Transform::from_translation(spawn_pos).with_rotation(spawn_rotation).with_scale(Vec3::ONE), Name::new("Mesh"), crate::common::game::assets::components::Mesh { asset_id: 0, normalize: false }, crate::common::game::bricks::components::BrickPhysics::default(), avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF), Pickable::default(), Visibility::Visible));
                            let new_entity = cmd.id();
                            if let Some(parent) = selection.entity { commands.entity(parent).add_child(new_entity); }
                            ui.close_menu();
                        }
                    });
                    is_insert_hovered_flag = insert_button_res.response.hovered();
                });
                ui.data_mut(|d| d.insert_temp(insert_id, is_insert_hovered_flag));

                let view_id = ui.make_persistent_id("view_menu_btn");
                let is_view_hovered = ui.data(|d| d.get_temp::<bool>(view_id)).unwrap_or(false);
                let view_text_color = if is_view_hovered {
                    egui::Color32::from_rgb(80, 160, 240)
                } else {
                    egui::Color32::from_rgb(60, 60, 60)
                };
                let mut is_view_hovered_flag = false;
                ui.scope(|ui| {
                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                    ui.style_mut().visuals.widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                    let view_button_res = egui::menu::menu_button(ui, egui::RichText::new("View").color(view_text_color).size(13.0), |ui| {
                        let mut show_studs = workspace_studs.enabled;
                        if ui.checkbox(&mut show_studs, "Show Studs").changed() {
                            workspace_studs.enabled = show_studs;
                        }
                        ui.separator();
                        if ui.button("Focus on Selection  (F)").clicked() {
                            if let Some(primary) = selection.entity {
                                if let Ok((_, _, _, _, _, _, _, global, _, _, _, _)) = entities_query.get(primary) {
                                    let target = global.translation();
                                    if let Some(mut cam_t) = camera_transform_query.iter_mut().next() {
                                        let dir = cam_t.forward().as_vec3();
                                        cam_t.translation = target - dir * 10.0;
                                        cam_t.look_at(target, Vec3::Y);
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Reset Camera").clicked() {
                            if let Some(mut cam_t) = camera_transform_query.iter_mut().next() {
                                *cam_t = Transform::from_xyz(0.0, 12.0, 14.0).looking_at(Vec3::ZERO, Vec3::Y);
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Select Workspace").clicked() {
                            selection.entity = None;
                            selection.entities.clear();
                            selection.workspace_selected = true;
                            selection.players_selected = false;
                            selection.lighting_selected = false;
                            ui.close_menu();
                        }
                        if ui.button("Select Players").clicked() {
                            selection.entity = None;
                            selection.entities.clear();
                            selection.workspace_selected = false;
                            selection.players_selected = true;
                            selection.lighting_selected = false;
                            ui.close_menu();
                        }
                        if ui.button("Select Lighting").clicked() {
                            selection.entity = None;
                            selection.entities.clear();
                            selection.workspace_selected = false;
                            selection.players_selected = false;
                            selection.lighting_selected = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Toggle Output").clicked() {
                            crate::scripting::output::push(crate::scripting::output::OutputLevel::Info, "View", None, "Output panel toggled".to_string(), None);
                            ui.close_menu();
                        }
                    });
                    is_view_hovered_flag = view_button_res.response.hovered();
                });
                ui.data_mut(|d| d.insert_temp(view_id, is_view_hovered_flag));

                let test_id = ui.make_persistent_id("test_menu_btn");
                let is_test_hovered = ui.data(|d| d.get_temp::<bool>(test_id)).unwrap_or(false);
                let test_text_color = if is_test_hovered {
                    egui::Color32::from_rgb(80, 160, 240)
                } else {
                    egui::Color32::from_rgb(60, 60, 60)
                };
                let mut is_test_hovered_flag = false;
                ui.scope(|ui| {
                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                    ui.style_mut().visuals.widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                    let test_button_res = egui::menu::menu_button(ui, egui::RichText::new("Test").color(test_text_color).size(13.0), |ui| {
                        let is_playing = physics_state == crate::common::game::physics::PhysicsSimulationState::Running;
                        if ui.add_enabled(!is_playing, egui::Button::new("Play  ▶")).clicked() {
                            physics_action_writer.write(crate::common::game::physics::PhysicsSimulationAction::Play);
                            ui.close_menu();
                        }
                        if ui.add_enabled(is_playing, egui::Button::new("Stop  ■")).clicked() {
                            physics_action_writer.write(crate::common::game::physics::PhysicsSimulationAction::Stop);
                            ui.close_menu();
                        }
                        ui.separator();
                        let playtesting_active = playtest_state.active;
                        if ui.add_enabled(!playtesting_active, egui::Button::new("Play In Studio")).clicked() {
                            playtest_state.active = true;
                            selection.entity = None;
                            selection.entities.clear();
                            selection.workspace_selected = false;
                            selection.players_selected = false;
                            selection.lighting_selected = false;
                            crate::scripting::output::start_run("Playtest");
                            if let Some(g) = gravity.as_ref() { playtest_backup.gravity = Some(g.0); } else { playtest_backup.gravity = None; }
                            if let Some(ps) = players_service.as_ref() { playtest_backup.players_service = Some((**ps).clone()); } else { playtest_backup.players_service = None; }
                            let mut backup_bricks = Vec::new();
                            for (entity, _, _name, _, _, brick_opt, _, _, _, _, _, _) in entities_query.iter() {
                                if brick_opt.is_some() {
                                    if let Some(data) = crate::common::game::bricks::data::capture_brick_data(entity, entities_query, studs_query, brick_colors, mesh_assets) {
                                        backup_bricks.push(data);
                                    }
                                    commands.entity(entity).despawn();
                                }
                            }
                            playtest_backup.bricks = backup_bricks;
                            let mut backup_scripts = Vec::new();
                            for (_entity, _name, child_of_opt, _, _, s_opt, l_opt, m_opt, _, _, _) in explorer_query.iter() {
                                let mut script_type_opt = None;
                                let mut code = String::new();
                                let mut enabled = true;
                                if let Some(s) = s_opt { script_type_opt = Some(0); code = s.code.clone(); enabled = s.enabled; } else if let Some(l) = l_opt { script_type_opt = Some(1); code = l.code.clone(); enabled = l.enabled; } else if let Some(m) = m_opt { script_type_opt = Some(2); code = m.code.clone(); }
                                if let Some(script_type) = script_type_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt { if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) { parent_name = Some(p_name.to_string()); } }
                                    backup_scripts.push(crate::common::core::vrtx::VrtxScript { name: _name.to_string(), script_type, code, parent_name, enabled });
                                    commands.entity(_entity).despawn();
                                }
                            }
                            playtest_backup.scripts = backup_scripts;
                            let mut backup_images = Vec::new();
                            for (_entity, _name, child_of_opt, image_opt) in images_query.iter() {
                                if let Some(image) = image_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt { if let Ok((_, p_name, _, _)) = images_query.get(child_of.parent()) { parent_name = Some(p_name.to_string()); } }
                                    let transform = entities_query.get(_entity).map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t).unwrap_or_default();
                                    backup_images.push(crate::common::core::vrtx::VrtxImage { name: _name.to_string(), asset_id: image.asset_id, face: image.face.as_ref().map(|f| f.as_str().to_string()), parent_name, transform });
                                    commands.entity(_entity).despawn();
                                }
                            }
                            playtest_backup.images = backup_images;
                            let mut backup_meshes = Vec::new();
                            for (_entity, _name, child_of_opt, _, _, _, _, _, _, _, mesh_opt) in explorer_query.iter() {
                                if let Some(mesh) = mesh_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt { if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) { parent_name = Some(p_name.to_string()); } }
                                    let transform = entities_query.get(_entity).map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t).unwrap_or_default();
                                    let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = entities_query.get(_entity).map(|(_, _, _, _, _, _, _, _, _, _, _, phys_opt)| { if let Some(phys) = phys_opt { (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass) } else { (true, 0.3, true, 0.3, 1.0, 1.0) } }).unwrap_or((true, 0.3, true, 0.3, 1.0, 1.0));
                                    backup_meshes.push(crate::common::core::vrtx::VrtxMesh { name: _name.to_string(), asset_id: mesh.asset_id, normalize: mesh.normalize, parent_name, transform, physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass });
                                    commands.entity(_entity).despawn();
                                }
                            }
                            playtest_backup.meshes = backup_meshes;
                            let mut backup_textures = Vec::new();
                            for (_entity, _name, child_of_opt, _, _, _, _, _, _, texture_opt, _) in explorer_query.iter() {
                                if let Some(texture) = texture_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt { if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) { parent_name = Some(p_name.to_string()); } }
                                    backup_textures.push(crate::common::core::vrtx::VrtxTexture { name: _name.to_string(), id_string: texture.as_content_id(), parent_name });
                                    commands.entity(_entity).despawn();
                                }
                            }
                            playtest_backup.textures = backup_textures;
                            let temp_map_path = "temp_play.vrtx".to_string();
                            let players = if let Some(ps) = players_service.as_ref() {
                                crate::common::core::vrtx::VrtxPlayers {
                                    speed: ps.speed,
                                    jump_power: ps.jump_power,
                                    gravity: ps.gravity,
                                    speed_response: ps.speed_response,
                                    friction: ps.friction,
                                    bounciness: ps.bounciness,
                                }
                            } else {
                                crate::common::core::vrtx::VrtxPlayers::default()
                            };
                            let state = crate::common::core::vrtx::VrtxFileState {
                                version: crate::common::core::vrtx::FORMAT_VERSION,
                                gravity: Vec3::new(0.0, -186.9 * 0.28, 0.0),
                                settings: crate::common::core::vrtx::VrtxSettings { ssao: graphics_settings.ssao, contact_shadows: graphics_settings.contact_shadows, bloom: graphics_settings.bloom },
                                lighting: crate::common::core::vrtx::VrtxLighting::from(&**lighting_config),
                                players,
                                camera_transform: Transform::IDENTITY,
                                bricks: playtest_backup.bricks.iter().map(|b| {
                                    let mut current_color = Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0));
                                    if let Some(ref studs_material_handle) = b.studs_material { if let Some(mat) = studs_materials.get(&studs_material_handle.0) { current_color = mat.base.base_color; } } else if let Some(ref mat_handle) = b.standard_material { if let Some(mat) = materials.get(&mat_handle.0) { current_color = mat.base.base_color; } }
                                    let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = if let Some(phys) = b.physics { (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass) } else { (true, 0.3, true, 0.3, 1.0, 1.0) };
                                    crate::common::core::vrtx::VrtxBrick { name: b.name.clone(), transform: b.transform, shape: b.shape, color: current_color, physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass, show_studs: b.studs }
                                }).collect(),
                                scripts: playtest_backup.scripts.clone(),
                                images: playtest_backup.images.clone(),
                                meshes: playtest_backup.meshes.clone(),
                                textures: playtest_backup.textures.clone(),
                            };
                            if state.save_to_file(&temp_map_path).is_ok() {
                                crate::app::server::bootstrap::begin_playtest_server_epoch();
                                let playtest_port = crate::app::server::bootstrap::pick_free_playtest_port();
                                let playtest_key: [u8; 32] = rand::random();
                                let playtest_protocol_id: u64 = rand::random();
                                let server_app = crate::app::server::bootstrap::RaveServerApp::new(crate::app::server::config::ServerAppConfig { port: playtest_port, map_path: temp_map_path, bind_addr: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), netcode_key: playtest_key, protocol_id: playtest_protocol_id, allow_unauthenticated: true });
                                std::thread::spawn(move || { server_app.run(); });
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                let server_addr = std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), playtest_port);
                                let client_id = rand::random::<u64>();
                                commands.insert_resource(crate::client::LocalClientId(client_id));
                                commands.insert_resource(crate::client::ClientUkey("studio_play_local_key".to_string()));
                                let auth = lightyear::prelude::Authentication::Manual { server_addr, client_id, private_key: playtest_key, protocol_id: playtest_protocol_id };
                                let netcode_config = lightyear::prelude::client::NetcodeConfig { client_timeout_secs: 15, ..default() };
                                let client_entity = commands.spawn((lightyear::prelude::client::Client::default(), lightyear::prelude::UdpIo::default(), lightyear::prelude::client::NetcodeClient::new(auth, netcode_config).unwrap(), lightyear::prelude::LocalAddr(std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)), 0)), lightyear::prelude::PeerAddr(server_addr), crate::studio::ui::resources::InEditorPlaytestClient)).id();
                                commands.trigger(lightyear::prelude::client::Connect { entity: client_entity });
                            }
                            ui.close_menu();
                        }
                        if ui.add_enabled(playtesting_active, egui::Button::new("Stop Playtest")).clicked() {
                            playtest_state.active = false;
                            selection.entity = None;
                            selection.entities.clear();
                            selection.workspace_selected = false;
                            selection.players_selected = false;
                            selection.lighting_selected = false;
                            crate::scripting::output::end_run();
                            crate::app::server::bootstrap::request_playtest_server_shutdown();
                            if let Some(client_entity) = playtest_client_query.iter().next() {
                                commands.trigger(lightyear::prelude::client::Disconnect { entity: client_entity });
                                commands.entity(client_entity).try_despawn();
                            }
                            for (entity, _, _name, _, _, brick_opt, _, _, _, _, _, _) in entities_query.iter() {
                                let name_str = _name.as_str();
                                if brick_opt.is_some() || name_str == "Player" || name_str.starts_with("Player_") { commands.entity(entity).try_despawn(); }
                            }
                            for entity in replicated_images { commands.entity(entity).try_despawn(); }
                            for (entity, _, _, _, _, _, _, _, _, _, mesh_opt) in explorer_query.iter() { if mesh_opt.is_some() { commands.entity(entity).try_despawn(); } }
                            let mut named_entities = std::collections::HashMap::new();
                            for brick_data in playtest_backup.bricks.drain(..) { let name = brick_data.name.clone(); let new_entity = crate::common::game::bricks::data::spawn_from_data(commands, &brick_data); named_entities.insert(name, new_entity); }
                            for script_data in playtest_backup.scripts.drain(..) {
                                let mut cmd = commands.spawn(Name::new(script_data.name));
                                match script_data.script_type {
                                    0 => { cmd.insert(crate::scripting::ecs::ServerScript { code: script_data.code, enabled: script_data.enabled, started: false, running_code: String::new() }); },
                                    1 => { cmd.insert((crate::scripting::ecs::LocalScript { code: script_data.code, enabled: script_data.enabled, started: false, running_code: String::new() }, lightyear::prelude::Replicate::default())); },
                                    _ => { cmd.insert((crate::scripting::ecs::ModuleScript { code: script_data.code }, lightyear::prelude::Replicate::default())); },
                                }
                                let new_script_entity = cmd.id();
                                if let Some(ref p_name) = script_data.parent_name { if let Some(&parent_entity) = named_entities.get(p_name) { commands.entity(parent_entity).add_child(new_script_entity); } }
                            }
                            for image_data in playtest_backup.images.drain(..) {
                                let face = image_data.face.as_deref().and_then(crate::common::game::assets::components::ImageFace::from_str);
                                let cmd = commands.spawn((image_data.transform, Name::new(image_data.name), crate::common::game::assets::components::Image { asset_id: image_data.asset_id, face }, Pickable::default(), Visibility::Visible));
                                let new_image_entity = cmd.id();
                                if let Some(ref p_name) = image_data.parent_name { if let Some(&parent_entity) = named_entities.get(p_name) { commands.entity(parent_entity).add_child(new_image_entity); } }
                            }
                            for mesh_data in playtest_backup.meshes.drain(..) {
                                let cmd = commands.spawn((mesh_data.transform, Name::new(mesh_data.name), crate::common::game::assets::components::Mesh { asset_id: mesh_data.asset_id, normalize: mesh_data.normalize }, crate::common::game::bricks::components::BrickPhysics { enabled: mesh_data.physics_enabled, bounciness: mesh_data.bounciness, player_can_collide: mesh_data.player_can_collide, friction: mesh_data.friction, gravity_scale: mesh_data.gravity_scale, mass: mesh_data.mass }, avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF), Pickable::default(), Visibility::Visible));
                                let new_mesh_entity = cmd.id();
                                if let Some(ref p_name) = mesh_data.parent_name { if let Some(&parent_entity) = named_entities.get(p_name) { commands.entity(parent_entity).add_child(new_mesh_entity); } }
                            }
                            for (entity, _, _, _, _, _, _, _, _, _, _, _) in entities_query.iter() {
                                commands.entity(entity).remove::<(avian3d::prelude::RigidBody, avian3d::prelude::Collider, crate::common::game::physics::TransformBackup, crate::common::game::physics::PhysicsAttached, avian3d::prelude::Friction, avian3d::prelude::Restitution, avian3d::prelude::GravityScale, avian3d::prelude::Mass, avian3d::prelude::LinearDamping, avian3d::prelude::AngularDamping, avian3d::prelude::SleepThreshold)>();
                            }
                            if let Some(gravity_val) = playtest_backup.gravity.take() { if let Some(g) = gravity { g.0 = gravity_val; } }
                            if let Some(ps_val) = playtest_backup.players_service.take() { if let Some(ps) = players_service { **ps = ps_val.clone(); } if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write() { *shared = ps_val; } }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Clear Output").clicked() {
                            crate::scripting::output::clear_entries();
                            ui.close_menu();
                        }
                    });
                    is_test_hovered_flag = test_button_res.response.hovered();
                });
                ui.data_mut(|d| d.insert_temp(test_id, is_test_hovered_flag));

                let settings_id = ui.make_persistent_id("settings_menu_btn");
                let is_hovered_settings = ui.data(|d| d.get_temp::<bool>(settings_id)).unwrap_or(false);
                let text_color = if is_hovered_settings {
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

    let (bottom_sep, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(bottom_sep, 0.0, egui::Color32::from_rgb(212, 212, 212));

    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.add_enabled_ui(!onboarding_active, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

                    let is_move = *current_tool.get() == ToolState::Move;
                    if ribbonbutton(ui, Some(move_tex), "Move", is_move)
                        .on_hover_text("Move tool (1)")
                        .clicked() {
                        if is_move {
                            next_tool.set(ToolState::None);
                        } else {
                            next_tool.set(ToolState::Move);
                        }
                    }

                    let is_rotate = *current_tool.get() == ToolState::Rotate;
                    if ribbonbutton(ui, Some(rotate_tex), "Rotate", is_rotate)
                        .on_hover_text("Rotate tool (2)")
                        .clicked() {
                        if is_rotate {
                            next_tool.set(ToolState::None);
                        } else {
                            next_tool.set(ToolState::Rotate);
                        }
                    }

                    let is_scale = *current_tool.get() == ToolState::Size;
                    if ribbonbutton(ui, Some(scale_tex), "Scale", is_scale)
                        .on_hover_text("Scale tool (3)")
                        .clicked() {
                        if is_scale {
                            next_tool.set(ToolState::None);
                        } else {
                            next_tool.set(ToolState::Size);
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
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut snap_config.rotation_angle)
                                    .speed(1.0)
                                    .range(0.01..=360.0)
                                    .suffix("°")
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

                            ui.set_min_width(160.0);
                            ui.horizontal(|ui| {
                                ui.label("🔍"); 
                                let text_edit_res = ui.text_edit_singleline(&mut search_query);
                                if text_edit_res.changed() {
                                    ui.data_mut(|d| d.insert_temp(popup_id, search_query.clone()));
                                }
                            });
                            ui.separator();

                            let parts_items = [
                                ("Block", crate::common::game::bricks::components::BrickShape::Block),
                                ("Sphere", crate::common::game::bricks::components::BrickShape::Sphere),
                                ("Cylinder", crate::common::game::bricks::components::BrickShape::Cylinder),
                                ("Wedge", crate::common::game::bricks::components::BrickShape::Wedge),
                                ("CornerWedge", crate::common::game::bricks::components::BrickShape::CornerWedge),
                            ];
                            let parts_visible = parts_items.iter().any(|(item, _)| item.to_lowercase().contains(&search_query.to_lowercase()));
                            if parts_visible {
                                ui.label(egui::RichText::new("Parts").color(egui::Color32::from_rgb(120, 120, 120)).size(12.0).strong());
                                for (item, shape) in parts_items {
                                    if item.to_lowercase().contains(&search_query.to_lowercase()) {
                                        if ui.add(egui::Button::image_and_text(
                                            (brick_tex, egui::vec2(16.0, 16.0)),
                                            item,
                                        )).clicked() {
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
                                            let half_ext = shape.base_half_extents_world();
                                            spawn_pos.x = ((spawn_pos.x - half_ext.x) / snap_interval).round() * snap_interval + half_ext.x;
                                            spawn_pos.z = ((spawn_pos.z - half_ext.z) / snap_interval).round() * snap_interval + half_ext.x;
                                            spawn_pos.y = ((spawn_pos.y - half_ext.y) / snap_interval).round() * snap_interval + half_ext.y;
                                            if spawn_pos.y < half_ext.y {
                                                spawn_pos.y = half_ext.y;
                                            }
                                        }

                                        let new_entity = spawn_brick(commands, meshes, studs_materials, studs_assets, count, spawn_pos, shape);

                                        commands.entity(new_entity).insert(crate::common::game::bricks::components::BrickStuds { enabled: workspace_studs.enabled });

                                        let default_name = format!(
                                            "{}{}",
                                            shape.default_name_prefix(),
                                            count.count - 1
                                        );

                                        let default_mesh = match shape {
                                            crate::common::game::bricks::components::BrickShape::Block => Some(Mesh3d(meshes.add(crate::common::game::bricks::block_brick_mesh(Vec3::ONE)))),
                                            crate::common::game::bricks::components::BrickShape::Sphere => Some(Mesh3d(meshes.add(Sphere::new(1.0 * 0.28)))),
                                            crate::common::game::bricks::components::BrickShape::Cylinder => Some(Mesh3d(meshes.add(crate::common::game::bricks::cylinder_brick_mesh()))),
                                            crate::common::game::bricks::components::BrickShape::Wedge => Some(Mesh3d(meshes.add(crate::common::game::bricks::wedge_brick_mesh()))),
                                            crate::common::game::bricks::components::BrickShape::CornerWedge => Some(Mesh3d(meshes.add(crate::common::game::bricks::corner_wedge_brick_mesh()))),
                                        };

                                        let data = crate::common::game::bricks::data::BrickData {
                                            transform: Transform::from_translation(spawn_pos),
                                            name: default_name,
                                            is_brick: true,
                                            shape,
                                            mesh: default_mesh,
                                            mesh_asset: None,
                                            standard_material: None,
                                            studs_material: Some(MeshMaterial3d(studs_materials.add(ExtendedMaterial {
                                                base: StandardMaterial {
                                                    base_color: Color::srgb(0.84, 0.24, 0.16),
                                                    perceptual_roughness: 0.95,
                                                    reflectance: 0.1,
                                                    ..default()
                                                },
                                                extension: crate::common::game::bricks::studs::StudsExtension {
                                                    stud_texture: studs_assets.stud.clone(),
                                                    stud_ambient_texture: studs_assets.stud_ambient.clone(),
                                                    stud_height_texture: studs_assets.stud_height.clone(),
                                                    inlet_ambient_texture: studs_assets.inlet_ambient.clone(),
                                                    inlet_height_texture: studs_assets.inlet_height.clone(),
                                                },
                                            }))),
                                            parent: None,
                                            physics: Some(crate::common::game::bricks::components::BrickPhysics::default()),
                                            studs: workspace_studs.enabled,
                                            color: None,
                                        };

                                        history.push_command(crate::studio::tools::UndoCommand::Spawn {
                                            entity: new_entity,
                                            data,
                                        });

                                        ui.memory_mut(|mem| mem.close_popup(popup_id));
                                    }
                                }
                            }
                            }

                            let script_items = [
                                ("Script", 0),
                                ("LocalScript", 1),
                                ("ModuleScript", 2),
                            ];
                            let scripts_visible = script_items.iter().any(|(item, _)| item.to_lowercase().contains(&search_query.to_lowercase()));
                            if scripts_visible {
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Scripts").color(egui::Color32::from_rgb(120, 120, 120)).size(12.0).strong());
                                for (item, script_type) in script_items {
                                    if item.to_lowercase().contains(&search_query.to_lowercase()) {
                                        let script_icon = match script_type {
                                            0 => script_tex,
                                            1 => localscript_tex,
                                            _ => modulescript_tex,
                                        };
                                        if ui.add(egui::Button::image_and_text(
                                            (script_icon, egui::vec2(16.0, 16.0)),
                                            item,
                                        )).clicked() {
                                        let new_entity = match script_type {
                                            0 => commands.spawn((
                                                Name::new(item),
                                                crate::scripting::ecs::ServerScript {
                                                    code: "print(\"Hello World from Server!\")\n".to_string(),
                                                    enabled: true,
                                                    started: false,
                                                    running_code: String::new(),
                                                },
                                            )).id(),
                                            1 => commands.spawn((
                                                Name::new(item),
                                                crate::scripting::ecs::LocalScript {
                                                    code: "print(\"Hello World from Local!\")\n".to_string(),
                                                    enabled: true,
                                                    started: false,
                                                    running_code: String::new(),
                                                },
                                                lightyear::prelude::Replicate::default(),
                                            )).id(),
                                            _ => commands.spawn((
                                                Name::new(item),
                                                crate::scripting::ecs::ModuleScript {
                                                    code: "local module = {}\nreturn module\n".to_string(),
                                                },
                                                lightyear::prelude::Replicate::default(),
                                            )).id(),
                                        };

                                        if let Some(parent) = selection.entity {
                                            commands.entity(parent).add_child(new_entity);
                                        }

                                        ui.memory_mut(|mem| mem.close_popup(popup_id));
                                    }
                                }
                            }
                            }

                            if "image".contains(&search_query.to_lowercase()) {
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Decals").color(egui::Color32::from_rgb(120, 120, 120)).size(12.0).strong());
                                if ui.add(egui::Button::image_and_text(
                                    (image_tex, egui::vec2(16.0, 16.0)),
                                    "Image",
                                )).clicked() {
                                    let mut spawn_pos = Vec3::new(0.0, 0.001, 0.0);
                                    let mut spawn_rotation = Quat::IDENTITY;
                                    if let Some(cam_t) = camera_transform {
                                        let camera_pos = cam_t.translation;
                                        let camera_forward = cam_t.forward();
                                        let mut found_hit = false;
                                        if camera_forward.y.abs() > 0.001 {
                                            let resting_y = 0.001;
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
                                        let yaw = f32::atan2(camera_forward.x, camera_forward.z);
                                        spawn_rotation = Quat::from_rotation_y(yaw);
                                    }
                                    let cmd = commands.spawn((
                                        Transform::from_translation(spawn_pos)
                                            .with_rotation(spawn_rotation)
                                            .with_scale(Vec3::new(2.0 * 0.28, 2.0 * 0.28, 0.001)),
                                        Name::new("Image"),
                                        crate::common::game::assets::components::Image {
                                            asset_id: 0,
                                            face: None,
                                        },
                                        Pickable::default(),
                                        Visibility::Visible,
                                    ));
                                    let new_entity = cmd.id();
                                    if let Some(parent) = selection.entity {
                                        commands.entity(parent).add_child(new_entity);
                                    }
                                    ui.memory_mut(|mem| mem.close_popup(popup_id));
                                }
                            }

                            if "mesh".contains(&search_query.to_lowercase()) {
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Meshes").color(egui::Color32::from_rgb(120, 120, 120)).size(12.0).strong());
                                if ui.add(egui::Button::image_and_text(
                                    (mesh_tex, egui::vec2(16.0, 16.0)),
                                    "Mesh",
                                )).clicked() {
                                    let mut spawn_pos = Vec3::new(0.0, 0.001, 0.0);
                                    let mut spawn_rotation = Quat::IDENTITY;
                                    if let Some(cam_t) = camera_transform {
                                        let camera_pos = cam_t.translation;
                                        let camera_forward = cam_t.forward();
                                        let mut found_hit = false;
                                        if camera_forward.y.abs() > 0.001 {
                                            let resting_y = 0.001;
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
                                        let yaw = f32::atan2(camera_forward.x, camera_forward.z);
                                        spawn_rotation = Quat::from_rotation_y(yaw);
                                    }
                                    let cmd = commands.spawn((
                                        Transform::from_translation(spawn_pos)
                                            .with_rotation(spawn_rotation)
                                            .with_scale(Vec3::ONE),
                                        Name::new("Mesh"),
                                        crate::common::game::assets::components::Mesh {
                                            asset_id: 0,
                                            normalize: false,
                                        },
                                        crate::common::game::bricks::components::BrickPhysics::default(),
                                        avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                                        Pickable::default(),
                                        Visibility::Visible,
                                    ));
                                    let new_entity = cmd.id();
                                    if let Some(parent) = selection.entity {
                                        commands.entity(parent).add_child(new_entity);
                                    }
                                    ui.memory_mut(|mem| mem.close_popup(popup_id));
                                }
                            }
                        },
                    );

                    ui.add_space(8.0);
                    ui.visuals_mut().window_fill = original_window_fill;
                    ui.visuals_mut().window_stroke = original_window_stroke;

                    ui.add_space(8.0);
                    let (sep_rect_play, _) = ui.allocate_exact_size(egui::vec2(1.0, 56.0), egui::Sense::hover());
                    ui.painter().rect_filled(sep_rect_play, 0.0, egui::Color32::from_rgb(212, 212, 212));
                    ui.add_space(8.0);

                    let is_playing = physics_state == crate::common::game::physics::PhysicsSimulationState::Running;
                    let play_btn_tex = if is_playing { stopp_tex } else { play_tex };
                    let play_btn_label = if is_playing { "Stop" } else { "Play" };

                    if ribbonbutton(ui, Some(play_btn_tex), play_btn_label, is_playing).clicked() {
                        if is_playing {
                            physics_action_writer.write(crate::common::game::physics::PhysicsSimulationAction::Stop);
                        } else {
                            physics_action_writer.write(crate::common::game::physics::PhysicsSimulationAction::Play);
                        }
                    }

                    let playtesting_active = playtest_state.active;
                    let playc_btn_label = if playtesting_active { "Stop Playtest" } else { "Play in Studio" };
                    let playc_btn_tex = if playtesting_active { stopp_tex } else { playc_tex };

                    if ribbonbutton(ui, Some(playc_btn_tex), playc_btn_label, playtesting_active).clicked() {
                        if playtesting_active {
                            playtest_state.active = false;
                            selection.entity = None;
                            selection.entities.clear();
                            selection.workspace_selected = false;
                            selection.players_selected = false;
                            selection.lighting_selected = false;

                            crate::scripting::output::end_run();
                            crate::app::server::bootstrap::request_playtest_server_shutdown();

                            if let Some(client_entity) = playtest_client_query.iter().next() {
                                commands.trigger(lightyear::prelude::client::Disconnect { entity: client_entity });
                                commands.entity(client_entity).try_despawn();
                            }

                            for (entity, _, _name, _, _, brick_opt, _, _, _, _, _, _) in entities_query.iter() {
                                let name_str = _name.as_str();
                                if brick_opt.is_some() || name_str == "Player" || name_str == "LocalPlayer" || name_str.starts_with("Player_") {
                                    commands.entity(entity).try_despawn();
                                }
                            }

                            for entity in replicated_images {
                                commands.entity(entity).try_despawn();
                            }

                            // Replicated mesh entities are not covered by the
                            // brick/player cleanup above; remove any that are
                            // still around so they don't leak or duplicate the
                            // restored ones below.
                            for (entity, _, _, _, _, _, _, _, _, _, mesh_opt) in explorer_query.iter() {
                                if mesh_opt.is_some() {
                                    commands.entity(entity).try_despawn();
                                }
                            }

                            let mut named_entities = std::collections::HashMap::new();
                            for brick_data in playtest_backup.bricks.drain(..) {
                                let name = brick_data.name.clone();
                                let new_entity = crate::common::game::bricks::data::spawn_from_data(commands, &brick_data);
                                named_entities.insert(name, new_entity);
                            }

                            for script_data in playtest_backup.scripts.drain(..) {
                                let mut cmd = commands.spawn(Name::new(script_data.name));
                                match script_data.script_type {
                                    0 => {
                                        cmd.insert(crate::scripting::ecs::ServerScript {
                                            code: script_data.code,
                                            enabled: script_data.enabled,
                                            started: false,
                                            running_code: String::new(),
                                        });
                                    }
                                    1 => {
                                        cmd.insert((
                                            crate::scripting::ecs::LocalScript {
                                                code: script_data.code,
                                                enabled: script_data.enabled,
                                                started: false,
                                                running_code: String::new(),
                                            },
                                            lightyear::prelude::Replicate::default(),
                                        ));
                                    }
                                    _ => {
                                        cmd.insert((
                                            crate::scripting::ecs::ModuleScript {
                                                code: script_data.code,
                                            },
                                            lightyear::prelude::Replicate::default(),
                                        ));
                                    }
                                }
                                let new_script_entity = cmd.id();
                                if let Some(ref p_name) = script_data.parent_name {
                                    if let Some(&parent_entity) = named_entities.get(p_name) {
                                        commands.entity(parent_entity).add_child(new_script_entity);
                                    }
                                }
                            }

                            for image_data in playtest_backup.images.drain(..) {
                                let face = image_data.face.as_deref().and_then(crate::common::game::assets::components::ImageFace::from_str);
                                let cmd = commands.spawn((
                                    image_data.transform,
                                    Name::new(image_data.name),
                                    crate::common::game::assets::components::Image {
                                        asset_id: image_data.asset_id,
                                        face,
                                    },
                                    Pickable::default(),
                                    Visibility::Visible,
                                ));
                                let new_image_entity = cmd.id();
                                if let Some(ref p_name) = image_data.parent_name {
                                    if let Some(&parent_entity) = named_entities.get(p_name) {
                                        commands.entity(parent_entity).add_child(new_image_entity);
                                    }
                                }
                            }

                            for mesh_data in playtest_backup.meshes.drain(..) {
                                let cmd = commands.spawn((
                                    mesh_data.transform,
                                    Name::new(mesh_data.name),
                                    crate::common::game::assets::components::Mesh {
                                        asset_id: mesh_data.asset_id,
                                        normalize: mesh_data.normalize,
                                    },
                                    crate::common::game::bricks::components::BrickPhysics {
                                        enabled: mesh_data.physics_enabled,
                                        bounciness: mesh_data.bounciness,
                                        player_can_collide: mesh_data.player_can_collide,
                                        friction: mesh_data.friction,
                                        gravity_scale: mesh_data.gravity_scale,
                                        mass: mesh_data.mass,
                                    },
                                    avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                                    Pickable::default(),
                                    Visibility::Visible,
                                ));
                                let new_mesh_entity = cmd.id();
                                if let Some(ref p_name) = mesh_data.parent_name {
                                    if let Some(&parent_entity) = named_entities.get(p_name) {
                                        commands.entity(parent_entity).add_child(new_mesh_entity);
                                    }
                                }
                            }

                            // Make sure the restored edit-mode world carries no
                            // transient simulation state (the studio physics
                            // state is only restored next frame).
                            for (entity, _, _, _, _, _, _, _, _, _, _, _) in entities_query.iter() {
                                commands.entity(entity).remove::<(
                                    avian3d::prelude::RigidBody,
                                    avian3d::prelude::Collider,
                                    crate::common::game::physics::TransformBackup,
                                    crate::common::game::physics::PhysicsAttached,
                                    avian3d::prelude::Friction,
                                    avian3d::prelude::Restitution,
                                    avian3d::prelude::GravityScale,
                                    avian3d::prelude::Mass,
                                    avian3d::prelude::LinearDamping,
                                    avian3d::prelude::AngularDamping,
                                    avian3d::prelude::SleepThreshold,
                                )>();
                            }

                            if let Some(gravity_val) = playtest_backup.gravity.take() {
                                if let Some(g) = gravity {
                                    g.0 = gravity_val;
                                }
                            }
                            if let Some(ps_val) = playtest_backup.players_service.take() {
                                if let Some(ps) = players_service {
                                    **ps = ps_val.clone();
                                }
                                if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write() {
                                    *shared = ps_val;
                                }
                            }
                        } else {
                            playtest_state.active = true;
                            selection.entity = None;
                            selection.entities.clear();
                            selection.workspace_selected = false;
                            selection.players_selected = false;
                            selection.lighting_selected = false;

                            crate::scripting::output::start_run("Playtest");

                            if let Some(g) = gravity.as_ref() {
                                playtest_backup.gravity = Some(g.0);
                            } else {
                                playtest_backup.gravity = None;
                            }
                            if let Some(ps) = players_service.as_ref() {
                                playtest_backup.players_service = Some((**ps).clone());
                            } else {
                                playtest_backup.players_service = None;
                            }

                            let mut backup_bricks = Vec::new();
                            for (entity, _, _name, _, _, brick_opt, _, _, _, _, _, _) in entities_query.iter() {
                                if brick_opt.is_some() {
                                    if let Some(data) = crate::common::game::bricks::data::capture_brick_data(entity, entities_query, studs_query, brick_colors, mesh_assets) {
                                        backup_bricks.push(data);
                                    }
                                    commands.entity(entity).despawn();
                                }
                            }
                            playtest_backup.bricks = backup_bricks;

                            let mut backup_scripts = Vec::new();
                            for (_entity, _name, child_of_opt, _, _, s_opt, l_opt, m_opt, _, _, _) in explorer_query.iter() {
                                let mut script_type_opt = None;
                                let mut code = String::new();
                                let mut enabled = true;
                                if let Some(s) = s_opt {
                                    script_type_opt = Some(0);
                                    code = s.code.clone();
                                    enabled = s.enabled;
                                } else if let Some(l) = l_opt {
                                    script_type_opt = Some(1);
                                    code = l.code.clone();
                                    enabled = l.enabled;
                                } else if let Some(m) = m_opt {
                                    script_type_opt = Some(2);
                                    code = m.code.clone();
                                }
                                if let Some(script_type) = script_type_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt {
                                        if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) {
                                            parent_name = Some(p_name.to_string());
                                        }
                                    }
                                    backup_scripts.push(crate::common::core::vrtx::VrtxScript {
                                        name: _name.to_string(),
                                        script_type,
                                        code,
                                        parent_name,
                                        enabled,
                                    });
                                    commands.entity(_entity).despawn();
                                }
                            }
                            playtest_backup.scripts = backup_scripts;

                            let mut backup_images = Vec::new();
                            for (_entity, _name, child_of_opt, image_opt) in images_query.iter() {
                                if let Some(image) = image_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt {
                                        if let Ok((_, p_name, _, _)) = images_query.get(child_of.parent()) {
                                            parent_name = Some(p_name.to_string());
                                        }
                                    }
                                    let transform = entities_query
                                        .get(_entity)
                                        .map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t)
                                        .unwrap_or_default();
                                    backup_images.push(crate::common::core::vrtx::VrtxImage {
                                        name: _name.to_string(),
                                        asset_id: image.asset_id,
                                        face: image.face.as_ref().map(|f| f.as_str().to_string()),
                                        parent_name,
                                        transform,
                                    });
                                    commands.entity(_entity).despawn();
                                }
                            }
                            playtest_backup.images = backup_images;

                            let mut backup_meshes = Vec::new();
                            for (_entity, _name, child_of_opt, _, _, _, _, _, _, _, mesh_opt) in explorer_query.iter() {
                                if let Some(mesh) = mesh_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt {
                                        if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) {
                                            parent_name = Some(p_name.to_string());
                                        }
                                    }
                                    let transform = entities_query
                                        .get(_entity)
                                        .map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t)
                                        .unwrap_or_default();
                                    let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = entities_query
                                        .get(_entity)
                                        .map(|(_, _, _, _, _, _, _, _, _, _, _, phys_opt)| {
                                            if let Some(phys) = phys_opt {
                                                (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass)
                                            } else {
                                                (true, 0.3, true, 0.3, 1.0, 1.0)
                                            }
                                        })
                                        .unwrap_or((true, 0.3, true, 0.3, 1.0, 1.0));
                                    backup_meshes.push(crate::common::core::vrtx::VrtxMesh {
                                        name: _name.to_string(),
                                        asset_id: mesh.asset_id,
                                        normalize: mesh.normalize,
                                        parent_name,
                                        transform,
                                        physics_enabled,
                                        bounciness,
                                        player_can_collide,
                                        friction,
                                        gravity_scale,
                                        mass,
                                    });
                                    commands.entity(_entity).despawn();
                                }
                            }
                            playtest_backup.meshes = backup_meshes;

                            let mut backup_textures = Vec::new();
                            for (_entity, _name, child_of_opt, _, _, _, _, _, _, texture_opt, _) in explorer_query.iter() {
                                if let Some(texture) = texture_opt {
                                    let mut parent_name = None;
                                    if let Some(child_of) = child_of_opt {
                                        if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) = explorer_query.get(child_of.parent()) {
                                            parent_name = Some(p_name.to_string());
                                        }
                                    }
                                    backup_textures.push(crate::common::core::vrtx::VrtxTexture {
                                        name: _name.to_string(),
                                        id_string: texture.as_content_id(),
                                        parent_name,
                                    });
                                    commands.entity(_entity).despawn();
                                }
                            }
                            playtest_backup.textures = backup_textures;

                            let temp_map_path = "temp_play.vrtx".to_string();
                            let players = if let Some(ps) = players_service.as_ref() {
                                crate::common::core::vrtx::VrtxPlayers {
                                    speed: ps.speed,
                                    jump_power: ps.jump_power,
                                    gravity: ps.gravity,
                                    speed_response: ps.speed_response,
                                    friction: ps.friction,
                                    bounciness: ps.bounciness,
                                }
                            } else {
                                crate::common::core::vrtx::VrtxPlayers::default()
                            };
                            let state = crate::common::core::vrtx::VrtxFileState {
                                version: crate::common::core::vrtx::FORMAT_VERSION,
                                gravity: Vec3::new(0.0, -186.9 * 0.28, 0.0),
                                settings: crate::common::core::vrtx::VrtxSettings {
                                    ssao: graphics_settings.ssao,
                                    contact_shadows: graphics_settings.contact_shadows,
                                    bloom: graphics_settings.bloom,
                                },
                                lighting: crate::common::core::vrtx::VrtxLighting::from(&**lighting_config),
                                players,
                                camera_transform: Transform::IDENTITY,
                                bricks: playtest_backup.bricks.iter().map(|b| {
                                    let mut current_color = Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0));
                                    if let Some(ref studs_material_handle) = b.studs_material {
                                        if let Some(mat) = studs_materials.get(&studs_material_handle.0) {
                                            current_color = mat.base.base_color;
                                        }
                                    } else if let Some(ref mat_handle) = b.standard_material {
                                        if let Some(mat) = materials.get(&mat_handle.0) {
                                            current_color = mat.base.base_color;
                                        }
                                    }
                                    let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = if let Some(phys) = b.physics {
                                        (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass)
                                    } else {
                                        (true, 0.3, true, 0.3, 1.0, 1.0)
                                    };
                                    crate::common::core::vrtx::VrtxBrick {
                                        name: b.name.clone(),
                                        transform: b.transform,
                                        shape: b.shape,
                                        color: current_color,
                                        physics_enabled,
                                        bounciness,
                                        player_can_collide,
                                        friction,
                                        gravity_scale,
                                        mass,
                                        show_studs: b.studs,
                                    }
                                }).collect(),
                                scripts: playtest_backup.scripts.clone(),
                                images: playtest_backup.images.clone(),
                                meshes: playtest_backup.meshes.clone(),
                                textures: playtest_backup.textures.clone(),
                            };

                            if state.save_to_file(&temp_map_path).is_ok() {
                                crate::app::server::bootstrap::begin_playtest_server_epoch();
                                let playtest_port = crate::app::server::bootstrap::pick_free_playtest_port();

                                let playtest_key: [u8; 32] = rand::random();
                                let playtest_protocol_id: u64 = rand::random();

                                let server_app = crate::app::server::bootstrap::RaveServerApp::new(
                                    crate::app::server::config::ServerAppConfig {
                                        port: playtest_port,
                                        map_path: temp_map_path,
                                        bind_addr: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                                        netcode_key: playtest_key,
                                        protocol_id: playtest_protocol_id,
                                        allow_unauthenticated: true,
                                    }
                                );
                                std::thread::spawn(move || {
                                    server_app.run();
                                });

                                std::thread::sleep(std::time::Duration::from_millis(100));

                                let server_addr = std::net::SocketAddr::new(
                                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                                    playtest_port,
                                );
                                let client_id = rand::random::<u64>();

                                commands.insert_resource(crate::client::LocalClientId(client_id));
                                commands.insert_resource(crate::client::ClientUkey("studio_play_local_key".to_string()));

                                let auth = lightyear::prelude::Authentication::Manual {
                                    server_addr,
                                    client_id,
                                    private_key: playtest_key,
                                    protocol_id: playtest_protocol_id,
                                };

                                let netcode_config = lightyear::prelude::client::NetcodeConfig {
                                    client_timeout_secs: 15,
                                    ..default()
                                };

                                let client_entity = commands.spawn((
                                    lightyear::prelude::client::Client::default(),
                                    lightyear::prelude::UdpIo::default(),
                                    lightyear::prelude::client::NetcodeClient::new(auth, netcode_config).unwrap(),
                                    lightyear::prelude::LocalAddr(std::net::SocketAddr::new(
                                        std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                                        0,
                                    )),
                                    lightyear::prelude::PeerAddr(server_addr),
                                    crate::studio::ui::resources::InEditorPlaytestClient,
                                )).id();

                                commands.trigger(lightyear::prelude::client::Connect { entity: client_entity });
                            }
                        }
                    }
                });
            });
        });

    let (bottom_sep, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(bottom_sep, 0.0, egui::Color32::from_rgb(180, 180, 180));
}

#[allow(deprecated)]
fn ribbonbutton(
    ui: &mut egui::Ui,
    icon: Option<egui::TextureId>,
    label: &str,
    selected: bool,
) -> egui::Response {
    let width = if label.len() > 8 { 88.0 } else { 56.0 };
    let size = egui::vec2(width, 56.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if selected {
        ui.painter()
            .rect_filled(rect, 4.0, egui::Color32::from_rgb(204, 232, 255));
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(153, 209, 255)),
            egui::StrokeKind::Inside,
        );
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 4.0, egui::Color32::from_rgb(224, 238, 249));
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 220, 240)),
            egui::StrokeKind::Inside,
        );
    }

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.vertical_centered(|ui| {
            if label.is_empty() {
                if let Some(texture_id) = icon {
                    ui.add_space(14.0);
                    ui.add(egui::Image::new((texture_id, egui::vec2(28.0, 28.0))));
                }
            } else {
                if let Some(texture_id) = icon {
                    ui.add_space(5.0);
                    ui.add(egui::Image::new((texture_id, egui::vec2(28.0, 28.0))));
                    ui.add_space(1.0);
                } else {
                    ui.add_space(18.0);
                }
                let text_color = egui::Color32::from_rgb(20, 20, 20);
                ui.label(egui::RichText::new(label).color(text_color).size(11.0));
            }
        });
    });

    response
}
