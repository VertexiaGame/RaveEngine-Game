use bevy::prelude::*;
use bevy::window::{CursorIcon, SystemCursorIcon};
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use crate::common::components::Brick;
use crate::studio::gizmos::ToolGizmo;
use bevy::pbr::ExtendedMaterial;

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
    pub start_transform: Option<Transform>,
    pub accumulated_displacement: f32,
}

#[derive(Resource, Default)]
pub struct PartDragState {
    pub active: bool,
    pub dragged_entity: Option<Entity>,
    pub start_transform: Option<Transform>,
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

#[derive(Clone, Debug)]
pub struct BrickData {
    pub transform: Transform,
    pub name: String,
    pub is_brick: bool,
    pub mesh: Option<Mesh3d>,
    pub standard_material: Option<MeshMaterial3d<StandardMaterial>>,
    pub studs_material: Option<MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>>,
    pub parent: Option<Entity>,
}

impl BrickData {
    pub fn remap(&mut self, old: Entity, new: Entity) {
        if let Some(p) = &mut self.parent {
            if *p == old {
                *p = new;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum UndoCommand {
    TransformChange {
        entity: Entity,
        old_transform: Transform,
        new_transform: Transform,
    },
    Spawn {
        entity: Entity,
        data: BrickData,
    },
    Delete {
        entity: Entity,
        data: BrickData,
    },
    ParentChange {
        entity: Entity,
        old_parent: Option<Entity>,
        new_parent: Option<Entity>,
        old_transform: Transform,
        new_transform: Transform,
    },
}

impl UndoCommand {
    pub fn remap(&mut self, old: Entity, new: Entity) {
        match self {
            UndoCommand::TransformChange { entity, .. } => {
                if *entity == old {
                    *entity = new;
                }
            }
            UndoCommand::Spawn { entity, data } => {
                if *entity == old {
                    *entity = new;
                }
                data.remap(old, new);
            }
            UndoCommand::Delete { entity, data } => {
                if *entity == old {
                    *entity = new;
                }
                data.remap(old, new);
            }
            UndoCommand::ParentChange { entity, old_parent, new_parent, .. } => {
                if *entity == old {
                    *entity = new;
                }
                if let Some(p) = old_parent {
                    if *p == old {
                        *p = new;
                    }
                }
                if let Some(p) = new_parent {
                    if *p == old {
                        *p = new;
                    }
                }
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct UndoRedoHistory {
    pub undo_stack: Vec<UndoCommand>,
    pub redo_stack: Vec<UndoCommand>,
}

impl UndoRedoHistory {
    pub fn push_command(&mut self, command: UndoCommand) {
        self.undo_stack.push(command);
        self.redo_stack.clear();
    }

    pub fn remap_entity(&mut self, old: Entity, new: Entity) {
        for cmd in &mut self.undo_stack {
            cmd.remap(old, new);
        }
        for cmd in &mut self.redo_stack {
            cmd.remap(old, new);
        }
    }
}

#[derive(Message, Clone, Copy, Debug)]
pub enum UndoRedoAction {
    Undo,
    Redo,
}

pub fn spawn_from_data(
    commands: &mut Commands,
    data: &BrickData,
) -> Entity {
    let mut spawned = commands.spawn((
        data.transform,
        Name::new(data.name.clone()),
        Pickable::default(),
    ));
    if data.is_brick {
        spawned.insert(Brick);
    }
    if let Some(ref m) = data.mesh {
        spawned.insert(m.clone());
    }
    if let Some(ref mat) = data.standard_material {
        spawned.insert(mat.clone());
    }
    if let Some(ref studs_mat) = data.studs_material {
        spawned.insert(studs_mat.clone());
    }
    let new_entity = spawned.id();
    if let Some(parent) = data.parent {
        commands.entity(parent).add_child(new_entity);
    }
    new_entity
}

pub fn capture_brick_data(
    entity: Entity,
    query: &Query<(
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
) -> Option<BrickData> {
    if let Ok((_, transform, name, child_of_opt, _, brick_opt, _, mesh_opt, mat_opt, studs_mat_opt)) = query.get(entity) {
        Some(BrickData {
            transform: *transform,
            name: name.to_string(),
            is_brick: brick_opt.is_some(),
            mesh: mesh_opt.cloned(),
            standard_material: mat_opt.cloned(),
            studs_material: studs_mat_opt.cloned(),
            parent: child_of_opt.map(|co| co.parent()),
        })
    } else {
        None
    }
}

pub fn handle_keyboard_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut action_writer: MessageWriter<UndoRedoAction>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    if ctrl {
        if keys.just_pressed(KeyCode::KeyZ) {
            if shift {
                action_writer.write(UndoRedoAction::Redo);
            } else {
                action_writer.write(UndoRedoAction::Undo);
            }
        } else if keys.just_pressed(KeyCode::KeyY) {
            action_writer.write(UndoRedoAction::Redo);
        }
    }
}

pub fn handle_undo_redo_action(
    mut actions: MessageReader<UndoRedoAction>,
    mut history: ResMut<UndoRedoHistory>,
    mut commands: Commands,
    mut selection: ResMut<Selection>,
    mut query: Query<(
        Entity,
        &mut Transform,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>>,
    )>,
) {
    for action in actions.read() {
        match *action {
            UndoRedoAction::Undo => {
                if let Some(command) = history.undo_stack.pop() {
                    match command.clone() {
                        UndoCommand::TransformChange { entity, old_transform, new_transform: _ } => {
                            if let Ok((_, mut transform, _, _, _, _, _, _, _)) = query.get_mut(entity) {
                                *transform = old_transform;
                            }
                            history.redo_stack.push(command);
                        }
                        UndoCommand::Spawn { entity, data: _ } => {
                            commands.entity(entity).despawn();
                            if selection.entity == Some(entity) {
                                selection.entity = None;
                            }
                            history.redo_stack.push(command);
                        }
                        UndoCommand::Delete { entity, data } => {
                            let new_entity = spawn_from_data(&mut commands, &data);
                            history.remap_entity(entity, new_entity);
                            if selection.entity == Some(entity) {
                                selection.entity = Some(new_entity);
                            }
                            let updated_command = UndoCommand::Delete { entity: new_entity, data };
                            history.redo_stack.push(updated_command);
                        }
                        UndoCommand::ParentChange { entity, old_parent, new_parent: _, old_transform, new_transform: _ } => {
                            if let Ok((_, mut transform, _, _, _, _, _, _, _)) = query.get_mut(entity) {
                                *transform = old_transform;
                            }
                            if let Some(parent) = old_parent {
                                commands.entity(parent).add_child(entity);
                            } else {
                                commands.entity(entity).remove::<ChildOf>();
                            }
                            history.redo_stack.push(command);
                        }
                    }
                }
            }
            UndoRedoAction::Redo => {
                if let Some(command) = history.redo_stack.pop() {
                    match command.clone() {
                        UndoCommand::TransformChange { entity, old_transform: _, new_transform } => {
                            if let Ok((_, mut transform, _, _, _, _, _, _, _)) = query.get_mut(entity) {
                                *transform = new_transform;
                            }
                            history.undo_stack.push(command);
                        }
                        UndoCommand::Spawn { entity, data } => {
                            let new_entity = spawn_from_data(&mut commands, &data);
                            history.remap_entity(entity, new_entity);
                            if selection.entity == Some(entity) {
                                selection.entity = Some(new_entity);
                            }
                            let updated_command = UndoCommand::Spawn { entity: new_entity, data };
                            history.undo_stack.push(updated_command);
                        }
                        UndoCommand::Delete { entity, data: _ } => {
                            commands.entity(entity).despawn();
                            if selection.entity == Some(entity) {
                                selection.entity = None;
                            }
                            history.undo_stack.push(command);
                        }
                        UndoCommand::ParentChange { entity, old_parent: _, new_parent, old_transform: _, new_transform } => {
                            if let Ok((_, mut transform, _, _, _, _, _, _, _)) = query.get_mut(entity) {
                                *transform = new_transform;
                            }
                            if let Some(parent) = new_parent {
                                commands.entity(parent).add_child(entity);
                            } else {
                                commands.entity(entity).remove::<ChildOf>();
                            }
                            history.undo_stack.push(command);
                        }
                    }
                }
            }
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
        let parent_rotation = parent.rotation();
        let parent_translation = parent.translation();

        let local_scale = world_scale;
        let local_rotation = parent_rotation.inverse() * world_rotation;
        let local_translation = parent_rotation.inverse().mul_vec3(world_translation - parent_translation);
        (local_translation, local_rotation, local_scale)
    } else {
        (world_translation, world_rotation, world_scale)
    }
}

fn compute_rotation_drag(
    delta: Vec2,
    center_world: Vec3,
    axis_world: Vec3,
    gizmo_axis: Vec3,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window: &Window,
) -> Option<Quat> {
    let center_screen = camera.world_to_viewport(camera_transform, center_world).ok()?;
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

    Some(Quat::from_axis_angle(gizmo_axis, angle))
}

fn compute_move_delta(
    delta: Vec2,
    center_world: Vec3,
    axis_world: Vec3,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<f32> {
    let tip_world = center_world + axis_world;
    let c = camera.world_to_viewport(camera_transform, center_world).ok()?;
    let t = camera.world_to_viewport(camera_transform, tip_world).ok()?;
    let screen_vec = t - c;
    let pixel_len = screen_vec.length();
    let screen_dir = screen_vec.normalize_or_zero();

    if pixel_len > 0.0 {
        Some(delta.dot(screen_dir) / pixel_len)
    } else {
        Some(0.0)
    }
}

fn apply_snap(
    value: f32,
    snap_config: &SnapConfig,
) -> f32 {
    if snap_config.enabled && snap_config.distance > 0.0 {
        let snap_interval = snap_config.distance * 0.28;
        (value / snap_interval).round() * snap_interval
    } else {
        value
    }
}

fn compute_resize(
    gizmo_axis: Vec3,
    snapped_displacement: f32,
    start_scale: Vec3,
    start_translation: Vec3,
    brick_rotation: Quat,
    parent_global: Option<&GlobalTransform>,
) -> (Vec3, Vec3) {
    let axis_abs = gizmo_axis.abs();
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

    let translation_delta = gizmo_axis * actual_delta_scale * base_extents;
    let final_translation_delta = brick_rotation.mul_vec3(translation_delta);
    let new_global_translation = start_translation + final_translation_delta;

    let (local_translation, _local_rotation, local_scale) = world_to_local(
        new_global_translation,
        brick_rotation,
        new_global_scale,
        parent_global,
    );
    (local_translation, local_scale)
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
    bricks: Query<&Transform, With<Brick>>,
) {
    for drag in drags.read() {
        let target = drag.event_target();
        if let Ok(gizmo) = gizmos.get(target) {
            drag_state.active = true;
            drag_state.gizmo_entity = Some(target);
            if let Ok(transform) = bricks.get(gizmo.target) {
                drag_state.start_transform = Some(*transform);
            }
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
    physics_state: Res<crate::common::physics::PhysicsSimulationState>,
) {
    if *physics_state == crate::common::physics::PhysicsSimulationState::Running { return; }
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
        let axis_world = brick_global.rotation().mul_vec3(gizmo.axis);

        if gizmo.tool == ToolState::Rotate {
            if let Some(rot) = compute_rotation_drag(
                delta,
                center_world,
                axis_world,
                gizmo.axis,
                camera,
                camera_transform,
                window,
            ) {
                brick_transform.rotate_local(rot);
            }
        } else {
            if let Some(amount_world) = compute_move_delta(
                delta,
                center_world,
                axis_world,
                camera,
                camera_transform,
            ) {
                drag_state.accumulated_displacement += amount_world;

                let snapped_displacement = apply_snap(drag_state.accumulated_displacement, &snap_config);

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
                        let (local_translation, local_scale) = compute_resize(
                            gizmo.axis,
                            snapped_displacement,
                            start_scale,
                            start_translation,
                            brick_global.rotation(),
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
    gizmos: Query<&ToolGizmo>,
    bricks: Query<&Transform, With<Brick>>,
    mut drag_state: ResMut<DragState>,
    mut history: ResMut<UndoRedoHistory>,
) {
    for _ in drags.read() {
        if let (Some(gizmo_entity), Some(start_transform)) = (drag_state.gizmo_entity, drag_state.start_transform) {
            if let Ok(gizmo) = gizmos.get(gizmo_entity) {
                if let Ok(final_transform) = bricks.get(gizmo.target) {
                    if start_transform != *final_transform {
                        history.push_command(UndoCommand::TransformChange {
                            entity: gizmo.target,
                            old_transform: start_transform,
                            new_transform: *final_transform,
                        });
                    }
                }
            }
        }
        drag_state.active = false;
        drag_state.gizmo_entity = None;
        drag_state.start_translation = None;
        drag_state.start_scale = None;
        drag_state.start_transform = None;
        drag_state.accumulated_displacement = 0.0;
    }
}

pub fn handle_part_drag_start(
    mut drags: MessageReader<Pointer<DragStart>>,
    bricks: Query<&Transform, With<Brick>>,
    gizmos: Query<&ToolGizmo>,
    mut part_drag_state: ResMut<PartDragState>,
) {
    for drag in drags.read() {
        let target = drag.event_target();
        if gizmos.get(target).is_ok() {
            return;
        }
        if let Ok(transform) = bricks.get(target) {
            part_drag_state.active = true;
            part_drag_state.dragged_entity = Some(target);
            part_drag_state.start_transform = Some(*transform);
        }
    }
}

fn is_descendant_of(
    entity: Entity,
    ancestor: Entity,
    parent_query: &Query<&ChildOf>,
) -> bool {
    let mut current = entity;
    while let Ok(child_of) = parent_query.get(current) {
        let parent_entity = child_of.parent();
        if parent_entity == ancestor {
            return true;
        }
        current = parent_entity;
    }
    false
}

pub fn handle_part_drag(
    mut drags: MessageReader<Pointer<Drag>>,
    part_drag_state: Res<PartDragState>,
    mut bricks: Query<(&mut Transform, &GlobalTransform, Option<&ChildOf>), With<Brick>>,
    parent_query: Query<&ChildOf>,
    name_query: Query<&Name>,
    parent_global_query: Query<&GlobalTransform>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut raycast: MeshRayCast,
    snap_config: Res<SnapConfig>,
    physics_state: Res<crate::common::physics::PhysicsSimulationState>,
) {
    if *physics_state == crate::common::physics::PhysicsSimulationState::Running { return; }
    if !part_drag_state.active { return; }
    let Some(dragged_entity) = part_drag_state.dragged_entity else { return };

    let Some((camera, camera_transform)) = camera_query.iter().next() else { return };
    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else { return };

    for _ in drags.read() {}

    let (brick_rotation, brick_scale, child_of_parent) = {
        let Ok((_, brick_global, child_of_opt)) = bricks.get(dragged_entity) else { return };
        (brick_global.rotation(), brick_global.scale(), child_of_opt.map(|co| co.parent()))
    };

    let parent_global = child_of_parent.and_then(|parent_entity| parent_global_query.get(parent_entity).ok());

    let base_extents = Vec3::new(2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28);
    let scaled_half_extents = base_extents * brick_scale;

    let filter_func = |entity: Entity| {
        entity != dragged_entity 
            && !is_descendant_of(entity, dragged_entity, &parent_query)
            && (bricks.contains(entity) || name_query.get(entity).map_or(false, |n| n.as_str() == "Baseplate"))
    };

    let raycast_settings = MeshRayCastSettings {
        filter: &filter_func,
        ..default()
    };

    let hits = raycast.cast_ray(ray, &raycast_settings);

    let mut target_world_translation = if let Some((_hit_entity, hit)) = hits.first() {
        let hit_point = hit.point;
        let hit_normal = hit.normal.normalize();

        let local_x = brick_rotation.mul_vec3(Vec3::X);
        let local_y = brick_rotation.mul_vec3(Vec3::Y);
        let local_z = brick_rotation.mul_vec3(Vec3::Z);

        let proj_x = hit_normal.dot(local_x).abs() * scaled_half_extents.x;
        let proj_y = hit_normal.dot(local_y).abs() * scaled_half_extents.y;
        let proj_z = hit_normal.dot(local_z).abs() * scaled_half_extents.z;

        let total_offset = proj_x + proj_y + proj_z;

        hit_point + hit_normal * total_offset
    } else {
        let plane_y = scaled_half_extents.y;
        if ray.direction.y.abs() > 0.001 {
            let t = (plane_y - ray.origin.y) / ray.direction.y;
            if t > 0.0 {
                ray.origin + ray.direction * t
            } else {
                return;
            }
        } else {
            return;
        }
    };

    if snap_config.enabled && snap_config.distance > 0.0 {
        let snap_interval = snap_config.distance * 0.28;
        target_world_translation.x = (target_world_translation.x / snap_interval).round() * snap_interval;
        target_world_translation.z = (target_world_translation.z / snap_interval).round() * snap_interval;
        target_world_translation.y = (target_world_translation.y / snap_interval).round() * snap_interval;
        if target_world_translation.y < scaled_half_extents.y {
            target_world_translation.y = scaled_half_extents.y;
        }
    }

    if let Ok((mut brick_transform, _, _)) = bricks.get_mut(dragged_entity) {
        let (local_translation, _local_rotation, _local_scale) = world_to_local(
            target_world_translation,
            brick_rotation,
            brick_scale,
            parent_global,
        );
        brick_transform.translation = local_translation;
    }
}

pub fn handle_part_drag_end(
    mut drags: MessageReader<Pointer<DragEnd>>,
    bricks: Query<&Transform, With<Brick>>,
    mut part_drag_state: ResMut<PartDragState>,
    mut history: ResMut<UndoRedoHistory>,
) {
    for _ in drags.read() {
        if let (Some(dragged_entity), Some(start_transform)) = (part_drag_state.dragged_entity, part_drag_state.start_transform) {
            if let Ok(final_transform) = bricks.get(dragged_entity) {
                if start_transform != *final_transform {
                    history.push_command(UndoCommand::TransformChange {
                        entity: dragged_entity,
                        old_transform: start_transform,
                        new_transform: *final_transform,
                    });
                }
            }
        }
        part_drag_state.active = false;
        part_drag_state.dragged_entity = None;
        part_drag_state.start_transform = None;
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
    part_drag_state: Res<PartDragState>,
    hover_state: Res<HoverState>,
    windows: Query<Entity, With<Window>>,
) {
    let Ok(window_entity) = windows.single() else { return };
    if drag_state.active || part_drag_state.active {
        commands.entity(window_entity).insert(CursorIcon::from(SystemCursorIcon::Grabbing));
    } else if hover_state.hovered_gizmo.is_some() {
        commands.entity(window_entity).insert(CursorIcon::from(SystemCursorIcon::Grab));
    } else {
        commands.entity(window_entity).remove::<CursorIcon>();
    }
}

pub fn correct_child_transforms(
    root_query: Query<(Entity, &GlobalTransform), Without<ChildOf>>,
    child_query: Query<(&Transform, Option<&Children>)>,
    mut global_transform_query: Query<&mut GlobalTransform, With<ChildOf>>,
) {
    for (root_entity, root_global) in &root_query {
        let root_unscaled = Transform {
            translation: root_global.translation(),
            rotation: root_global.rotation(),
            scale: Vec3::ONE,
        };
        propagate_unscaled(root_entity, root_unscaled, &child_query, &mut global_transform_query);
    }
}

fn propagate_unscaled(
    parent_entity: Entity,
    parent_unscaled: Transform,
    child_query: &Query<(&Transform, Option<&Children>)>,
    global_transform_query: &mut Query<&mut GlobalTransform, With<ChildOf>>,
) {
    if let Ok((_, Some(children))) = child_query.get(parent_entity) {
        for child in children.iter() {
            if let Ok((local_transform, _)) = child_query.get(child) {
                let child_global_translation = parent_unscaled.translation + parent_unscaled.rotation.mul_vec3(local_transform.translation);
                let child_global_rotation = parent_unscaled.rotation * local_transform.rotation;
                let child_global_scale = local_transform.scale;

                let child_global_transform = Transform {
                    translation: child_global_translation,
                    rotation: child_global_rotation,
                    scale: child_global_scale,
                };

                if let Ok(mut global_transform) = global_transform_query.get_mut(child) {
                    *global_transform = GlobalTransform::from(child_global_transform);
                }

                let child_unscaled = Transform {
                    translation: child_global_translation,
                    rotation: child_global_rotation,
                    scale: Vec3::ONE,
                };

                propagate_unscaled(child, child_unscaled, child_query, global_transform_query);
            }
        }
    }
}