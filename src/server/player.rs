use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use avian3d::prelude::*;
use crate::common::net::components::{Player, NetworkTransform};
use crate::common::net::messages::PlayerInputMessage;
use crate::server::ClientPlayerMap;
use std::time::Instant;

const INPUT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(150);

#[derive(Component)]
pub struct PlayerInputTracker {
    pub last_input: Instant,
}

impl Default for PlayerInputTracker {
    fn default() -> Self {
        Self {
            last_input: Instant::now(),
        }
    }
}

#[derive(Component, Default)]
pub struct PlayerMovementState {
    pub movement: crate::common::game::movement::CharacterMoveState,
    pub wish_direction: Vec2,
    pub yaw: f32,
    pub in_first_person: bool,
    pub jump_held: bool,
}

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
    mut player_map: ResMut<ClientPlayerMap>,
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
                        Collider::cuboid(2.0 * 0.28, 5.0 * 0.28, 1.17 * 0.28),
                        CollisionLayers::from_bits(0b0010, 0b0011),
                        LockedAxes::ROTATION_LOCKED,
                    )).insert((
                        Friction::new(friction),
                        Restitution::new(bounciness),
                        GravityScale(gravity_scale),
                        CollidingEntities::default(),
                        SleepingDisabled,
                        SweptCcd::default().with_velocity_threshold(2.0, 0.5),
                        SpeculativeMargin(0.0),
                        PlayerInputTracker::default(),
                        PlayerMovementState::default(),
                        crate::common::game::movement::PlayerMovementPlan::default(),
                        Replicate::default(),
                    )).id();

                    info!("Server successfully spawned player entity {:?} for client {}", player_entity, response.username);
                    player_map.0.insert(client_id, player_entity);
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
    player_map: Res<ClientPlayerMap>,
    mut players: Query<(&mut PlayerMovementState, &mut PlayerInputTracker)>,
) {
    let now = Instant::now();
    for (client_entity, remote_id, mut receiver) in receivers.iter_mut() {
        let client_id = remote_id.0.to_bits();
        let mut last_message: Option<PlayerInputMessage> = None;
        for message in receiver.receive() {
            last_message = Some(message);
        }
        let Some(message) = last_message else {
            continue;
        };

        let Some(&player_entity) = player_map.0.get(&client_id) else {
            warn!("handle_player_inputs: No player entity found for client_id: {} / entity: {:?}", client_id, client_entity);
            continue;
        };

        let Ok((mut state, mut tracker)) = players.get_mut(player_entity) else {
            continue;
        };
        tracker.last_input = now;

        let rotation = Quat::from_rotation_y(message.yaw);
        let forward = rotation * Vec3::NEG_Z;
        let right = rotation * Vec3::X;

        let mut move_direction = Vec3::ZERO;
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

        state.wish_direction = Vec2::new(direction.x, direction.z);
        state.yaw = message.yaw;
        state.in_first_person = message.in_first_person;
        state.jump_held = message.jump;
    }
}

