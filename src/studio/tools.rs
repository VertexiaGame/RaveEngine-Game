use bevy::prelude::*;
use bevy::window::{CursorIcon, SystemCursorIcon};
use crate::common::components::Brick;
use crate::studio::gizmos::ToolGizmo;

#[derive(Default, States, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ToolState {
    #[default]
    None,
    Move,
    Size,
    Rotate,
}

#[derive(Resource, Default)]
pub struct Selection {
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct DragState {
    pub active: bool,
    pub gizmo_entity: Option<Entity>,
    pub start_translation: Option<Vec3>,
    pub start_scale: Option<Vec3>,
    pub accumulated_displacement: f32,
}

#[derive(Resource, Default)]
pub struct HoverState {
    pub hovered_gizmo: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct CanvasContextMenu {
    pub entity: Option<Entity>,
    pub position: Option<Vec2>,
    pub just_opened: bool,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct SnapConfig {
    pub enabled: bool,
    pub distance: f32,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            distance: 1.0,
        }
    }
}

fn world_to_local(
    world_translation: Vec3,
    world_rotation: Quat,
    world_scale: Vec3,
    parent_global: Option<&GlobalTransform>,
) -> (Vec3, Quat, Vec3) {
    if let Some(parent) = parent_global {
        let parent_scale = parent.scale();
        let parent_rotation = parent.rotation();
        let parent_translation = parent.translation();

        let local_scale = Vec3::new(
            if parent_scale.x != 0.0 { world_scale.x / parent_scale.x } else { world_scale.x },
            if parent_scale.y != 0.0 { world_scale.y / parent_scale.y } else { world_scale.y },
            if parent_scale.z != 0.0 { world_scale.z / parent_scale.z } else { world_scale.z },
        );
        let local_rotation = parent_rotation.inverse() * world_rotation;
        let unscaled_translation = parent_rotation.inverse().mul_vec3(world_translation - parent_translation);
        let local_translation = Vec3::new(
            if parent_scale.x != 0.0 { unscaled_translation.x / parent_scale.x } else { unscaled_translation.x },
            if parent_scale.y != 0.0 { unscaled_translation.y / parent_scale.y } else { unscaled_translation.y },
            if parent_scale.z != 0.0 { unscaled_translation.z / parent_scale.z } else { unscaled_translation.z },
        );
        (local_translation, local_rotation, local_scale)
    } else {
        (world_translation, world_rotation, world_scale)
    }
}

pub fn select_brick(
    mut clicks: MessageReader<Pointer<Click>>,
    bricks: Query<Entity, With<Brick>>,
    gizmos: Query<Entity, With<ToolGizmo>>,
    mut selection: ResMut<Selection>,
    mut context_menu: ResMut<CanvasContextMenu>,
    mut contexts: bevy_egui::EguiContexts,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.egui_wants_pointer_input() || ctx.egui_wants_keyboard_input() {
            return;
        }
    }

    for click in clicks.read() {
        let target = click.event_target();
        if click.button == PointerButton::Primary {
            if bricks.get(target).is_ok() {
                selection.entity = Some(target);
                context_menu.entity = None;
                context_menu.position = None;
            } else if gizmos.get(target).is_err() {
                selection.entity = None;
                context_menu.entity = None;
                context_menu.position = None;
            }
        }
    }
}

pub fn handle_drag_start(
    mut drags: MessageReader<Pointer<DragStart>>,
    gizmos: Query<&ToolGizmo>,
    mut drag_state: ResMut<DragState>,
) {
    for drag in drags.read() {
        let target = drag.event_target();
        if gizmos.get(target).is_ok() {
            drag_state.active = true;
            drag_state.gizmo_entity = Some(target);
        }
    }
}

