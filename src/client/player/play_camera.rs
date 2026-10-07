use super::{CameraSettings, PlayerCamera};
use crate::client::LocalPlayer;
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

pub fn update_camera(
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut player_query: Query<(Entity, &mut Transform, Option<&Children>), With<LocalPlayer>>,
    mut camera_query: Query<
        (&mut Transform, &mut CameraSettings),
        (With<PlayerCamera>, Without<LocalPlayer>),
    >,
    spatial_query: SpatialQuery,
    mut window_query: Query<&mut CursorOptions, With<bevy::window::PrimaryWindow>>,
    time: Res<Time>,
    mut last_log: Local<f32>,
    mut excluded: Local<Vec<Entity>>,
    mut contexts: bevy_egui::EguiContexts,
) {
    let Some((player_entity, mut player_transform, _children_opt)) = player_query.iter_mut().next()
    else {
        return;
    };
    let Some((mut camera_transform, mut settings)) = camera_query.iter_mut().next() else {
        return;
    };

    let mut rotation_move = Vec2::ZERO;
    for event in mouse_motion.read() {
        rotation_move += event.delta;
    }

    let mut scroll_val = 0.0;
    for event in mouse_wheel.read() {
        scroll_val += event.y;
    }
    let pointer_over_ui = contexts
        .ctx_mut()
        .is_ok_and(|ctx| ctx.egui_wants_pointer_input() || ctx.is_pointer_over_egui());
    if !pointer_over_ui && scroll_val != 0.0 {
        settings.distance = (settings.distance - scroll_val * 0.5).clamp(0.5, 40.0);
    }

    let lerp_factor = (15.0 * time.delta_secs()).min(1.0);
    settings.current_distance =
        settings.current_distance + (settings.distance - settings.current_distance) * lerp_factor;

    let in_first_person = settings.current_distance <= 0.6;

    if let Some(mut cursor_opts) = window_query.iter_mut().next() {
        if in_first_person || mouse_buttons.pressed(MouseButton::Right) {
            cursor_opts.visible = false;
            cursor_opts.grab_mode = CursorGrabMode::Locked;
            settings.yaw -= rotation_move.x * 0.005;
            settings.pitch = (settings.pitch - rotation_move.y * 0.005).clamp(-1.4, 1.4);
        } else {
            cursor_opts.visible = true;
            cursor_opts.grab_mode = CursorGrabMode::None;
        }
    }

    if in_first_person {
        player_transform.rotation = Quat::from_rotation_y(settings.yaw);
    }

    let target_translation = player_transform.translation + settings.target_offset;
    let rotation = Quat::from_rotation_y(settings.yaw) * Quat::from_rotation_x(settings.pitch);
    let camera_offset = rotation.mul_vec3(Vec3::new(0.0, 0.0, settings.current_distance));

    let ray_dir = camera_offset.normalize_or_zero();
    let mut final_distance = settings.current_distance;

    if !in_first_person {
        if let Ok(dir3) = Dir3::new(ray_dir) {
            excluded.clear();
            excluded.push(player_entity);
            if let Some(children) = _children_opt {
                excluded.extend(children.iter());
            }
            let filter = SpatialQueryFilter::default()
                .with_excluded_entities(excluded.iter().copied())
                .with_mask(0b0101);
            if let Some(hit) = spatial_query.cast_ray(
                target_translation,
                dir3,
                settings.current_distance,
                true,
                &filter,
            ) {
                final_distance = (hit.distance - 0.15).max(0.2);
            }
        }
    } else {
        final_distance = 0.0;
    }

    let final_offset = rotation.mul_vec3(Vec3::new(0.0, 0.0, final_distance));
    camera_transform.translation = target_translation + final_offset;
    camera_transform.rotation = rotation;

    let now = time.elapsed_secs();
    if now - *last_log >= 1.0 {
        *last_log = now;
        debug!(
            "PLAYER_LOG: update_camera current_distance={}, final_distance={}, camera_translation={:?}, target_translation={:?}",
            settings.current_distance,
            final_distance,
            camera_transform.translation,
            target_translation
        );
    }
}
