use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use avian3d::prelude::*;
use crate::common::game::movement::{
    validate_client_move, ClientMoveClaim, MoveValidationConfig, PlayerMovementPlan,
    ValidatedMove, VALIDATION_VIOLATION_LOG_THRESHOLD,
};
use crate::common::net::components::{Player, NetworkTransform};
use crate::common::net::messages::PlayerMoveMessage;
use crate::server::ClientPlayerMap;
use std::time::Instant;
#[derive(Component)]
pub struct ServerMoveState {
    pub last: Option<ValidatedMove>,
    pub budget: f32,
    pub violations: u32,
    pub last_move_at: Option<Instant>,
}

impl Default for ServerMoveState {
    fn default() -> Self {
        Self {
            last: None,
            budget: 0.0,
            violations: 0,
            last_move_at: None,
        }
    }
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

const MAX_PENDING_AUTHS: usize = 64;

pub fn handle_hello_messages(
    mut pending: ResMut<crate::server::PendingAuths>,
    mut auth_pool: ResMut<crate::server::AuthWorkerPool>,
    settings: Res<crate::server::ServerSettings>,
    mut receivers: Query<(Entity, &RemoteId, &mut MessageReceiver<crate::common::net::messages::HelloMessage>, Option<&ReplicationSender>)>,
) {
    auth_pool.ensure_started(settings.allow_unauthenticated);
    for (client_entity, remote_id, mut receiver, rep_sender) in receivers.iter_mut() {
        if rep_sender.is_some() {
            continue;
        }
        let client_id = remote_id.0.to_bits();
        for hello in receiver.receive() {
            if pending.0.contains_key(&client_id) {
                continue;
            }
            if pending.0.len() >= MAX_PENDING_AUTHS {
                warn!("Pending auth queue full; dropping hello from client {}", client_id);
                break;
            }
            if !auth_pool.submit(client_id, hello.ukey.clone()) {
                warn!("Auth worker queue full; dropping hello from client {}", client_id);
                continue;
            }
            debug!("Received HelloMessage from client {}, queued async auth", client_id);
            pending.0.insert(client_id, client_entity);
        }
    }
}

pub fn handle_auth_results(
    mut commands: Commands,
    mut player_map: ResMut<crate::server::ClientPlayerMap>,
    mut pending: ResMut<crate::server::PendingAuths>,
    auth_pool: Res<crate::server::AuthWorkerPool>,
    mut sender_query: Query<&mut MessageSender<crate::common::net::messages::KickMessage>>,
    mut success_sender_query: Query<&mut MessageSender<crate::common::net::messages::AuthSuccessMessage>>,
) {
    let results = auth_pool.drain_results();

    for (client_id, result) in results {
        let Some(&client_entity) = pending.0.get(&client_id) else {
            continue;
        };
        pending.0.remove(&client_id);
        match result {
            Ok(response) => {
                if commands.get_entity(client_entity).is_err() {
                    warn!("Authentication completed for disconnected client {}, skipping spawn", client_id);
                    continue;
                }
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
                        velocity: Vec3::ZERO,
                    },
                    ControlledBy {
                        owner: client_entity,
                        lifetime: Default::default(),
                    },
                    RigidBody::Kinematic,
                    Collider::cuboid(2.0 * 0.28, 5.0 * 0.28, 1.17 * 0.28),
                    CollisionLayers::from_bits(0b0010, 0b0011),
                    LockedAxes::ROTATION_LOCKED,
                    CustomPositionIntegration,
                )).insert((
                    Friction::new(friction),
                    Restitution::new(bounciness),
                    GravityScale(gravity_scale),
                    CollidingEntities::default(),
                    SleepingDisabled,
                    ServerMoveState::default(),
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

pub fn handle_player_moves(
    mut receivers: Query<(Entity, &RemoteId, &mut MessageReceiver<PlayerMoveMessage>)>,
    player_map: Res<ClientPlayerMap>,
    mut players: Query<(&Player, &mut ServerMoveState, &mut PlayerMovementPlan, &GravityScale)>,
    gravity: Res<Gravity>,
) {
    let now = Instant::now();
    for (client_entity, remote_id, mut receiver) in receivers.iter_mut() {
        let client_id = remote_id.0.to_bits();
        let mut last_message: Option<PlayerMoveMessage> = None;
        for message in receiver.receive() {
            last_message = Some(message);
        }
        let Some(message) = last_message else {
            continue;
        };

        let Some(&player_entity) = player_map.0.get(&client_id) else {
            warn!("handle_player_moves: No player entity found for client_id: {} / entity: {:?}", client_id, client_entity);
            continue;
        };

        let Ok((player, mut state, mut plan, gravity_scale)) = players.get_mut(player_entity)
        else {
            continue;
        };

        let dt = state
            .last_move_at
            .map(|at| now.duration_since(at).as_secs_f32())
            .unwrap_or(0.0);
        state.last_move_at = Some(now);

        let config = MoveValidationConfig::new(
            crate::common::game::movement::HARD_SPEED_LIMIT,
            crate::common::game::movement::MAX_FALL_SPEED,
            player.jump_power,
            gravity.0.y.abs() * gravity_scale.0,
        );

        let last = state.last;
        let validated = validate_client_move(
            last.as_ref(),
            ClientMoveClaim {
                position: message.position,
                velocity: message.velocity,
                grounded: message.grounded,
                yaw: message.yaw,
                in_first_person: message.in_first_person,
            },
            dt,
            &config,
            &mut state.budget,
        );

        if !validated.accepted {
            state.violations += 1;
            if state.violations % VALIDATION_VIOLATION_LOG_THRESHOLD == 0 {
                warn!(
                    "Client {} move rejected ({} violations): claimed pos {:?} / vel {:?}, applied {:?} / {:?}",
                    client_id, state.violations, message.position, message.velocity,
                    validated.position, validated.velocity
                );
            }
        }
        state.last = Some(validated);

        plan.velocity = validated.velocity;
        plan.position_xz = Some(Vec2::new(validated.position.x, validated.position.z));
        plan.position_y = Some(validated.position.y);
        plan.grounded = validated.grounded;
        plan.rotation = server_rotation_from_move(&validated);
    }
}
fn server_rotation_from_move(move_: &ValidatedMove) -> Option<Quat> {
    if move_.in_first_person {
        Some(Quat::from_rotation_y(move_.yaw + std::f32::consts::PI))
    } else {
        let horizontal = Vec2::new(move_.velocity.x, move_.velocity.z);
        if horizontal.length() > 0.1 {
            let target_angle = horizontal.y.atan2(horizontal.x);
            Some(Quat::from_rotation_y(-target_angle + std::f32::consts::FRAC_PI_2))
        } else {
            None
        }
    }
}

pub fn apply_player_movement(
    mut players: Query<(
        &mut Position,
        &mut Rotation,
        &mut LinearVelocity,
        &mut Transform,
        &crate::common::game::movement::PlayerMovementPlan,
        &CollidingEntities,
    )>,
    mut dynamic_bodies: Query<
        (
            Entity,
            &Transform,
            &RigidBody,
            &mut LinearVelocity,
            &ComputedMass,
            &Collider,
        ),
        Without<crate::common::game::movement::PlayerMovementPlan>,
    >,
    time: Res<Time<Physics>>,
) {
    for (mut position, mut rotation, mut lin_vel, mut transform, plan, colliding) in &mut players {
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
        crate::common::game::movement::push_collided_dynamic_bodies(
            position.0,
            Vec2::new(plan.velocity.x, plan.velocity.z),
            colliding,
            &mut dynamic_bodies,
            time.delta_secs(),
        );
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
    mut pending: ResMut<crate::server::PendingAuths>,
    players_query: Query<(Entity, &Player)>,
    mut commands: Commands,
) {
    let Ok(remote_id) = query.get(trigger.entity) else { return };
    let client_id = remote_id.0.to_bits();
    info!("Client disconnected: {}", client_id);
    player_map.0.remove(&client_id);
    pending.0.remove(&client_id);
    for (player_entity, player) in &players_query {
        if player.client_id == client_id {
            commands.entity(player_entity).despawn();
        }
    }
}

pub fn sync_transforms_to_network(
    mut query: Query<(&Transform, Option<&LinearVelocity>, &mut NetworkTransform), Changed<Transform>>,
) {
    for (transform, lin_vel_opt, mut net_transform) in &mut query {
        net_transform.translation = transform.translation;
        net_transform.rotation = transform.rotation;
        net_transform.scale = transform.scale;
        net_transform.velocity = lin_vel_opt.map_or(Vec3::ZERO, |v| v.0);
    }
}

#[cfg(test)]
mod test_movement_simulation { //END MY LIFE.
    use super::*;

    pub const INPUT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(250);

    #[derive(Component)]
    pub struct PlayerInputTracker {
        pub last_input: Instant,
        pub input_timeout: std::time::Duration,
    }

    impl Default for PlayerInputTracker {
        fn default() -> Self {
            Self {
                last_input: Instant::now(),
                input_timeout: INPUT_TIMEOUT,
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

    pub fn tick_player_movement(
        mut players: Query<(
            Entity,
            &Player,
            &Collider,
            &Position,
            &Rotation,
            &LinearVelocity,
            &GravityScale,
            &PlayerInputTracker,
            &mut PlayerMovementState,
            &mut crate::common::game::movement::PlayerMovementPlan,
        )>,
        support_velocities: Query<&LinearVelocity>,
        move_and_slide: MoveAndSlide,
        gravity: Res<Gravity>,
        time: Res<Time>,
    ) {
        use crate::common::game::movement::*;

        let dt = time.delta_secs().min(0.1);
        let elapsed = time.elapsed_secs();
        let now = Instant::now();

        for (player_entity, player, collider, position, rotation, lin_vel, gravity_scale, tracker, mut state, mut plan) in &mut players {
            let has_input = now.duration_since(tracker.last_input) <= tracker.input_timeout;
            let wish_direction = if has_input { state.wish_direction } else { Vec2::ZERO };
            let jump_held = has_input && state.jump_held;

            let support_velocity = state
                .movement
                .support
                .and_then(|support| support_velocities.get(support).ok())
                .map(|velocity| velocity.0);

            let filter = SpatialQueryFilter::default()
                .with_excluded_entities([player_entity])
                .with_mask(0b0011);

            let params = CharacterMoveParams {
                wish_direction,
                jump_held,
                speed: player.speed,
                jump_power: player.jump_power,
                gravity_y: gravity.0.y,
                gravity_scale: gravity_scale.0,
                dt,
                elapsed,
            };
            let yaw = rotation.0.to_euler(EulerRot::YXZ).0;
            let output = character_move(
                position.0,
                lin_vel.0,
                yaw,
                &mut state.movement,
                &params,
                collider,
                &move_and_slide,
                &filter,
                support_velocity,
            );

            let has_wish = wish_direction.length_squared() > 0.001;
            let rotation = if state.in_first_person {
                Some(Quat::from_rotation_y(state.yaw + std::f32::consts::PI))
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
            plan.grounded = output.grounded;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_movement_simulation::*;
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
                RigidBody::Kinematic,
                Collider::cuboid(2.0 * 0.28, 5.0 * 0.28, 1.17 * 0.28),
                CollisionLayers::from_bits(0b0010, 0b0011),
                LockedAxes::ROTATION_LOCKED,
                CustomPositionIntegration,
                Friction::new(0.0),
                Restitution::new(0.0),
                GravityScale(1.0),
                CollidingEntities::default(),
                SleepingDisabled,
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
    fn diagnostic_sweep_math() {
        fn intersect_sweep(aabb_min: bevy::math::Vec3A, aabb_max: bevy::math::Vec3A, sweep: bevy::math::Vec3A, probe_center: bevy::math::Vec3A, probe_half: bevy::math::Vec3A, tmin_sweep: f32) -> f32 {
            let shift = -probe_center;
            let margin = probe_half + bevy::math::Vec3A::splat(tmin_sweep);
            let msum_min = aabb_min + shift - margin;
            let msum_max = aabb_max + shift + margin;
            let inv = bevy::math::Vec3A::new(
                if sweep.x.abs() <= f32::EPSILON { 0.0 } else { 1.0 / sweep.x },
                if sweep.y.abs() <= f32::EPSILON { 0.0 } else { 1.0 / sweep.y },
                if sweep.z.abs() <= f32::EPSILON { 0.0 } else { 1.0 / sweep.z },
            );
            let t1 = msum_min * inv;
            let t2 = msum_max * inv;
            let tmin = t1.min(t2);
            let tmax = t1.max(t2);
            let tmin_n = tmin.max_element();
            let tmax_n = tmax.min_element();
            if tmax_n >= tmin_n && tmax_n >= 0.0 {
                tmin_n
            } else {
                f32::INFINITY
            }
        }

        let probe_center = bevy::math::Vec3A::new(0.0, 0.099985, -11.6);
        let probe_half = bevy::math::Vec3A::new(0.252, 0.02, 0.1475);
        let sweep_dir = bevy::math::Vec3A::new(0.0, -1.0, 0.0);
        let floor = intersect_sweep(
            bevy::math::Vec3A::new(-20.0, -0.28, -20.0),
            bevy::math::Vec3A::new(20.0, 0.0, 20.0),
            sweep_dir,
            probe_center,
            probe_half,
            0.0,
        );
        println!("floor intersect_sweep = {:?}", floor);
        let player = intersect_sweep(
            bevy::math::Vec3A::new(-0.28, -0.000015, -11.764),
            bevy::math::Vec3A::new(0.28, 1.399985, -11.436),
            sweep_dir,
            probe_center,
            probe_half,
            0.0,
        );
        println!("player intersect_sweep = {:?}", player);
    }

    #[test]
    fn diagnostic_probe_against_embedded_floor() {
        let mut app = test_app();
        spawn_floor(&mut app);
        app.world_mut().spawn((
            Name::new("Part0"),
            Transform::from_xyz(0.0, 0.14, 0.0),
            RigidBody::Dynamic,
            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
            Friction::new(0.3),
            Restitution::new(0.3),
            GravityScale(1.0),
            Mass(1.0),
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
            SleepingDisabled,
        ));
        let player = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.7, -4.0)),
                RigidBody::Dynamic,
                Collider::cuboid(2.0 * 0.28, 5.0 * 0.28, 1.17 * 0.28),
                CollisionLayers::from_bits(0b0010, 0b0011),
                Friction::new(0.0),
                Restitution::new(0.0),
                SleepingDisabled,
                SpeculativeMargin(0.0),
                SweptCcd::default(),
            ))
            .id();

        for probe_y in [0.69, 0.692, 0.6925, 0.695, 0.699, 0.699985, 0.7, 0.71, 0.75, 0.85] {
            app.update();
            let target_y = probe_y;
            app.world_mut().run_system_once(
                move |spatial: avian3d::prelude::SpatialQuery| {
                    let center = Vec3::new(0.0, target_y, -4.0);
                    let feet =
                        center.y - crate::common::game::movement::PLAYER_HALF_HEIGHT;
                    let origin = Vec3::new(center.x, feet + 0.1, center.z);
                    let probe = Collider::cuboid(
                        crate::common::game::movement::PLAYER_HALF_WIDTH * 1.8,
                        0.04,
                        crate::common::game::movement::PLAYER_HALF_DEPTH * 1.8,
                    );
                    let dir = Dir3::new(Vec3::NEG_Y).unwrap();
                    let mut filter = avian3d::prelude::SpatialQueryFilter::default();
                    filter.mask = avian3d::prelude::LayerMask(0b0011);
                    filter.excluded_entities = std::iter::once(player).collect();
                    let hit = spatial.cast_shape(
                        &probe,
                        origin,
                        Quat::from_rotation_y(std::f32::consts::PI),
                        dir,
                        &ShapeCastConfig::from_max_distance(0.25),
                        &filter,
                    );
                    println!(
                        "center_y={:.4} feet={:.4} probe_origin={:.4} hit_distance={:?}",
                        center.y,
                        feet,
                        origin.y,
                        hit.map(|h| (h.distance, h.normal1))
                    );
                },
            );
        }
    }

    #[test]
    fn diagnostic_jump_over_dynamic_brick() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        app.world_mut().spawn((
            Name::new("Part0"),
            Transform::from_xyz(0.0, 0.14, 0.0),
            RigidBody::Dynamic,
            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
            Friction::new(0.3),
            Restitution::new(0.3),
            GravityScale(1.0),
            Mass(1.0),
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
            SleepingDisabled,
        ));
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.7, 2.0));

        #[derive(Resource)]
        struct MovementPhaseLog(Vec<(u32, f32, f32)>);
        app.insert_resource(MovementPhaseLog(Vec::new()));
        app.add_systems(
            FixedPostUpdate,
            (
                |mut log: ResMut<MovementPhaseLog>,
                 mut frames: Local<u32>,
                 q: Query<(&Position, &LinearVelocity, &Name)>| {
                    for (pos, vel, name) in &q {
                        if name.as_str() == "TestPlayer" {
                            log.0.push((*frames, pos.0.y, vel.0.y));
                        }
                    }
                    *frames += 1;
                },
            )
                .before(tick_player_movement)
                .before(apply_player_movement),
        );

        app.add_systems(
            Update,
            |mut q: Query<(&mut PlayerInputTracker, &mut PlayerMovementState)>, mut frames: Local<u32>| {
                for (mut tracker, mut state) in &mut q {
                    tracker.last_input = std::time::Instant::now();
                    state.wish_direction = Vec2::new(0.0, -1.0);
                    state.jump_held = *frames >= 20 && *frames < 25;
                }
                *frames += 1;
            },
        );

        let mut post_log: Vec<(u32, f32, f32, f32, bool)> = Vec::new();
        for tick in 0..300 {
            app.update();
            let transform = app
                .world_mut()
                .query::<(&Transform, &LinearVelocity, &Name)>()
                .iter(&app.world())
                .filter(|(t, _, n)| n.as_str() == "TestPlayer")
                .next()
                .map(|(t, v, _)| (t.translation, v.0))
                .unwrap();
            let plan = app
                .world_mut()
                .query::<(&crate::common::game::movement::PlayerMovementPlan, &Position, &Name)>()
                .iter(&app.world())
                .filter(|(_, _, n)| n.as_str() == "TestPlayer")
                .next()
                .map(|(p, pos, _)| (p.velocity.y, p.grounded, pos.0.y))
                .unwrap();
            post_log.push((
                tick as u32,
                transform.0.y,
                transform.1.y,
                plan.0,
                plan.1,
            ));
        }
        let phase_log = app.world_mut().resource::<MovementPhaseLog>().0.clone();
        println!(
            "LOG length={} first={:?} last={:?}",
            phase_log.len(),
            phase_log.first(),
            phase_log.last()
        );
        for tick in [50, 51, 52, 53, 54, 55, 56, 57, 178, 179, 180, 181, 182, 183, 184, 185] {
            let phase = phase_log
                .iter()
                .find(|(f, _, _)| *f == tick)
                .map(|(_, y, vy)| (*y, *vy))
                .unwrap_or((f32::NAN, f32::NAN));
            let post = post_log
                .iter()
                .find(|(f, _, _, _, _)| *f == tick)
                .map(|(_, py, pvy, plan_y, grounded)| (*py, *pvy, *plan_y, *grounded))
                .unwrap_or((f32::NAN, f32::NAN, f32::NAN, false));
            println!(
                "TICK {:3} movement_phase_pos_y={:.6} movement_phase_vel_y={:.4} | post: pos_y={:.6} vel_y={:.4} plan_y={:.4} grounded={}",
                tick, phase.0, phase.1, post.0, post.1, post.2, post.3
            );
        }
        let _ = player;
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
    fn player_cuboid_steps_up_a_step_diagonally() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.14, -4.0),
            RigidBody::Static,
            Collider::cuboid(8.0, 0.28, 4.0),
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
        ));
        let player = spawn_player(&mut app, Vec3::new(0.5, 0.7, -1.5));

        let mut state_query = app.world_mut().query::<&mut PlayerMovementState>();
        let mut state = state_query.get_mut(app.world_mut(), player).unwrap();
        state.wish_direction = Vec2::new(0.7071, -0.7071);

        app.add_systems(
            Update,
            |mut q: Query<(&mut PlayerInputTracker, &mut PlayerMovementState)>, mut frames: Local<u32>| {
                for (mut tracker, mut state) in &mut q {
                    tracker.last_input = std::time::Instant::now();
                    if *frames < 35 {
                        state.wish_direction = Vec2::new(0.7071, -0.7071);
                    } else {
                        state.wish_direction = Vec2::ZERO;
                    }
                }
                *frames += 1;
            },
        );

        for _ in 0..80 {
            app.update();
        }

        let transform = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap();
        assert!(
            transform.translation.y > 0.9,
            "player did not step up the step while moving diagonally, y: {}",
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
    fn player_pushes_dynamic_brick() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        let brick = app
            .world_mut()
            .spawn((
                Name::new("PushBrick"),
                Transform::from_xyz(0.0, 0.28, -2.5),
                RigidBody::Dynamic,
                Collider::cuboid(1.0 * 0.28, 2.0 * 0.28, 1.0 * 0.28),
                Friction::new(0.3),
                Restitution::new(0.0),
                GravityScale(1.0),
                Mass(1.0),
                CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                SleepingDisabled,
            ))
            .id();
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.7, -1.0));

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

        let brick_pos = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), brick)
            .unwrap()
            .translation;
        let player_pos = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap()
            .translation;
        assert!(
            brick_pos.z < -5.0,
            "brick was pushed too slowly, brick z: {}",
            brick_pos.z
        );
        assert!(
            player_pos.z < -2.0,
            "player did not advance while pushing, player z: {}",
            player_pos.z
        );
    }

    #[test]
    fn player_pushes_brick_along_movement_direction_not_center_line() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        let brick = app
            .world_mut()
            .spawn((
                Name::new("PushBrick"),
                Transform::from_xyz(0.4, 0.28, -2.5),
                RigidBody::Dynamic,
                Collider::cuboid(1.0 * 0.28, 2.0 * 0.28, 1.0 * 0.28),
                Friction::new(0.3),
                Restitution::new(0.0),
                GravityScale(1.0),
                Mass(1.0),
                CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                SleepingDisabled,
            ))
            .id();
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.7, -1.0));

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

        let brick_pos = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), brick)
            .unwrap()
            .translation;
        assert!(
            brick_pos.z < -4.0,
            "brick was pushed too slowly, brick z: {}",
            brick_pos.z
        );
        assert!(
            (brick_pos.x - 0.4).abs() < 0.1,
            "brick drifted diagonally, brick x: {}",
            brick_pos.x
        );
        let _ = player;
    }

    #[test]
    fn player_steps_onto_dynamic_brick_without_pushing_it() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        let brick = app
            .world_mut()
            .spawn((
                Name::new("StepBrick"),
                Transform::from_xyz(0.0, 0.14, -2.5),
                RigidBody::Dynamic,
                Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
                Friction::new(0.3),
                Restitution::new(0.0),
                GravityScale(1.0),
                Mass(1.0),
                CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                SleepingDisabled,
            ))
            .id();
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.7, -1.0));

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

        let brick_pos = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), brick)
            .unwrap()
            .translation;
        let player_pos = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap()
            .translation;
        let player_vel = app
            .world_mut()
            .query::<&LinearVelocity>()
            .get(app.world(), player)
            .unwrap()
            .0;
        assert!(
            (brick_pos.z - (-2.5)).abs() < 0.1,
            "brick was pushed when stepped on, brick z: {}",
            brick_pos.z
        );
        assert!(
            player_pos.z < -3.0,
            "player did not walk across the brick, player z: {}",
            player_pos.z
        );
        assert!(
            player_vel.length() < 5.5,
            "player is gliding on the brick, speed: {:.2}",
            player_vel.length()
        );
    }

    #[test]
    fn player_standing_on_dynamic_brick_does_not_glide() {
        let mut app = test_app();
        register_movement(&mut app);
        spawn_floor(&mut app);
        let brick = app
            .world_mut()
            .spawn((
                Name::new("StandBrick"),
                Transform::from_xyz(0.0, 0.14, 0.0),
                RigidBody::Dynamic,
                Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
                Friction::new(0.3),
                Restitution::new(0.0),
                GravityScale(1.0),
                Mass(1.0),
                CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                SleepingDisabled,
            ))
            .id();
        let player = spawn_player(&mut app, Vec3::new(0.0, 0.98, 0.0));

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

        let brick_pos = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), brick)
            .unwrap()
            .translation;
        let player_pos = app
            .world_mut()
            .query::<&Transform>()
            .get(app.world(), player)
            .unwrap()
            .translation;
        assert!(
            brick_pos.x.abs() < 0.05 && brick_pos.z.abs() < 0.05,
            "brick moved while player stood on it, brick pos: {:?}",
            brick_pos
        );
        assert!(
            player_pos.x.abs() < 0.05 && player_pos.z.abs() < 0.05,
            "player glided while standing on the brick, player pos: {:?}",
            player_pos
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