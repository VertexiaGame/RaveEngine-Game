use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use avian3d::prelude::*;
use crate::common::net::components::{Player, NetworkTransform};
use crate::common::net::messages::PlayerInputMessage;

pub fn handle_new_client(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
    _players_service_query: Query<Entity, With<crate::common::net::components::PlayersServiceContainer>>,
) {
    let Ok(remote_id) = query.get(trigger.entity) else {
        warn!("handle_new_client failed: RemoteId missing on entity {:?}", trigger.entity);
        return;
    };
    let client_id = remote_id.0.to_bits();
    info!("Server handle_new_client: Client connected with id: {}", client_id);

    commands.entity(trigger.entity).insert(ReplicationSender);

    let player_entity = commands.spawn((
        Name::new(format!("Player_{}", client_id)),
        Player {
            client_id,
            speed: 16.0 * 0.28,
            jump_power: 50.0 * 0.28,
        },
        Transform::from_xyz(0.0, 5.0, 0.0),
        NetworkTransform {
            translation: Vec3::new(0.0, 5.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
        ControlledBy {
            owner: trigger.entity,
            lifetime: Default::default(),
        },
        RigidBody::Dynamic,
        Collider::cuboid(2.0 * 0.28, 5.0 * 0.28, 1.0 * 0.28),
        LockedAxes::ROTATION_LOCKED,
        Friction::new(0.0),
        Restitution::new(0.0),
        CollidingEntities::default(),
        SleepingDisabled,
        Replicate::default(),
    )).id();

    info!("Server successfully spawned player entity {:?} for client {}", player_entity, client_id);
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

                    let is_grounded = {
                        let mut grounded = !colliding.is_empty() && lin_vel.y.abs() < 0.2;
                        if !grounded {
                            let ray_origin = transform.translation;
                            let max_ray_dist = 2.5 * 0.28 + 0.15;
                            let filter = SpatialQueryFilter::default().with_excluded_entities([player_entity]);
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