pub fn handle_drag(
    mut drags: MessageReader<Pointer<Drag>>,
    gizmos: Query<&ToolGizmo>,
    mut bricks: Query<(&mut Transform, &GlobalTransform, Option<&ChildOf>), With<Brick>>,
    parent_global_query: Query<&GlobalTransform>,
    mut drag_state: ResMut<DragState>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    snap_config: Res<SnapConfig>,
) {
    if !drag_state.active { return; }
    
    let Some(gizmo_entity) = drag_state.gizmo_entity else { return };
    let Ok(gizmo) = gizmos.get(gizmo_entity) else { return };
    let Ok((mut brick_transform, brick_global, child_of_opt)) = bricks.get_mut(gizmo.target) else { return };
    let Some((camera, camera_transform)) = camera_query.iter().next() else { return };
    let Ok(window) = windows.single() else { return };

    let parent_global = child_of_opt.and_then(|co| parent_global_query.get(co.parent()).ok());

    let start_translation = *drag_state.start_translation.get_or_insert(brick_global.translation());
    let start_scale = *drag_state.start_scale.get_or_insert(brick_global.scale());

    for drag in drags.read() {
        let delta = drag.delta;
        let center_world = brick_global.translation();

        if gizmo.tool == ToolState::Rotate {
            let axis_world = brick_global.rotation().mul_vec3(gizmo.axis);

            if let Ok(center_screen) = camera.world_to_viewport(camera_transform, center_world) {
                let cursor_pos = window.cursor_position().unwrap_or(center_screen + Vec2::new(100.0, 0.0));
                let to_cursor = cursor_pos - center_screen;
                let tangent = if to_cursor.length_squared() > 1.0 {
                    Vec2::new(-to_cursor.y, to_cursor.x).normalize()
                } else {
                    Vec2::new(1.0, 0.0)
                };

                let drag_amount = delta.dot(tangent);

                let to_camera = camera_transform.translation() - center_world;
                let alignment = axis_world.dot(to_camera);
                let sign = if alignment >= 0.0 { 1.0 } else { -1.0 };

                let rotation_speed = 0.01;
                let angle = -drag_amount * rotation_speed * sign;

                let rot = Quat::from_axis_angle(gizmo.axis, angle);
                brick_transform.rotate_local(rot);
            }
        } else {
            let axis_world = brick_global.rotation().mul_vec3(gizmo.axis);
            let tip_world = center_world + axis_world;

            if let (Ok(c), Ok(t)) = (
                camera.world_to_viewport(camera_transform, center_world),
                camera.world_to_viewport(camera_transform, tip_world)
            ) {
                let screen_vec = t - c;
                let pixel_len = screen_vec.length();
                let screen_dir = screen_vec.normalize_or_zero();
                
                let mut amount_world = 0.0;
                if pixel_len > 0.0 {
                    amount_world = delta.dot(screen_dir) / pixel_len;
                }

                drag_state.accumulated_displacement += amount_world;

                let mut snapped_displacement = drag_state.accumulated_displacement;
                if snap_config.enabled && snap_config.distance > 0.0 {
                    let snap_interval = snap_config.distance * 0.28;
                    snapped_displacement = (drag_state.accumulated_displacement / snap_interval).round() * snap_interval;
                }

                match gizmo.tool {
                    ToolState::Move => {
                        let new_global_translation = start_translation + axis_world * snapped_displacement;
                        let (local_translation, _local_rotation, _local_scale) = world_to_local(
                            new_global_translation,
                            brick_global.rotation(),
                            brick_global.scale(),
                            parent_global,
                        );
                        brick_transform.translation = local_translation;
                    }
                    ToolState::Size => {
                        let axis_abs = gizmo.axis.abs();
                        let base_extents = Vec3::new(2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28);
                        let base_dimension = axis_abs * base_extents * 2.0;
                        let base_dim_scalar = base_dimension.length();

                        let total_delta_scale = if base_dim_scalar > 0.0 {
                            snapped_displacement / base_dim_scalar
                        } else {
                            0.0
                        };

                        let new_global_scale = (start_scale + axis_abs * total_delta_scale).max(Vec3::splat(0.1));
                        let actual_delta_scale = new_global_scale - start_scale;
                        
                        let translation_delta = gizmo.axis * actual_delta_scale * base_extents;
                        let final_translation_delta = brick_global.rotation().mul_vec3(translation_delta);
                        let new_global_translation = start_translation + final_translation_delta;

                        let (local_translation, _local_rotation, local_scale) = world_to_local(
                            new_global_translation,
                            brick_global.rotation(),
                            new_global_scale,
                            parent_global,
                        );
                        brick_transform.scale = local_scale;
                        brick_transform.translation = local_translation;
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn handle_drag_end(
    mut drags: MessageReader<Pointer<DragEnd>>,
    mut drag_state: ResMut<DragState>,
) {
    for _ in drags.read() {
        drag_state.active = false;
        drag_state.gizmo_entity = None;
        drag_state.start_translation = None;
        drag_state.start_scale = None;
        drag_state.accumulated_displacement = 0.0;
    }
}

pub fn handle_hover(
    mut overs: MessageReader<Pointer<Over>>,
    mut outs: MessageReader<Pointer<Out>>,
    gizmos: Query<&ToolGizmo>,
    mut hover_state: ResMut<HoverState>,
) {
    for over in overs.read() {
        let target = over.event_target();
        if gizmos.get(target).is_ok() {
            hover_state.hovered_gizmo = Some(target);
        }
    }
    for out in outs.read() {
        let target = out.event_target();
        if Some(target) == hover_state.hovered_gizmo {
            hover_state.hovered_gizmo = None;
        }
    }
}

pub fn update_cursor(
    mut commands: Commands,
    drag_state: Res<DragState>,
    hover_state: Res<HoverState>,
    windows: Query<Entity, With<Window>>,
) {
    let Ok(window_entity) = windows.single() else { return };
    if drag_state.active {
        commands.entity(window_entity).insert(CursorIcon::from(SystemCursorIcon::Grabbing));
    } else if hover_state.hovered_gizmo.is_some() {
        commands.entity(window_entity).insert(CursorIcon::from(SystemCursorIcon::Grab));
    } else {
        commands.entity(window_entity).remove::<CursorIcon>();
    }
}