pub fn tick_player_movement(
    mut players: Query<(
        Entity,
        &Player,
        &Position,
        &Rotation,
        &LinearVelocity,
        &PlayerInputTracker,
        &mut PlayerMovementState,
        &mut crate::common::game::movement::PlayerMovementPlan,
    )>,
    spatial_query: SpatialQuery,
    time: Res<Time>,
) {
    use crate::common::game::movement::*;

    let dt = time.delta_secs().min(0.1);
    let elapsed = time.elapsed_secs();
    let now = Instant::now();

    for (player_entity, player, position, rotation, lin_vel, tracker, mut state, mut plan) in &mut players {
        let has_input = now.duration_since(tracker.last_input) <= INPUT_TIMEOUT;
        let wish_direction = if has_input { state.wish_direction } else { Vec2::ZERO };
        let jump_held = has_input && state.jump_held;

        let filter = SpatialQueryFilter::default()
            .with_excluded_entities([player_entity])
            .with_mask(0b0011);
        let mut cast = |probe, origin, rotation, direction: Vec3, max_distance| {
            let Ok(direction) = Dir3::new(direction) else {
                return None;
            };
            spatial_query
                .cast_shape(
                    &probe_shape(probe),
                    origin,
                    rotation,
                    direction,
                    &ShapeCastConfig::from_max_distance(max_distance),
                    &filter,
                )
                .map(|hit| CastHit {
                    distance: hit.distance,
                    normal: hit.normal1,
                })
        };

        let params = CharacterMoveParams {
            wish_direction,
            jump_held,
            speed: player.speed,
            jump_power: player.jump_power,
            dt,
            elapsed,
        };
        let yaw = rotation.0.to_euler(EulerRot::YXZ).0;
        let output = character_move(position.0, lin_vel.0, yaw, &mut state.movement, &params, &mut cast);

        let has_wish = wish_direction.length_squared() > 0.001;
        let rotation = if state.in_first_person {
            Some(Quat::from_rotation_y(state.yaw))
        } else if has_wish {
            let target_angle = wish_direction.y.atan2(wish_direction.x);
            let target_rotation =
                Quat::from_rotation_y(-target_angle + std::f32::consts::FRAC_PI_2);
            let turn_factor = 1.0 - (-TURN_SPEED * dt).exp();
            Some(rotation.0.slerp(target_rotation, turn_factor))
        } else {
            None
        };

        plan.velocity = output.velocity;
        plan.position_y = output.position_y;
        plan.position_xz = output.position_xz;
        plan.rotation = rotation;

        trace!("Player ID: {} - Position: {:?}", player.client_id, position.0);
    }
}

