use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use avian3d::prelude::*;
use crate::common::net::components::{Player, NetworkTransform};
use crate::common::net::messages::PlayerInputMessage;

pub fn handle_new_client(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    _players_service_query: Query<Entity, With<crate::common::net::components::PlayersServiceContainer>>,
) {
    let Ok(remote_id) = query.get(trigger.entity) else {
        warn!("handle_new_client failed: RemoteId missing on entity {:?}", trigger.entity);
        return;
    };
    let _client_id = remote_id.0.to_bits();
}

pub fn handle_hello_messages(
    mut commands: Commands,
    mut receivers: Query<(Entity, &RemoteId, &mut MessageReceiver<crate::common::net::messages::HelloMessage>, Option<&ReplicationSender>)>,
    mut sender_query: Query<&mut MessageSender<crate::common::net::messages::KickMessage>>,
    mut success_sender_query: Query<&mut MessageSender<crate::common::net::messages::AuthSuccessMessage>>,
) {
    for (client_entity, remote_id, mut receiver, rep_sender) in receivers.iter_mut() {
        if rep_sender.is_some() {
            continue;
        }
        let client_id = remote_id.0.to_bits();
        for _hello in receiver.receive() {
            debug!("Received HelloMessage from client {}", client_id);

            match crate::common::net::auth::validate_user_ukey(&_hello.ukey) {
                Ok(response) => {
                    if let Ok(mut success_sender) = success_sender_query.get_mut(client_entity) {
                        let _ = success_sender.send::<crate::common::net::messages::GameChannel>(
                            crate::common::net::messages::AuthSuccessMessage {
                                uid: response.uid,
                                username: response.username.clone(),
                            }
                        );
                    }

                    commands.entity(client_entity).insert(ReplicationSender);

                    let (speed, jump_power, gravity_scale, friction, bounciness) = if let Ok(shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.read() {
                        (shared.speed, shared.jump_power, shared.gravity_scale, shared.friction, shared.bounciness)
                    } else {
                        (16.0 * 0.28, 50.0 * 0.28, 1.0, 0.0, 0.0)
                    };

                    let player_entity = commands.spawn((
                        Name::new(response.username.clone()),
                        Player {
                            client_id,
                            speed,
                            jump_power,
                            username: response.username.clone(),
                        },
                        Transform::from_xyz(0.0, 5.0, 0.0),
                        NetworkTransform {
                            translation: Vec3::new(0.0, 5.0, 0.0),
                            rotation: Quat::IDENTITY,
                            scale: Vec3::ONE,
                        },
                        ControlledBy {
                            owner: client_entity,
                            lifetime: Default::default(),
                        },
                        RigidBody::Dynamic,
                        Collider::capsule(1.0 * 0.28, 3.0 * 0.28),
                        CollisionLayers::from_bits(0b0010, 0b0011),
                        LockedAxes::ROTATION_LOCKED,
                    )).insert((
                        Friction::new(friction),
                        Restitution::new(bounciness),
                        GravityScale(gravity_scale),
                        CollidingEntities::default(),
                        SleepingDisabled,
                        Replicate::default(),
                    )).id();

                    info!("Server successfully spawned player entity {:?} for client {}", player_entity, response.username);
                }
                Err(e) => {
                    warn!("Authentication failed for client {}: {}. Sending KickMessage...", client_id, e);
                    if let Ok(mut sender) = sender_query.get_mut(client_entity) {
                        let _ = sender.send::<crate::common::net::messages::GameChannel>(
                            crate::common::net::messages::KickMessage {
                                reason: format!("Authentication failed: {}", e),
                            }
                        );
                    }
                }
            }
        }
    }
}

pub fn handle_player_inputs(
    mut receivers: Query<(Entity, &RemoteId, &mut MessageReceiver<PlayerInputMessage>)>,
    mut players: Query<(Entity, &Player, &mut Transform, &mut LinearVelocity, &CollidingEntities, Option<&ControlledBy>)>,
    spatial_query: SpatialQuery,
) {
    for (client_entity, remote_id, mut receiver) in receivers.iter_mut() {
        let client_id = remote_id.0.to_bits();
        for message in receiver.receive() {
            let mut found_player = false;
            for (player_entity, player, mut transform, mut lin_vel, colliding, controlled_by) in players.iter_mut() {
                let is_owner = if let Some(ctrl) = controlled_by {
                    ctrl.owner == client_entity
                } else {
                    false
                };

                if is_owner || player.client_id == client_id {
                    found_player = true;
                    let speed = player.speed;

                    let mut move_direction = Vec3::ZERO;
                    let rotation = Quat::from_rotation_y(message.yaw);
                    let forward = rotation * Vec3::NEG_Z;
                    let right = rotation * Vec3::X;

                    if message.w {
                        move_direction += forward;
                    }
                    if message.s {
                        move_direction -= forward;
                    }
                    if message.a {
                        move_direction -= right;
                    }
                    if message.d {
                        move_direction += right;
                    }

                    let direction = if move_direction.length_squared() > 0.001 {
                        move_direction.normalize()
                    } else {
                        Vec3::ZERO
                    };

                    lin_vel.x = direction.x * speed;
                    lin_vel.z = direction.z * speed;

                    if direction.length_squared() > 0.001 {
                        let angles = [0.0, 35.0f32.to_radians(), -35.0f32.to_radians()];
                        let mut best_target_y = None;
                        let mut best_step_height = 0.0;

                        for &angle in &angles {
                            let check_dir = if angle == 0.0 {
                                direction
                            } else {
                                Quat::from_rotation_y(angle).mul_vec3(direction)
                            };

                            let step_check_offset = check_dir * 0.35;
                            let player_bottom_y = transform.translation.y - 2.5 * 0.28;
                            let ray_start = transform.translation + step_check_offset;
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
                            transform.translation.y += (target_y - transform.translation.y) * 0.25;
                            if lin_vel.y < 0.0 {
                                lin_vel.y = 0.0;
                            }
                        }
                    }

                    let is_grounded = {
                        let mut grounded = !colliding.is_empty() && lin_vel.y.abs() < 0.2;
                        if !grounded {
                            let ray_origin = transform.translation;
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
                    if message.jump && is_grounded {
                        lin_vel.y = player.jump_power;
                    }

                    if message.in_first_person {
                        transform.rotation = Quat::from_rotation_y(message.yaw);
                    } else if direction.length_squared() > 0.001 {
                        let target_angle = direction.z.atan2(direction.x);
                        transform.rotation = Quat::from_rotation_y(-target_angle + std::f32::consts::FRAC_PI_2);
                    }

                    trace!("Player ID: {} - Position: {:?}", player.client_id, transform.translation);
                    break;
                }
            }

            if !found_player {
                warn!("handle_player_inputs: No player entity found for client_id: {} / entity: {:?}", client_id, client_entity);
            }
        }
    }
}

pub fn sync_players_service_properties(
    mut last_service: Local<crate::studio::tools::PlayersService>,
    mut query: Query<(&mut Player, &mut Friction, &mut Restitution, &mut GravityScale)>,
) {
    let current_service = if let Ok(shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.read() {
        shared.clone()
    } else {
        return;
    };

    if current_service.speed != last_service.speed
        || current_service.jump_power != last_service.jump_power
        || current_service.gravity_scale != last_service.gravity_scale
        || current_service.friction != last_service.friction
        || current_service.bounciness != last_service.bounciness
    {
        for (mut player, mut friction, mut restitution, mut gravity_scale) in &mut query {
            if player.speed != current_service.speed {
                player.speed = current_service.speed;
            }
            if player.jump_power != current_service.jump_power {
                player.jump_power = current_service.jump_power;
            }
            let new_friction = Friction::new(current_service.friction);
            if *friction != new_friction {
                *friction = new_friction;
            }
            let new_restitution = Restitution::new(current_service.bounciness);
            if *restitution != new_restitution {
                *restitution = new_restitution;
            }
            let new_gravity_scale = GravityScale(current_service.gravity_scale);
            if *gravity_scale != new_gravity_scale {
                *gravity_scale = new_gravity_scale;
            }
        }
        *last_service = current_service;
    }
}

pub fn handle_client_disconnect(
    trigger: On<Remove, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    players_query: Query<(Entity, &Player)>,
    mut commands: Commands,
) {
    let Ok(remote_id) = query.get(trigger.entity) else { return };
    let client_id = remote_id.0.to_bits();
    info!("Client disconnected: {}", client_id);
    for (player_entity, player) in &players_query {
        if player.client_id == client_id {
            commands.entity(player_entity).despawn();
        }
    }
}

pub fn sync_transforms_to_network(
    mut query: Query<(&Transform, &mut NetworkTransform), Changed<Transform>>,
) {
    for (transform, mut net_transform) in &mut query {
        net_transform.translation = transform.translation;
        net_transform.rotation = transform.rotation;
        net_transform.scale = transform.scale;
    }
}