use bevy::prelude::*;
use avian3d::prelude::*;
use bevy_egui::EguiContexts;
use super::{Player, PlayerController, CameraSettings, PlayerCamera};

pub fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<
        (
            Entity,
            &mut Transform,
            &mut LinearVelocity,
            &PlayerController,
            &CollidingEntities,
        ),
        With<Player>,
    >,
    camera_query: Query<(&Transform, &CameraSettings), (With<PlayerCamera>, Without<Player>)>,
    spatial_query: SpatialQuery,
    mut contexts: EguiContexts,
) {
    let wants_keyboard = if let Ok(ctx) = contexts.ctx_mut() {
        ctx.egui_wants_keyboard_input()
    } else {
        false
    };

    let Some((player_entity, mut player_transform, mut velocity, controller, colliding)) =
        player_query.iter_mut().next()
    else {
        return;
    };
    let Some((camera_transform, camera_settings)) = camera_query.iter().next() else {
        return;
    };

    if wants_keyboard {
        velocity.x = 0.0;
        velocity.z = 0.0;
        return;
    }

    let in_first_person = camera_settings.distance <= 0.6;

    let mut move_direction = Vec3::ZERO;

    let camera_forward = *camera_transform.forward();
    let camera_right = *camera_transform.right();

    let mut forward_proj = Vec3::new(camera_forward.x, 0.0, camera_forward.z);
    let mut right_proj = Vec3::new(camera_right.x, 0.0, camera_right.z);

    if forward_proj.length_squared() > 0.001 {
        forward_proj = forward_proj.normalize();
    }
    if right_proj.length_squared() > 0.001 {
        right_proj = right_proj.normalize();
    }

    if keys.pressed(KeyCode::KeyW) {
        move_direction += forward_proj;
    }
    if keys.pressed(KeyCode::KeyS) {
        move_direction -= forward_proj;
    }
    if keys.pressed(KeyCode::KeyA) {
        move_direction -= right_proj;
    }
    if keys.pressed(KeyCode::KeyD) {
        move_direction += right_proj;
    }

    if move_direction.length_squared() > 0.001 {
        let move_direction = move_direction.normalize();
        let target_vel = move_direction * controller.move_speed;
        velocity.x = target_vel.x;
        velocity.z = target_vel.z;

        let angles = [0.0, 35.0f32.to_radians(), -35.0f32.to_radians()];
        let mut best_target_y = None;
        let mut best_step_height = 0.0;

        for &angle in &angles {
            let check_dir = if angle == 0.0 {
                move_direction
            } else {
                Quat::from_rotation_y(angle).mul_vec3(move_direction)
            };

            let step_check_offset = check_dir * 0.35;
            let player_bottom_y = player_transform.translation.y - 2.5 * 0.28;
            let ray_start = player_transform.translation + step_check_offset;
            let ray_origin = Vec3::new(ray_start.x, player_bottom_y + 0.32, ray_start.z);

            let filter = SpatialQueryFilter::default()
                .with_excluded_entities([player_entity])
                .with_mask(0b0011);
            if let Some(hit) = spatial_query.cast_ray(ray_origin, Dir3::NEG_Y, 0.45, true, &filter) {
                let hit_point_y = ray_origin.y - hit.distance;
                let step_height = hit_point_y - player_bottom_y;

                if step_height > 0.01 && step_height <= 0.29 {
                    if step_height > best_step_height {
                        best_step_height = step_height;
                        best_target_y = Some(hit_point_y + 2.5 * 0.28);
                    }
                }
            }
        }

        if let Some(target_y) = best_target_y {
            player_transform.translation.y += (target_y - player_transform.translation.y) * 0.25;
            if velocity.y < 0.0 {
                velocity.y = 0.0;
            }
        }

        if !in_first_person {
            let target_angle = move_direction.z.atan2(move_direction.x);
            let target_rotation =
                Quat::from_rotation_y(-target_angle + std::f32::consts::FRAC_PI_2);
            player_transform.rotation = player_transform.rotation.lerp(target_rotation, 0.15);
        }
    } else {
        velocity.x = 0.0;
        velocity.z = 0.0;
    }

    let is_grounded = {
        let mut grounded = !colliding.is_empty() && velocity.y.abs() < 0.2;
        if !grounded {
            let ray_origin = player_transform.translation;
            let max_ray_dist = 2.5 * 0.28 + 0.15;
            let filter = SpatialQueryFilter::default()
                .with_excluded_entities([player_entity])
                .with_mask(0b0011);
            if spatial_query.cast_ray(ray_origin, Dir3::NEG_Y, max_ray_dist, true, &filter).is_some() {
                grounded = true;
            }
        }
        grounded
    };
    if keys.pressed(KeyCode::Space) && is_grounded {
        velocity.y = controller.jump_power;
    }
}