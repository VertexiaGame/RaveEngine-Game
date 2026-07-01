use bevy::prelude::*;
use bevy::winit::{WinitSettings, UpdateMode};
use bevy::window::{PrimaryWindow, WindowMode};
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::post_process::bloom::Bloom;
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ContactShadows};
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
        app.insert_resource(WinitSettings::desktop_app())
            .init_resource::<GraphicsSettings>()
            .add_systems(Update, (
                apply_graphics_settings,
                manage_winit_performance,
            ));
    }
}

pub fn apply_graphics_settings(
    settings: Res<GraphicsSettings>,
    mut commands: Commands,
    camera_query: Query<Entity, (With<Camera3d>, With<FreeCamera>)>,
) {
    if !settings.is_changed() {
        return;
    }
    for entity in &camera_query {
        if settings.ssao {
            commands.entity(entity).insert(ScreenSpaceAmbientOcclusion::default());
        } else {
            commands.entity(entity).remove::<ScreenSpaceAmbientOcclusion>();
        }

        if settings.contact_shadows {
            commands.entity(entity).insert(ContactShadows::default());
        } else {
            commands.entity(entity).remove::<ContactShadows>();
        }

        if settings.bloom {
            commands.entity(entity).insert(Bloom::default());
        } else {
            commands.entity(entity).remove::<Bloom>();
        }
    }
}

pub fn manage_winit_performance(
    mut winit_settings: ResMut<WinitSettings>,
    drag_state: Option<Res<crate::studio::tools::DragState>>,
    part_drag_state: Option<Res<crate::studio::tools::PartDragState>>,
    physics_state: Option<Res<crate::common::physics::PhysicsSimulationState>>,
    camera_query: Query<(Entity, &Transform), (With<Camera3d>, With<FreeCamera>)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut prev_transforms: Query<&mut PreviousTransform>,
    mut commands: Commands,
    mut last_mouse_position: Local<Option<Vec2>>,
    mut last_mouse_movement_time: Local<f32>,
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
    let is_mouse_active = is_hovered && (time_since_last_move < 30.0);

    let mut is_active = is_fullscreen || is_mouse_active;

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
    if let Some(ps) = physics_state {
        if *ps == crate::common::physics::PhysicsSimulationState::Running {
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
        winit_settings.focused_mode = UpdateMode::reactive(Duration::from_millis(16));
    }
}