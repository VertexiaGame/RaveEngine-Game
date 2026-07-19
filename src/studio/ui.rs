pub mod assets;
pub mod indicator;
pub mod panels;
pub mod visuals;
pub mod resources;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle};
use crate::studio::tools::ToolState;
use crate::studio::tools::Selection;
use crate::common::game::bricks::components::Brick;
use bevy::ecs::system::SystemParam;
use bevy::pbr::ExtendedMaterial;

pub use assets::{StudioUiAssets, StudioUiTextureIds, setup_ui_assets};
pub use indicator::{CameraSpeedIndicator, updatecameraspeedindicator, FovIndicator, update_camera_fov};
pub use visuals::configure_visuals;
pub use resources::{CopiedEntityBuffer, HierarchyDraggedEntity, SettingsWindow, ActiveScriptEditor, FileDialogState};
pub use crate::common::core::performance::GraphicsSettings;

pub(crate) fn line_numbers(cache: &mut Option<(usize, String)>, total_lines: usize) -> &str {
    if cache.as_ref().map_or(true, |cached| cached.0 != total_lines) {
        let max_digit_width = total_lines.to_string().len();
        let mut text = String::with_capacity(total_lines * (max_digit_width + 1));
        for line in 1..=total_lines {
            text.push_str(&format!("{:>width$}\n", line, width = max_digit_width));
        }
        *cache = Some((total_lines, text));
    }
    cache.as_ref().unwrap().1.as_str()
}

#[derive(SystemParam)]
pub struct UiResources<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub studs_materials: ResMut<'w, Assets<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
    pub studs_assets: Res<'w, crate::common::game::bricks::studs::StudsAssets>,
    pub count: ResMut<'w, crate::common::game::bricks::data::BrickSpawnerCount>,
    pub snap_config: ResMut<'w, crate::studio::tools::SnapConfig>,
    pub history: ResMut<'w, crate::studio::tools::UndoRedoHistory>,
    pub action_writer: MessageWriter<'w, crate::studio::tools::UndoRedoAction>,
    pub physics_state: Res<'w, crate::common::game::physics::PhysicsSimulationState>,
    pub physics_action_writer: MessageWriter<'w, crate::common::game::physics::PhysicsSimulationAction>,
    pub gravity: Option<ResMut<'w, avian3d::prelude::Gravity>>,
    pub brick_colors: Query<'w, 's, &'static mut crate::common::game::bricks::components::BrickColor>,
    pub players_service: Option<ResMut<'w, crate::studio::tools::PlayersService>>,
    pub lighting_service: Option<ResMut<'w, crate::common::game::environment::lighting::LightingService>>,
}

#[derive(SystemParam)]
pub struct UiStateResources<'w> {
    pub next_tool: ResMut<'w, NextState<ToolState>>,
    pub current_tool: Res<'w, State<ToolState>>,
    pub ui_assets: Option<Res<'w, StudioUiAssets>>,
    pub texture_ids: ResMut<'w, StudioUiTextureIds>,
    pub cameraindicator: ResMut<'w, CameraSpeedIndicator>,
    pub fovindicator: ResMut<'w, FovIndicator>,
    pub diagnostics: Res<'w, bevy::diagnostic::DiagnosticsStore>,
    pub selection: ResMut<'w, Selection>,
    pub copiedbuffer: ResMut<'w, CopiedEntityBuffer>,
    pub dragged_entity: ResMut<'w, HierarchyDraggedEntity>,
    pub context_menu: ResMut<'w, crate::studio::tools::CanvasContextMenu>,
    pub hover_state: ResMut<'w, crate::studio::tools::HoverState>,
    pub settings_window: ResMut<'w, SettingsWindow>,
    pub graphics_settings: ResMut<'w, GraphicsSettings>,
    pub onboarding_state: Res<'w, State<crate::studio::tools::OnboardingState>>,
    pub next_onboarding_state: ResMut<'w, NextState<crate::studio::tools::OnboardingState>>,
    pub onboarding_data: ResMut<'w, crate::studio::ui::panels::onboarding::OnboardingData>,
    pub marquee_state: Res<'w, crate::studio::tools::MarqueeState>,
    pub play_processes: ResMut<'w, crate::studio::ui::resources::PlayInClientProcesses>,
    pub playtest_state: ResMut<'w, crate::client::PlaytestState>,
    pub playtest_backup: ResMut<'w, crate::studio::ui::resources::PlaytestBackup>,
    pub active_editor: ResMut<'w, ActiveScriptEditor>,
    pub file_dialog_state: ResMut<'w, FileDialogState>,
}

