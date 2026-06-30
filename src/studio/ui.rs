pub mod assets;
pub mod indicator;
pub mod panels;
pub mod visuals;
pub mod resources;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle};
use crate::studio::tools::ToolState;
use crate::studio::tools::Selection;
use crate::common::components::Brick;
use bevy::ecs::system::SystemParam;
use bevy::pbr::ExtendedMaterial;
//////////
//todo
//make it so you cannot interact with the viewport while hovering over UI !!DONE
//qol updates to general usage of UI, like double clicking a grouped part displays the children of the part
pub use assets::{StudioUiAssets, StudioUiTextureIds, setup_ui_assets};
pub use indicator::{CameraSpeedIndicator, updatecameraspeedindicator, FovIndicator, update_camera_fov};
pub use visuals::configure_visuals;
pub use resources::{CopiedEntityBuffer, HierarchyDraggedEntity};

#[derive(SystemParam)]
pub struct UiResources<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub studs_materials: ResMut<'w, Assets<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>>,
    pub studs_assets: Res<'w, crate::studio::studs::StudsAssets>,
    pub count: ResMut<'w, crate::studio::bricks::BrickSpawnerCount>,
    pub snap_config: ResMut<'w, crate::studio::tools::SnapConfig>,
    pub history: ResMut<'w, crate::studio::tools::UndoRedoHistory>,
    pub action_writer: MessageWriter<'w, crate::studio::tools::UndoRedoAction>,
    pub physics_state: Res<'w, crate::common::physics::PhysicsSimulationState>,
    pub physics_action_writer: MessageWriter<'w, crate::common::physics::PhysicsSimulationAction>,
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
    pub camera_transform_query: Query<'w, 's, &'static Transform, With<Camera3d>>,
    pub entities_query: Query<
        'w,
        's,
        (
            Entity,
            &'static mut Transform,
            &'static mut Name,
            Option<&'static ChildOf>,
            Option<&'static Children>,
            Option<&'static Brick>,
            &'static GlobalTransform,
            Option<&'static Mesh3d>,
            Option<&'static MeshMaterial3d<StandardMaterial>>,
            Option<&'static MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>>,
        ),
        Without<Camera3d>,
    >,
}

#[allow(deprecated)]
pub fn studio_ui(
    mut contexts: EguiContexts,
    mut ui_res: UiResources,
    mut ui_state: UiStateResources<'_>,
    mut queries: UiQueries<'_, '_>,
) {
    let Some(assets) = &ui_state.ui_assets else { return; };

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

    let Ok(ctx) = contexts.ctx_mut() else { return; };
    ctx.set_visuals(egui::Visuals::light());

    let frame = egui::Frame::NONE
        .fill(egui::Color32::from_rgb(245, 246, 247))
        .inner_margin(egui::Margin::same(0));

    let camera_transform = queries.camera_transform_query.iter().next();

    let top_bar_res = egui::Panel::top("topbar")
        .frame(frame)
        .show(ctx, |ui| {
            panels::draw_top_bar(
                ui,
                &mut ui_state.next_tool,
                &ui_state.current_tool,
                &mut ui_res.commands,
                &mut ui_res.meshes,
                &mut ui_res.studs_materials,
                &ui_res.studs_assets,
                &mut ui_res.count,
                &mut ui_res.snap_config,
                move_tex,
                rotate_tex,
                scale_tex,
                add_tex,
                &ui_state.diagnostics,
                camera_transform,
                &mut ui_res.action_writer,
                &mut ui_res.history,
                *ui_res.physics_state,
                &mut ui_res.physics_action_writer,
            );
        });

    let panel_res = egui::SidePanel::left("explorer")
        .frame(egui::Frame::none()
            .fill(egui::Color32::from_rgb(245, 246, 247))
            .inner_margin(egui::Margin::symmetric(12, 12))
        )
        .default_width(220.0)
        .show(ctx, |ui| {
            let total_height = ui.available_height();

            let mut selected_brick = None;
            if let Some(selected_entity) = ui_state.selection.entity {
                if let Ok((_, _, _, _, _, Some(_), _, _, _, _)) = queries.entities_query.get(selected_entity) {
                    selected_brick = Some(selected_entity);
                }
            }

            let has_selection = selected_brick.is_some();
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
                                &queries.entities_query,
                                &mut ui_state.copiedbuffer,
                                &mut ui_state.dragged_entity,
                                &mut ui_res.history,
                            );
                        });
                }
            );

            if let Some(brick_entity) = selected_brick {
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
                            brick_entity,
                            &mut queries.entities_query,
                            &mut ui_res.materials,
                            &mut ui_res.studs_materials,
                        );
                    });
            }
        });

    if let Some(dragged) = ui_state.dragged_entity.entity {
        if panel_res.response.hovered() && ctx.input(|i| i.pointer.any_released()) {
            if let Ok((_, _, _, child_of_opt, _, _, child_global, _, _, _)) = queries.entities_query.get(dragged) {
                let old_parent = child_of_opt.map(|co| co.parent());
                let old_transform = queries.entities_query.get(dragged).ok().map(|(_, t, _, _, _, _, _, _, _, _)| *t).unwrap_or(Transform::IDENTITY);

                let new_transform = Transform {
                    translation: child_global.translation(),
                    rotation: child_global.rotation(),
                    scale: child_global.scale(),
                };

                ui_res.commands.entity(dragged).insert(new_transform);
                ui_res.commands.entity(dragged).remove::<ChildOf>();

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

    indicator::draw_indicator(ctx, &mut ui_state.cameraindicator, &mut queries.cameraquery);
    indicator::draw_fov_indicator(ctx, &mut ui_state.fovindicator, &mut queries.camera_projection_query);

    if let (Some(entity), Some(pos)) = (ui_state.context_menu.entity, ui_state.context_menu.position) {
        let mut close_menu = false;

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
            close_menu = true;
        }

        if !ui_state.context_menu.just_opened && ctx.input(|i| i.pointer.any_pressed()) {
            if let Some(mouse_pos) = ctx.pointer_interact_pos() {
                if !inner_res.response.rect.contains(mouse_pos) {
                    close_menu = true;
                }
            }
        }

        ui_state.context_menu.just_opened = false;

        if close_menu {
            ui_state.context_menu.entity = None;
            ui_state.context_menu.position = None;
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
        if ctx.is_pointer_over_area() {
            is_hovering_ui = true;
        }
    }
    ui_state.hover_state.is_hovering_ui = is_hovering_ui;
}