pub fn apply_player_movement(
    mut players: Query<(
        &mut Position,
        &mut Rotation,
        &mut LinearVelocity,
        &mut Transform,
        &crate::common::game::movement::PlayerMovementPlan,
    )>,
) {
    for (mut position, mut rotation, mut lin_vel, mut transform, plan) in &mut players {
        lin_vel.0 = plan.velocity;
        if let Some(xz) = plan.position_xz {
            position.0.x = xz.x;
            position.0.z = xz.y;
        }
        if let Some(y) = plan.position_y {
            position.0.y = y;
        }
        if let Some(target_rotation) = plan.rotation {
            transform.rotation = target_rotation;
            rotation.0 = target_rotation;
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
    mut player_map: ResMut<ClientPlayerMap>,
    players_query: Query<(Entity, &Player)>,
    mut commands: Commands,
) {
    let Ok(remote_id) = query.get(trigger.entity) else { return };
    let client_id = remote_id.0.to_bits();
    info!("Client disconnected: {}", client_id);
    player_map.0.remove(&client_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::game::physics::PhysicsSimulationPlugin;
    use bevy::asset::{AssetEvent, Assets};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::render::mesh::Mesh;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsSimulationPlugin));
        app.add_systems(Update, |mut time_physics: ResMut<Time<Physics>>| {
            time_physics.unpause();
        });
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
        app.init_resource::<Assets<Mesh>>();
        app.add_message::<AssetEvent<Mesh>>();
        app.init_resource::<avian3d::collider_tree::ColliderTreeDiagnostics>();
        app.init_resource::<avian3d::collision::CollisionDiagnostics>();
        app.init_resource::<avian3d::dynamics::solver::SolverDiagnostics>();
        app.init_resource::<avian3d::spatial_query::SpatialQueryDiagnostics>();
        app
    }

    fn spawn_floor(app: &mut App) {
        app.world_mut().spawn((
            Transform::from_xyz(0.0, -0.14, 0.0),
            RigidBody::Static,
            Collider::cuboid(20.0, 0.28, 20.0),
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
        ));
    }

    fn spawn_player(app: &mut App, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                Name::new("TestPlayer"),
                Player {
                    client_id: 0,
                    speed: 16.0 * 0.28,
                    jump_power: 50.0 * 0.28,
                    username: "TestPlayer".to_string(),
                },
                Transform::from_translation(position),
                RigidBody::Dynamic,
                Collider::cuboid(2.0 * 0.28, 5.0 * 0.28, 1.17 * 0.28),
                CollisionLayers::from_bits(0b0010, 0b0011),
                LockedAxes::ROTATION_LOCKED,
                Friction::new(0.0),
                Restitution::new(0.0),
                CollidingEntities::default(),
                SleepingDisabled,
                SweptCcd::default(),
                SpeculativeMargin(0.0),
            ))
            .insert(LinearVelocity::ZERO)
            .insert(PlayerInputTracker::default())
            .insert(PlayerMovementState::default())
            .insert(crate::common::game::movement::PlayerMovementPlan::default())
            .id()
    }

    fn register_movement(app: &mut App) {
        app.add_systems(
            FixedPostUpdate,
            (tick_player_movement, apply_player_movement)
                .chain()
                .in_set(PhysicsSystems::Prepare)
                .after(avian3d::physics_transform::PhysicsTransformSystems::TransformToPosition),
        );
    }

    #[test]
    fn player_cuboid_rests_without_sliding() {
        let mut app = test_app();
        spawn_floor(&mut app);
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.7, 0.0));

        for _ in 0..240 {
            app.update();
        }

        let transform = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap();
        assert!(
            transform.translation.x.abs() < 0.05,
            "player slid in x: {}",
            transform.translation.x
        );
        assert!(
            transform.translation.z.abs() < 0.05,
            "player slid in z: {}",
            transform.translation.z
        );
        assert!(
            (transform.translation.y - 0.7).abs() < 0.1,
            "player y: {}",
            transform.translation.y
        );
    }

    #[test]
    fn player_cuboid_steps_up_a_step() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.14, -3.0),
            RigidBody::Static,
            Collider::cuboid(4.0, 0.28, 2.0),
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
        ));
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.7, -1.5));

        let mut state_query = app.world_mut().query::<&mut PlayerMovementState>();
        let mut state = state_query.get_mut(app.world_mut(), player).unwrap();
        state.wish_direction = Vec2::new(0.0, -1.0);

        app.add_systems(
            Update,
            |mut q: Query<(&mut PlayerInputTracker, &mut PlayerMovementState)>| {
                for (mut tracker, mut state) in &mut q {
                    tracker.last_input = std::time::Instant::now();
                    state.wish_direction = Vec2::new(0.0, -1.0);
                }
            },
        );

        for _ in 0..25 {
            app.update();
        }

        let transform = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap();
        assert!(
            transform.translation.y > 0.9,
            "player did not step up, y: {}",
            transform.translation.y
        );
    }

    #[test]
    fn player_cuboid_rests_still_on_a_step_top() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.14, -3.0),
            RigidBody::Static,
            Collider::cuboid(4.0, 0.28, 2.0),
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
        ));
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.98, -3.0));

        app.add_systems(
            Update,
            |mut q: Query<&mut PlayerInputTracker>| {
                for mut tracker in &mut q {
                    tracker.last_input = std::time::Instant::now();
                }
            },
        );

        for _ in 0..240 {
            app.update();
        }

        let transform = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap();
        assert!(
            transform.translation.x.abs() < 0.05,
            "player slid in x: {}",
            transform.translation.x
        );
        assert!(
            (transform.translation.z - (-3.0)).abs() < 0.05,
            "player slid in z: {}",
            transform.translation.z
        );
        assert!(
            (transform.translation.y - 0.98).abs() < 0.1,
            "player y: {}",
            transform.translation.y
        );
    }

    #[test]
    fn player_rotation_follows_movement_direction() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.7, 0.0));

        app.add_systems(
            Update,
            |mut q: Query<(&mut PlayerInputTracker, &mut PlayerMovementState)>| {
                for (mut tracker, mut state) in &mut q {
                    tracker.last_input = std::time::Instant::now();
                    state.wish_direction = Vec2::new(0.0, -1.0);
                }
            },
        );

        for _ in 0..120 {
            app.update();
        }

        let transform = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap();
        let target_angle = (-1.0f32).atan2(0.0);
        let target = Quat::from_rotation_y(-target_angle + std::f32::consts::FRAC_PI_2);
        assert!(
            transform.rotation.angle_between(target) < 0.1,
            "player did not turn toward movement direction, rotation: {:?}, expected: {:?}",
            transform.rotation,
            target
        );
    }

    #[test]
    fn player_walks_up_staircase_then_rests_without_drifting() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        for i in 0..4 {
            app.world_mut().spawn((
                Transform::from_xyz(0.0, 0.14 + i as f32 * 0.28, -3.0 - i as f32 * 0.7),
                RigidBody::Static,
                Collider::cuboid(4.0, 0.28, 2.0),
                CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
            ));
        }
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.7, 1.0));

        app.add_systems(
            Update,
            |mut q: Query<(&mut PlayerInputTracker, &mut PlayerMovementState)>, mut frames: Local<u32>| {
                for (mut tracker, mut state) in &mut q {
                    tracker.last_input = std::time::Instant::now();
                    if *frames < 60 {
                        state.wish_direction = Vec2::new(0.0, -1.0);
                    } else {
                        state.wish_direction = Vec2::ZERO;
                    }
                }
                *frames += 1;
            },
        );

        for _ in 0..60 {
            app.update();
        }

        for _ in 0..400 {
            app.update();
        }

        let transform = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap();
        let pos = transform.translation;
        let vel = app
            .world_mut()
            .query::<&LinearVelocity>()
            .get(app.world(), player)
            .unwrap()
            .0;
        assert!(
            vel.length() < 0.05,
            "player keeps moving after resting, vel: {:?}",
            vel
        );
        for _ in 0..240 {
            app.update();
        }
        let transform = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap();
        assert!(
            (transform.translation.x - pos.x).abs() < 0.05,
            "player drifted in x: {} -> {}",
            pos.x,
            transform.translation.x
        );
        assert!(
            (transform.translation.z - pos.z).abs() < 0.05,
            "player drifted in z: {} -> {}",
            pos.z,
            transform.translation.z
        );
        assert!(
            (transform.translation.y - pos.y).abs() < 0.1,
            "player y: {} -> {}",
            pos.y,
            transform.translation.y
        );
    }

    #[test]
    fn player_steps_up_then_rests_without_drifting() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.14, -3.0),
            RigidBody::Static,
            Collider::cuboid(4.0, 0.28, 2.0),
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
        ));
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.7, -1.5));

        app.add_systems(
            Update,
            |mut q: Query<(&mut PlayerInputTracker, &mut PlayerMovementState)>, mut frames: Local<u32>| {
                for (mut tracker, mut state) in &mut q {
                    tracker.last_input = std::time::Instant::now();
                    if *frames < 30 {
                        state.wish_direction = Vec2::new(0.0, -1.0);
                    } else {
                        state.wish_direction = Vec2::ZERO;
                    }
                }
                *frames += 1;
            },
        );

        for _ in 0..30 {
            app.update();
        }

        for _ in 0..60 {
            app.update();
        }
        let settle_xz = {
            let transform = app
                .world_mut()
                .query::<&Transform>()
                .get(app.world(), player)
                .unwrap()
                .translation;
            (transform.x, transform.z)
        };

        for _ in 0..400 {
            app.update();
        }

        let transform = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap();
        let pos = transform.translation;
        let vel = app
            .world_mut()
            .query::<&LinearVelocity>()
            .get(app.world(), player)
            .unwrap()
            .0;
        assert!(
            (pos.x - settle_xz.0).abs() < 0.05,
            "player drifted in x: {} -> {}",
            settle_xz.0,
            pos.x
        );
        assert!(
            (pos.z - settle_xz.1).abs() < 0.05,
            "player drifted in z: {} -> {}",
            settle_xz.1,
            pos.z
        );
        assert!(
            (pos.y - 0.98).abs() < 0.01,
            "player did not rest on the step, y: {}",
            pos.y
        );
        assert!(
            vel.y.abs() < 0.05,
            "player keeps falling into the step, vel.y: {}",
            vel.y
        );
    }
}