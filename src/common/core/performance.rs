use bevy::prelude::*;
use bevy::winit::{WinitSettings, UpdateMode};
use bevy::window::{PrimaryWindow, WindowMode};
use std::time::Duration;

#[derive(Component)]
pub struct PreviousTransform(pub Transform);

#[derive(Resource)]
pub struct GraphicsSettings {
    pub ssao: bool,
    pub contact_shadows: bool,
    pub bloom: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            ssao: false,
            contact_shadows: false,
            bloom: true,
        }
    }
}

pub struct PerformancePlugin;

impl Plugin for PerformancePlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.insert_resource(WinitSettings::desktop_app())
                .init_resource::<GraphicsSettings>()
                .add_systems(Update, manage_winit_performance);
        }
    }
}

pub fn manage_winit_performance(
    mut winit_settings: ResMut<WinitSettings>,
    drag_state: Option<Res<crate::studio::tools::DragState>>,
    part_drag_state: Option<Res<crate::studio::tools::PartDragState>>,
    physics_state: Option<Res<crate::common::game::physics::PhysicsSimulationState>>,
    camera_query: Query<(Entity, &Transform), With<Camera3d>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut prev_transforms: Query<&mut PreviousTransform>,
    mut commands: Commands,
    mut last_mouse_position: Local<Option<Vec2>>,
    mut last_mouse_movement_time: Local<f32>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
) {
    let current_time = time.elapsed_secs();
    
    let mut is_hovered = false;
    let mut is_fullscreen = false;
    
    if let Ok(window) = windows.single() {
        if !matches!(window.mode, WindowMode::Windowed) {
            is_fullscreen = true;
        }
        if let Some(cursor_pos) = window.cursor_position() {
            is_hovered = true;
            if let Some(last_pos) = *last_mouse_position {
                if cursor_pos.distance_squared(last_pos) > 0.0001 {
                    *last_mouse_position = Some(cursor_pos);
                    *last_mouse_movement_time = current_time;
                }
            } else {
                *last_mouse_position = Some(cursor_pos);
                *last_mouse_movement_time = current_time;
            }
        } else {
            *last_mouse_position = None;
        }
    }

    let time_since_last_move = current_time - *last_mouse_movement_time;
    let is_mouse_active = is_hovered && (time_since_last_move < 3.0);

    let buttons_pressed = mouse_buttons.is_some_and(|b| b.any_pressed([
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
        MouseButton::Back,
        MouseButton::Forward,
    ]));
    let keys_pressed = keys.is_some_and(|k| k.any_pressed([
        KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD,
        KeyCode::KeyQ, KeyCode::KeyE, KeyCode::ArrowUp, KeyCode::ArrowDown,
        KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::Space,
        KeyCode::ShiftLeft, KeyCode::ShiftRight,
        KeyCode::ControlLeft, KeyCode::ControlRight,
    ]));

    let mut is_active = is_fullscreen || is_mouse_active || buttons_pressed || keys_pressed;

    if let Some(ds) = drag_state {
        if ds.active {
            is_active = true;
        }
    }
    if let Some(pds) = part_drag_state {
        if pds.active {
            is_active = true;
        }
    }

    let mut physics_running = false;
    if let Some(ps) = physics_state {
        if *ps == crate::common::game::physics::PhysicsSimulationState::Running {
            physics_running = true;
            is_active = true;
        }
    }

    for (entity, transform) in &camera_query {
        if let Ok(mut prev) = prev_transforms.get_mut(entity) {
            let dist_sq = transform.translation.distance_squared(prev.0.translation);
            let rot_diff = transform.rotation.dot(prev.0.rotation).abs();
            if dist_sq > 0.00001 || rot_diff < 0.99999 {
                is_active = true;
            }
            prev.0 = *transform;
        } else {
            commands.entity(entity).insert(PreviousTransform(*transform));
            is_active = true;
        }
    }

    if is_active {
        winit_settings.focused_mode = UpdateMode::Continuous;
    } else {
        winit_settings.focused_mode = UpdateMode::reactive(Duration::from_secs(60));
    }

    if physics_running {
        winit_settings.unfocused_mode = UpdateMode::Continuous;
    } else {
        winit_settings.unfocused_mode = UpdateMode::reactive_low_power(Duration::from_secs(60));
    }
}