#[derive(SystemParam)]
pub struct UiQueries<'w, 's> {
    pub cameraquery: Query<
        'w,
        's,
        (
            &'static bevy::camera_controller::free_camera::FreeCamera,
            &'static mut bevy::camera_controller::free_camera::FreeCameraState,
        ),
    >,
    pub camera_projection_query: Query<'w, 's, &'static mut Projection, With<Camera3d>>,
    pub camera_transform_query: Query<'w, 's, &'static mut Transform, With<Camera3d>>,
    pub entities_query: Query<
        'w,
        's,
        (
            Entity,
            &'static mut Transform,
            &'static Name,
            Option<&'static ChildOf>,
            Option<&'static Children>,
            Option<&'static Brick>,
            Option<&'static mut crate::common::game::bricks::components::BrickShapeComponent>,
            &'static GlobalTransform,
            Option<&'static Mesh3d>,
            Option<&'static MeshMaterial3d<StandardMaterial>>,
            Option<&'static MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
            Option<&'static mut crate::common::game::bricks::components::BrickPhysics>,
        ),
        Without<Camera3d>,
    >,
    pub explorer_query: Query<
        'w,
        's,
        (
            Entity,
            &'static Name,
            Option<&'static ChildOf>,
            Option<&'static Children>,
            Option<&'static Brick>,
            Option<&'static crate::scripting::ecs::ServerScript>,
            Option<&'static crate::scripting::ecs::LocalScript>,
            Option<&'static crate::scripting::ecs::ModuleScript>,
        ),
        Without<Camera3d>,
    >,
    pub playtest_client_query: Query<'w, 's, Entity, With<crate::studio::ui::resources::InEditorPlaytestClient>>,
    pub playtest_players: Query<'w, 's, Entity, With<crate::common::net::components::Player>>,
    pub playtest_cameras: Query<'w, 's, Entity, With<crate::client::player::PlayerCamera>>,
    pub playtest_visuals: Query<'w, 's, Entity, With<crate::client::PlayerVisualChild>>,
}

#[allow(deprecated)]
pub fn studio_ui(
    mut contexts: EguiContexts,
    mut ui_res: UiResources,
    mut ui_state: UiStateResources<'_>,
    mut queries: UiQueries<'_, '_>,
) {
    let Some(assets) = &ui_state.ui_assets else { return; };

    let thumb_empty_tex = *ui_state.texture_ids.thumb_empty_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.thumb_empty.clone()))
    });
    let thumb_baseplate_tex = *ui_state.texture_ids.thumb_baseplate_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.thumb_baseplate.clone()))
    });
    let move_tex = *ui_state.texture_ids.move_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.move_icon.clone()))
    });
    let rotate_tex = *ui_state.texture_ids.rotate_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.rotate_icon.clone()))
    });
    let scale_tex = *ui_state.texture_ids.scale_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.scale_icon.clone()))
    });
    let add_tex = *ui_state.texture_ids.add_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.add_icon.clone()))
    });
    let workspace_tex = *ui_state.texture_ids.workspace_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.workspace_icon.clone()))
    });
    let brick_tex = *ui_state.texture_ids.brick_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.brick_icon.clone()))
    });
    let players_tex = *ui_state.texture_ids.players_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.players_icon.clone()))
    });
    let lighting_tex = *ui_state.texture_ids.lighting_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.lighting_icon.clone()))
    });
    let play_tex = *ui_state.texture_ids.play_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.play_icon.clone()))
    });
    let playc_tex = *ui_state.texture_ids.playc_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.playc_icon.clone()))
    });
    let stopp_tex = *ui_state.texture_ids.stopp_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.stopp_icon.clone()))
    });
    let script_tex = *ui_state.texture_ids.script_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.script_icon.clone()))
    });
    let localscript_tex = *ui_state.texture_ids.localscript_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.localscript_icon.clone()))
    });
    let modulescript_tex = *ui_state.texture_ids.modulescript_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.modulescript_icon.clone()))
    });

    let Ok(ctx) = contexts.ctx_mut() else { return; };
    ctx.set_visuals(egui::Visuals::light());

    let onboarding_active = *ui_state.onboarding_state.get() != crate::studio::tools::OnboardingState::Inactive;

    if ui_state.playtest_state.active {
        egui::Area::new(egui::Id::new("stop_playtest_overlay"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 10.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(61, 61, 61, 200))
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let stop_btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Stop playtest")
                                        .color(egui::Color32::WHITE)
                                        .strong()
                                        .size(14.0)
                                )
                                .fill(egui::Color32::from_rgb(190, 60, 60))
                            );
                            if stop_btn.clicked() {
                                ui_state.playtest_state.active = false;

                                crate::app::server::bootstrap::SHUTDOWN_SERVER.store(true, std::sync::atomic::Ordering::Relaxed);

                                for e in queries.playtest_client_query.iter() {
                                    ui_res.commands.trigger(lightyear::prelude::client::Disconnect { entity: e });
                                    if let Ok(mut entity_cmd) = ui_res.commands.get_entity(e) {
                                        entity_cmd.despawn();
                                    }
                                }

                                for (entity, _, name, _, _, brick_opt, _, _, _, _, _, _) in queries.entities_query.iter() {
                                    let name_str = name.as_str();
                                    if brick_opt.is_some() || name_str == "Player" || name_str.starts_with("Player_") {
                                        if let Ok(mut e) = ui_res.commands.get_entity(entity) {
                                            e.despawn();
                                        }
                                    }
                                }

                                for camera_entity in queries.playtest_cameras.iter() {
                                    if let Ok(mut e) = ui_res.commands.get_entity(camera_entity) {
                                        e.despawn();
                                    }
                                }

                                for visual_entity in queries.playtest_visuals.iter() {
                                    if let Ok(mut e) = ui_res.commands.get_entity(visual_entity) {
                                        e.despawn();
                                    }
                                }

                                for player_entity in queries.playtest_players.iter() {
                                    if let Ok(mut e) = ui_res.commands.get_entity(player_entity) {
                                        e.despawn();
                                    }
                                }

                                for (entity, _, name, child_of_opt, _, brick_opt, _, _, _, _, _, _) in queries.entities_query.iter() {
                                    if child_of_opt.is_none() && brick_opt.is_none() {
                                        let n = name.as_str();
                                        if n.contains("Armature") || n == "LocalPlayer" || n.starts_with("Player_") {
                                            if let Ok(mut e) = ui_res.commands.get_entity(entity) {
                                                e.despawn();
                                            }
                                        }
                                    }
                                }

                                for (entity, _, _, _, _, s_opt, l_opt, m_opt) in queries.explorer_query.iter() {
                                    if s_opt.is_some() || l_opt.is_some() || m_opt.is_some() {
                                        if let Ok(mut e) = ui_res.commands.get_entity(entity) {
                                            e.despawn();
                                        }
                                    }
                                }

                                let mut named_entities = std::collections::HashMap::new();
                                for brick_data in ui_state.playtest_backup.bricks.drain(..) {
                                    let name = brick_data.name.clone();
                                    let new_entity = crate::common::game::bricks::data::spawn_from_data(&mut ui_res.commands, &brick_data);
                                    named_entities.insert(name, new_entity);
                                }

                                for script_data in ui_state.playtest_backup.scripts.drain(..) {
                                    let mut cmd = ui_res.commands.spawn(Name::new(script_data.name));
                                    match script_data.script_type {
                                        0 => {
                                            cmd.insert(crate::scripting::ecs::ServerScript {
                                                code: script_data.code,
                                                enabled: script_data.enabled,
                                                started: false,
                                            });
                                        }
                                        1 => {
                                            cmd.insert((
                                                crate::scripting::ecs::LocalScript {
                                                    code: script_data.code,
                                                    enabled: script_data.enabled,
                                                    started: false,
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
                                            ui_res.commands.entity(parent_entity).add_child(new_script_entity);
                                        }
                                    }
                                }

                                if let Some(gravity_val) = ui_state.playtest_backup.gravity.take() {
                                    if let Some(ref mut g) = ui_res.gravity {
                                        g.0 = gravity_val;
                                    }
                                }
                                if let Some(ps_val) = ui_state.playtest_backup.players_service.take() {
                                    if let Some(ref mut ps) = ui_res.players_service {
                                        **ps = ps_val.clone();
                                    }
                                    if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write() {
                                        *shared = ps_val;
                                    }
                                }
                                if let Some(ls_val) = ui_state.playtest_backup.lighting_service.take() {
                                    if let Some(ref mut ls) = ui_res.lighting_service {
                                        **ls = ls_val.clone();
                                    }
                                    if let Ok(mut shared) = crate::studio::tools::SHARED_LIGHTING_SERVICE.write() {
                                        *shared = ls_val.time_of_day;
                                    }
                                }
                            }
                        });
                    });
            });
        return;
    }

    let frame = egui::Frame::NONE
        .fill(egui::Color32::from_rgb(245, 246, 247))
        .inner_margin(egui::Margin::same(0));

    let camera_transform_val = queries.camera_transform_query.iter().next().map(|t| *t);

    let top_bar_res = egui::Panel::top("topbar")
        .frame(frame)
        .show(ctx, |ui| {
            panels::draw_top_bar(
                ui,
                &mut ui_state.next_tool,
                &ui_state.current_tool,
                &mut ui_res.commands,
                &mut ui_res.meshes,
                &mut ui_res.materials,
                &mut ui_res.studs_materials,
                &ui_res.studs_assets,
                &mut ui_res.count,
                &mut ui_res.snap_config,
                move_tex,
                rotate_tex,
                scale_tex,
                add_tex,
                play_tex,
                playc_tex,
                stopp_tex,
                &ui_state.diagnostics,
                camera_transform_val.as_ref(),
                &mut ui_res.action_writer,
                &mut ui_res.history,
                *ui_res.physics_state,
                &mut ui_res.physics_action_writer,
                &mut ui_state.settings_window,
                &mut ui_state.graphics_settings,
                &mut ui_res.gravity,
                &mut queries.camera_transform_query,
                &mut queries.entities_query,
                &mut ui_state.onboarding_data,
                &mut ui_state.play_processes,
                &mut ui_state.playtest_state,
                &mut ui_state.playtest_backup,
                &queries.playtest_client_query,
                &ui_state.selection,
                &queries.explorer_query,
                onboarding_active,
                &mut ui_res.players_service,
                &mut ui_res.lighting_service,
                &ui_state.file_dialog_state,
            );
        });

    let panel_res = egui::SidePanel::left("explorer")
        .frame(egui::Frame::none()
            .fill(egui::Color32::from_rgb(245, 246, 247))
            .inner_margin(egui::Margin::symmetric(12, 12))
        )
        .default_width(220.0)
        .show(ctx, |ui| {
            ui.add_enabled_ui(!onboarding_active, |ui| {
                let total_height = ui.available_height();

                let mut selected_bricks = Vec::new();
                for &entity in &ui_state.selection.entities {
                    if let Ok((_, _, _, _, _, Some(_), _, _, _, _, _, _)) = queries.entities_query.get(entity) {
                        selected_bricks.push(entity);
                    }
                }

                let mut selected_scripts = Vec::new();
                for &entity in &ui_state.selection.entities {
                    if let Ok((_, _, _, _, _, server_opt, local_opt, module_opt)) = queries.explorer_query.get(entity) {
                        if server_opt.is_some() || local_opt.is_some() || module_opt.is_some() {
                            selected_scripts.push(entity);
                        }
                    }
                }

                let has_selection = !selected_bricks.is_empty() || !selected_scripts.is_empty() || ui_state.selection.workspace_selected || ui_state.selection.players_selected || ui_state.selection.lighting_selected;
                let mut explorer_height = if has_selection {
                    let id = ui.make_persistent_id("explorer_height_split");
                    ui.data_mut(|d| d.get_temp::<f32>(id).unwrap_or(180.0))
                } else {
                    total_height
                };

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), explorer_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_source("explorer_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                panels::draw_explorer(
                                    ui,
                                    &mut ui_res.commands,
                                    &mut ui_state.selection,
                                    &queries.explorer_query,
                                    &queries.entities_query,
                                    &mut ui_state.copiedbuffer,
                                    &mut ui_state.dragged_entity,
                                    &mut ui_res.history,
                                    &mut ui_state.active_editor,
                                    workspace_tex,
                                    brick_tex,
                                    players_tex,
                                    lighting_tex,
                                    script_tex,
                                    localscript_tex,
                                    modulescript_tex,
                                );
                            });
                    }
                );

                if !selected_bricks.is_empty() || !selected_scripts.is_empty() {
                    let sep_height = 20.0;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), sep_height),
                        egui::Sense::click_and_drag()
                    );

                    if response.hovered() || response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }

                    if response.dragged() {
                        let delta_y = response.drag_delta().y;
                        let max_limit = (total_height - 100.0).max(100.0);
                        explorer_height = (explorer_height + delta_y).clamp(50.0, max_limit);
                        let id = ui.make_persistent_id("explorer_height_split");
                        ui.data_mut(|d| d.insert_temp(id, explorer_height));
                    }

                    let line_y = rect.center().y;
                    let line_rect = egui::Rect::from_x_y_ranges(
                        rect.left()..=rect.right(),
                        (line_y - 0.5)..=(line_y + 0.5)
                    );
                    ui.painter().rect_filled(line_rect, 0.0, egui::Color32::from_rgb(180, 180, 180));

                    egui::ScrollArea::vertical()
                        .id_source("properties_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            panels::draw_properties(
                                ui,
                                &ui_state.selection.entities,
                                &mut ui_res.commands,
                                &mut queries.entities_query,
                                &mut ui_res.brick_colors,
                                &mut ui_res.materials,
                                &mut ui_res.studs_materials,
                                &queries.explorer_query,
                                &mut ui_state.active_editor,
                            );
                        });
                } else if ui_state.selection.workspace_selected {
                    let sep_height = 20.0;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), sep_height),
                        egui::Sense::click_and_drag()
                    );

                    if response.hovered() || response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }

                    if response.dragged() {
                        let delta_y = response.drag_delta().y;
                        let max_limit = (total_height - 100.0).max(100.0);
                        explorer_height = (explorer_height + delta_y).clamp(50.0, max_limit);
                        let id = ui.make_persistent_id("explorer_height_split");
                        ui.data_mut(|d| d.insert_temp(id, explorer_height));
                    }

                    let line_y = rect.center().y;
                    let line_rect = egui::Rect::from_x_y_ranges(
                        rect.left()..=rect.right(),
                        (line_y - 0.5)..=(line_y + 0.5)
                    );
                    ui.painter().rect_filled(line_rect, 0.0, egui::Color32::from_rgb(180, 180, 180));

                    egui::ScrollArea::vertical()
                        .id_source("properties_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            panels::draw_workspace_properties(
                                ui,
                                &mut ui_res.gravity,
                            );
                        });
                } else if ui_state.selection.players_selected {
                    let sep_height = 20.0;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), sep_height),
                        egui::Sense::click_and_drag()
                    );

                    if response.hovered() || response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }

                    if response.dragged() {
                        let delta_y = response.drag_delta().y;
                        let max_limit = (total_height - 100.0).max(100.0);
                        explorer_height = (explorer_height + delta_y).clamp(50.0, max_limit);
                        let id = ui.make_persistent_id("explorer_height_split");
                        ui.data_mut(|d| d.insert_temp(id, explorer_height));
                    }

                    let line_y = rect.center().y;
                    let line_rect = egui::Rect::from_x_y_ranges(
                        rect.left()..=rect.right(),
                        (line_y - 0.5)..=(line_y + 0.5)
                    );
                    ui.painter().rect_filled(line_rect, 0.0, egui::Color32::from_rgb(180, 180, 180));

                    egui::ScrollArea::vertical()
                        .id_source("properties_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            panels::draw_players_properties(
                                ui,
                                &mut ui_res.players_service,
                            );
                        });
                } else if ui_state.selection.lighting_selected {
                    let sep_height = 20.0;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), sep_height),
                        egui::Sense::click_and_drag()
                    );

                    if response.hovered() || response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }

                    if response.dragged() {
                        let delta_y = response.drag_delta().y;
                        let max_limit = (total_height - 100.0).max(100.0);
                        explorer_height = (explorer_height + delta_y).clamp(50.0, max_limit);
                        let id = ui.make_persistent_id("explorer_height_split");
                        ui.data_mut(|d| d.insert_temp(id, explorer_height));
                    }

                    let line_y = rect.center().y;
                    let line_rect = egui::Rect::from_x_y_ranges(
                        rect.left()..=rect.right(),
                        (line_y - 0.5)..=(line_y + 0.5)
                    );
                    ui.painter().rect_filled(line_rect, 0.0, egui::Color32::from_rgb(180, 180, 180));

                    egui::ScrollArea::vertical()
                        .id_source("properties_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            panels::draw_lighting_properties(
                                ui,
                                &mut ui_res.lighting_service,
                            );
                        });
                }
            });
        });

    if let Some(dragged) = ui_state.dragged_entity.entity {
        if panel_res.response.hovered() && ctx.input(|i| i.pointer.any_released()) {
            if let Ok((_, _, _, child_of_opt, _, _, _, child_global, _, _, _, _)) = queries.entities_query.get(dragged) {
                let old_parent = child_of_opt.map(|co| co.parent());
                let old_transform = queries.entities_query.get(dragged).ok().map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t).unwrap_or(Transform::IDENTITY);

                let new_transform = Transform {
                    translation: child_global.translation(),
                    rotation: child_global.rotation(),
                    scale: child_global.scale(),
                };

                if let Ok(mut d_cmd) = ui_res.commands.get_entity(dragged) {
                    d_cmd.insert(new_transform);
                    d_cmd.remove::<ChildOf>();
                }

                ui_res.history.push_command(crate::studio::tools::UndoCommand::ParentChange {
                    entity: dragged,
                    old_parent,
                    new_parent: None,
                    old_transform,
                    new_transform,
                });
            }
            ui_state.dragged_entity.entity = None;
        }
    }

    if ui_state.settings_window.open {
        panels::draw_settings_window(ctx, &mut ui_state.settings_window.open, &mut ui_state.graphics_settings);
    }

    indicator::draw_indicator(ctx, &mut ui_state.cameraindicator, &mut queries.cameraquery);
    indicator::draw_fov_indicator(ctx, &mut ui_state.fovindicator, &mut queries.camera_projection_query);

    if let (Some(entity), Some(pos)) = (ui_state.context_menu.entity, ui_state.context_menu.position) {
        let mut open_status = true;

        let inner_res = egui::Area::new(egui::Id::new("hahasosigma"))
            .fixed_pos(egui::pos2(pos.x, pos.y))
            .show(ctx, |ui| {
                let frame = egui::Frame::menu(ui.style());
                frame.show(ui, |ui| {
                    ui.set_min_width(120.0);
                    panels::draw_entity_context_menu(
                        ui,
                        entity,
                        &mut ui_res.commands,
                        &mut ui_state.selection,
                        &mut ui_state.copiedbuffer,
                        &queries.entities_query,
                        &mut ui_res.history,
                    )
                })
            });

        let clicked_button = inner_res.inner.inner;
        if clicked_button {
            open_status = false;
        }

        if !ui_state.context_menu.just_opened && ctx.input(|i| i.pointer.any_pressed()) {
            if let Some(mouse_pos) = ctx.pointer_interact_pos() {
                if !inner_res.response.rect.contains(mouse_pos) {
                    open_status = false;
                }
            }
        }

        ui_state.context_menu.just_opened = false;

        if !open_status {
            ui_state.context_menu.entity = None;
            ui_state.context_menu.position = None;
        }
    }

    let mut script_editor_rect = None;
    if !ui_state.active_editor.open_entities.is_empty() {
        if ui_state.active_editor.entity.is_none() {
            ui_state.active_editor.entity = ui_state.active_editor.open_entities.first().copied();
        }

        if let Some(active_entity) = ui_state.active_editor.entity {
            let mut script_found = false;
            let mut current_source = String::new();

            if let Ok((_, _, _, _, _, server_opt, local_opt, module_opt)) = queries.explorer_query.get(active_entity) {
                if let Some(ref script) = server_opt {
                    current_source = script.code.clone();
                    script_found = true;
                } else if let Some(ref script) = local_opt {
                    current_source = script.code.clone();
                    script_found = true;
                } else if let Some(ref script) = module_opt {
                    current_source = script.code.clone();
                    script_found = true;
                }
            }

            if script_found {
                let bg_color = egui::Color32::from_rgb(255, 255, 255);
                let tab_bar_color = egui::Color32::from_rgb(240, 241, 242);
                let active_tab_color = egui::Color32::from_rgb(255, 255, 255);
                let border_color = egui::Color32::from_rgb(212, 212, 212);

                let mut should_close_tab = None;
                let mut should_save = false;
                let mut active_tab_rect = None;

                let last_change_id = egui::Id::new(("last_change", active_entity));
                let current_time = ctx.input(|i| i.time);

                let mut state = ctx.data_mut(|d| {
                    d.get_temp::<(f64, String, Option<String>, bool, Option<(usize, String)>)>(last_change_id)
                        .unwrap_or((-1.0, current_source.clone(), None, true, None))
                });

                if current_source != state.1 {
                    state.0 = current_time;
                    state.1 = current_source.clone();
                    state.3 = true;
                }

                if state.3 && current_time - state.0 >= 0.8 {
                    let compiler = mlua::chunk::Compiler::default();
                    state.2 = match compiler.compile(&state.1) {
                        Ok(_) => None,
                        Err(e) => {
                            let err_msg = e.to_string();
                            let clean_msg = if err_msg.contains("[string ") {
                                let parts: Vec<&str> = err_msg.splitn(3, ':').collect();
                                if parts.len() >= 3 {
                                    format!("Line {}: {}", parts[1].trim(), parts[2].trim())
                                } else {
                                    err_msg.clone()
                                }
                            } else {
                                err_msg.clone()
                            };
                            Some(clean_msg)
                        }
                    };
                    state.3 = false;
                }

                let compile_error = state.2.clone();

                let panel_res = egui::CentralPanel::default()
                    .frame(egui::Frame::none()
                        .fill(egui::Color32::TRANSPARENT)
                        .inner_margin(egui::Margin {
                            left: 12,
                            right: 12,
                            top: 12,
                            bottom: 12,
                        }))
                    .show(ctx, |ui| {
                        ui.style_mut().visuals = egui::Visuals::light();

                        egui::Frame::none()
                            .fill(bg_color)
                            .stroke(egui::Stroke::new(1.0, border_color))
                            .corner_radius(8.0)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    let tab_height = 36.0;
                                    let bar_res = egui::Frame::none()
                                        .fill(tab_bar_color)
                                        .corner_radius(egui::CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 })
                                        .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 0 })
                                        .show(ui, |ui| {
                                            ui.set_height(tab_height);
                                            ui.horizontal(|ui| {
                                                ui.set_height(tab_height);
                                                ui.with_layout(egui::Layout::left_to_right(egui::Align::Max), |ui| {
                                                    let open_entities_cloned = ui_state.active_editor.open_entities.clone();
                                                    for &open_entity in &open_entities_cloned {
                                                        let is_active = ui_state.active_editor.entity == Some(open_entity);
                                                        let open_script_name = queries
                                                            .explorer_query
                                                            .get(open_entity)
                                                            .map(|(_, name, _, _, _, _, _, _)| name.as_str().to_string())
                                                            .unwrap_or_else(|_| "Script".to_string());

                                                        let is_local_tab = queries
                                                            .explorer_query
                                                            .get(open_entity)
                                                            .map(|(_, _, _, _, _, _, local_opt, _)| {
                                                                local_opt.is_some()
                                                            })
                                                            .unwrap_or(false);

                                                        let tab_icon = if is_local_tab { localscript_tex } else { script_tex };

                                                        let tab_rect_id = ui.make_persistent_id(("tab_rect", open_entity));
                                                        let last_rect = ui.data_mut(|d| d.get_temp::<egui::Rect>(tab_rect_id));

                                                        let is_hovered = if let Some(rect) = last_rect {
                                                            ui.rect_contains_pointer(rect)
                                                        } else {
                                                            false
                                                        };

                                                        let fill_color = if is_active {
                                                            active_tab_color
                                                        } else if is_hovered {
                                                            egui::Color32::from_rgb(225, 226, 227)
                                                        } else {
                                                            tab_bar_color
                                                        };

                                                        let frame_res = egui::Frame::none()
                                                            .fill(fill_color)
                                                            .stroke(if is_active {
                                                                egui::Stroke::new(1.0, border_color)
                                                            } else {
                                                                egui::Stroke::NONE
                                                            })
                                                            .corner_radius(if is_active {
                                                                egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }
                                                            } else if is_hovered {
                                                                egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }
                                                            } else {
                                                                egui::CornerRadius { nw: 0, ne: 0, sw: 0, se: 0 }
                                                            })
                                                            .inner_margin(egui::Margin { left: 10, right: 6, top: 6, bottom: 6 })
                                                            .show(ui, |ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);

                                                                    let tab_click = ui.horizontal(|ui| {
                                                                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                                                                        ui.add(egui::Image::new((tab_icon, egui::vec2(14.0, 14.0))));
                                                                        let label = ui.add(egui::Label::new(
                                                                            egui::RichText::new(&open_script_name)
                                                                                .strong()
                                                                                .size(13.0)
                                                                                .color(if is_active { egui::Color32::BLACK } else { egui::Color32::from_rgb(120, 120, 120) })
                                                                        ).sense(egui::Sense::click()));
                                                                        label.clicked()
                                                                    }).inner;

                                                                    if tab_click {
                                                                        ui_state.active_editor.entity = Some(open_entity);
                                                                    }

                                                                    let close_btn = ui.add(
                                                                        egui::Button::new(
                                                                            egui::RichText::new("x")
                                                                                .size(14.0)
                                                                                .color(egui::Color32::from_rgb(140, 140, 140))
                                                                        )
                                                                        .frame(false)
                                                                    );
                                                                    if close_btn.clicked() {
                                                                        should_close_tab = Some(open_entity);
                                                                    }
                                                                });
                                                            });

                                                        let tab_rect = frame_res.response.rect;
                                                        ui.data_mut(|d| d.insert_temp(tab_rect_id, tab_rect));

                                                        if is_active {
                                                            active_tab_rect = Some(tab_rect);

                                                            let erase_stroke = egui::Stroke::new(1.5, active_tab_color);
                                                            ui.painter().line_segment(
                                                                [egui::pos2(tab_rect.min.x + 1.0, tab_rect.max.y - 0.5), egui::pos2(tab_rect.max.x - 1.0, tab_rect.max.y - 0.5)],
                                                                erase_stroke,
                                                            );
                                                        }
                                                        ui.add_space(2.0);
                                                    }
                                                });

                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    ui.add_space(8.0);
                                                    if ui.button("Close Editor").clicked() {
                                                        ui_state.active_editor.entity = None;
                                                        ui_state.active_editor.open_entities.clear();
                                                    }
                                                    if ui.button("Save").clicked() {
                                                        should_save = true;
                                                    }
                                                });
                                            });
                                        });

                                    let bar_rect = bar_res.response.rect;
                                    let line_y = bar_rect.max.y;
                                    let line_stroke = egui::Stroke::new(1.0, border_color);

                                    if let Some(tab_rect) = active_tab_rect {
                                        if tab_rect.min.x > bar_rect.min.x {
                                            ui.painter().line_segment(
                                                [egui::pos2(bar_rect.min.x, line_y), egui::pos2(tab_rect.min.x, line_y)],
                                                line_stroke,
                                            );
                                        }
                                        if bar_rect.max.x > tab_rect.max.x {
                                            ui.painter().line_segment(
                                                [egui::pos2(tab_rect.max.x, line_y), egui::pos2(bar_rect.max.x, line_y)],
                                                line_stroke,
                                            );
                                        }
                                    } else {
                                        ui.painter().line_segment(
                                            [egui::pos2(bar_rect.min.x, line_y), egui::pos2(bar_rect.max.x, line_y)],
                                            line_stroke,
                                        );
                                    }

                                    ui.add_space(4.0);

                                    if let Some(err) = &compile_error {
                                        egui::Frame::none()
                                            .fill(egui::Color32::from_rgb(253, 236, 236))
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(245, 186, 186)))
                                            .inner_margin(egui::Margin::symmetric(16, 8))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new("Error:").strong().color(egui::Color32::from_rgb(190, 60, 60)));
                                                    ui.label(egui::RichText::new(err)
                                                        .color(egui::Color32::from_rgb(190, 60, 60))
                                                        .strong()
                                                        .size(13.0)
                                                    );
                                                });
                                            });
                                        ui.add_space(4.0);
                                    }

                                    let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
                                    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                                        let mut layout_job = egui_extras::syntax_highlighting::highlight(
                                            ui.ctx(),
                                            ui.style(),
                                            &theme,
                                            buf.as_str(),
                                            "lua",
                                        );
                                        layout_job.wrap.max_width = wrap_width;
                                        ui.fonts_mut(|f| f.layout_job(layout_job))
                                    };

                                    egui::ScrollArea::both()
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            egui::Frame::none()
                                                .inner_margin(egui::Margin::symmetric(16, 12))
                                                .show(ui, |ui| {
                                                    ui.horizontal_top(|ui| {
                                                        let total_lines = current_source.split('\n').count();
                                                        let line_numbers_text = line_numbers(&mut state.4, total_lines);

                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(line_numbers_text)
                                                                    .font(egui::FontId::new(14.0, egui::FontFamily::Monospace))
                                                                    .color(egui::Color32::from_rgb(140, 140, 140))
                                                            )
                                                        );

                                                        ui.add_space(8.0);

                                                        let editor = egui::TextEdit::multiline(&mut current_source)
                                                            .id_source(active_entity)
                                                            .font(egui::FontId::new(14.0, egui::FontFamily::Monospace))
                                                            .code_editor()
                                                            .frame(egui::Frame::none())
                                                            .desired_width(f32::INFINITY)
                                                            .layouter(&mut layouter);
                                                        ui.add(editor);
                                                    });
                                                });
                                        });

                                    let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                                    if ctrl && ui.input(|i| i.key_pressed(egui::Key::W)) {
                                        should_close_tab = Some(active_entity);
                                    }
                                    if ctrl && ui.input(|i| i.key_pressed(egui::Key::S)) {
                                        should_save = true;
                                    }
                                });
                            });
                    });

                ctx.data_mut(|d| d.insert_temp(last_change_id, state));
                script_editor_rect = Some(panel_res.response.rect);

                if let Some(entity_to_close) = should_close_tab {
                    ui_state.active_editor.open_entities.retain(|&e| e != entity_to_close);
                    if ui_state.active_editor.entity == Some(entity_to_close) {
                        ui_state.active_editor.entity = ui_state.active_editor.open_entities.last().copied();
                    }
                }

                let mut source_changed = false;
                if let Ok((_, _, _, _, _, server_opt, local_opt, module_opt)) = queries.explorer_query.get(active_entity) {
                    if let Some(ref script) = server_opt {
                        if script.code != current_source { source_changed = true; }
                    } else if let Some(ref script) = local_opt {
                        if script.code != current_source { source_changed = true; }
                    } else if let Some(ref script) = module_opt {
                        if script.code != current_source { source_changed = true; }
                    }
                }

                if source_changed {
                    if let Ok((_, _, _, _, _, server_opt, local_opt, module_opt)) = queries.explorer_query.get(active_entity) {
                        if let Ok(mut e_cmd) = ui_res.commands.get_entity(active_entity) {
                            if let Some(server_script) = server_opt {
                                e_cmd.insert(crate::scripting::ecs::ServerScript {
                                    code: current_source.clone(),
                                    enabled: server_script.enabled,
                                    started: false,
                                });
                            } else if let Some(local_script) = local_opt {
                                e_cmd.insert(crate::scripting::ecs::LocalScript {
                                    code: current_source.clone(),
                                    enabled: local_script.enabled,
                                    started: false,
                                });
                            } else if module_opt.is_some() {
                                e_cmd.insert(crate::scripting::ecs::ModuleScript {
                                    code: current_source.clone(),
                                });
                            }
                        }
                    }
                }
            } else {
                ui_state.active_editor.open_entities.retain(|&e| e != active_entity);
                ui_state.active_editor.entity = ui_state.active_editor.open_entities.first().copied();
            }
        }
    }

    let mut is_hovering_ui = false;
    if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
        if top_bar_res.response.rect.contains(pos) {
            is_hovering_ui = true;
        }
        if panel_res.response.rect.contains(pos) {
            is_hovering_ui = true;
        }
        if let Some(rect) = script_editor_rect {
            if rect.contains(pos) {
                is_hovering_ui = true;
            }
        }
        if ctx.is_pointer_over_area() {
            is_hovering_ui = true;
        }
    }
    ui_state.hover_state.is_hovering_ui = is_hovering_ui;

    if *ui_state.onboarding_state.get() != crate::studio::tools::OnboardingState::Inactive {
        panels::draw_onboarding(
            ctx,
            &mut ui_state.next_onboarding_state,
            &ui_state.onboarding_state,
            &mut ui_state.onboarding_data,
            &mut ui_res.commands,
            &mut ui_res.meshes,
            &mut ui_res.studs_materials,
            &ui_res.studs_assets,
            &mut ui_res.count,
            thumb_empty_tex,
            thumb_baseplate_tex,
            &ui_state.file_dialog_state,
        );

        ui_state.hover_state.is_hovering_ui = true;
    }

    if ui_state.marquee_state.active {
        egui::Area::new(egui::Id::new("marquee_overlay"))
            .interactable(false)
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                if let (Some(start), Some(end)) = (ui_state.marquee_state.start_pos, ui_state.marquee_state.current_pos) {
                    let rect = egui::Rect::from_two_pos(
                        egui::pos2(start.x, start.y),
                        egui::pos2(end.x, end.y),
                    );
                    ui.painter().rect_filled(
                        rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(80, 160, 240, 30),
                    );
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 160, 240)),
                        egui::StrokeKind::Inside,
                    );
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::line_numbers;

    #[test]
    fn reuses_line_numbers_for_the_same_count() {
        let mut cache = None;
        let first_ptr = line_numbers(&mut cache, 12).as_ptr();
        let second_ptr = line_numbers(&mut cache, 12).as_ptr();

        assert_eq!(first_ptr, second_ptr);
        assert_eq!(cache.as_ref().unwrap().1, " 1\n 2\n 3\n 4\n 5\n 6\n 7\n 8\n 9\n10\n11\n12\n");
    }

    #[test]
    fn rebuilds_line_numbers_when_the_count_changes() {
        let mut cache = None;
        line_numbers(&mut cache, 2);
        let text = line_numbers(&mut cache, 3);

        assert_eq!(text, "1\n2\n3\n");
        assert_eq!(cache.as_ref().unwrap().0, 3);
    }
}
