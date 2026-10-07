pub mod cursor;
pub mod player;
pub mod sky;
pub mod ui;
pub mod uri;
use crate::common::game::bricks::components::{Brick, BrickShapeComponent, BrickStuds};
use crate::common::game::bricks::studs;
use crate::common::game::bricks::studs::{StudsAssets, StudsExtension};
use crate::common::game::physics::PhysicsSimulationState;
use crate::common::net::components::NetworkTransform;
use avian3d::prelude::*;
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::change_detection::Ref;
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use lightyear::prelude::*;

#[derive(Resource)]
pub struct ClientUkey(pub String);

#[derive(Resource)]
pub struct LocalClientId(pub u64);

#[derive(Component)]
pub struct LocalPlayer;

#[derive(Component)]
pub struct UniqueLocalMaterial;

#[derive(Component)]
struct ClientPhysicsInitializer;

#[derive(Component)]
pub struct StartupCamera;

#[derive(Component)]
pub struct NeedsCharacterVisuals;

#[derive(Component)]
pub struct CharacterVisualsSpawned;

#[derive(Component)]
pub struct PlayerVisualChild {
    pub parent: Entity,
}

#[derive(Resource, Default)]
pub struct PlaytestState {
    pub active: bool,
}

#[derive(Resource, Default)]
struct StudioPlaytestPhysicsState {
    previous: Option<(PhysicsSimulationState, bool)>,
}

#[derive(Component)]
pub struct HelloSent;

fn app_mode() -> &'static str {
    static APP_MODE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    APP_MODE
        .get_or_init(|| std::env::var("VERTIGO_APP").unwrap_or_default())
        .as_str()
}

pub fn is_playtesting(playtest: Option<Res<PlaytestState>>) -> bool {
    if app_mode() == "client" {
        return true;
    }
    playtest.map_or(false, |p| p.active)
}

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_egui::EguiPlugin>() {
            app.add_plugins(bevy_egui::EguiPlugin::default());
        }

        app.init_resource::<ui::ChatboxState>()
            .init_resource::<ui::chat_container::ChatContState>()
            .init_resource::<ui::chat_container::ChatPanelState>()
            .init_resource::<ui::chat_container::LocalUsername>()
            .init_resource::<ui::ScoreboardPanelState>()
            .init_resource::<PlaytestState>()
            .init_resource::<StudioPlaytestPhysicsState>()
            .init_resource::<LocalPredictionState>()
            .init_resource::<ReplicationStats>()
            .add_plugins(player::PlayerPlugin)
            .add_plugins(sky::SkyPlugin)
            .add_plugins(crate::common::net::ProtocolPlugin)
            .add_systems(Startup, (setup_physics_initializer, setup_player_assets))
            .add_systems(
                PreUpdate,
                (initialize_client_physics, sync_studio_playtest_physics),
            )
            .add_systems(
                FixedPostUpdate,
                (predict_local_player_transform, apply_local_player_movement)
                    .chain()
                    .in_set(PhysicsSystems::Prepare)
                    .after(avian3d::physics_transform::PhysicsTransformSystems::TransformToPosition)
                    .run_if(is_playtesting),
            )
            .add_systems(
                Update,
                (
                    sync_network_transforms_to_client,
                    update_replication_stats,
                    sync_predicted_interpolated_transforms,
                    interpolate_remote_transforms.after(sync_predicted_interpolated_transforms),
                    sync_brick_color_to_material,
                    send_player_moves,
                    sync_local_player,
                    sync_local_player_properties.after(sync_local_player),
                    attach_character_visuals.after(sync_local_player),
                    update_local_player_transparency,
                    hide_confirmed_player_visuals.after(update_local_player_transparency),
                    update_avatar_visual_lod,
                    send_hello_message,
                    handle_kick_message,
                    handle_auth_success,
                    handle_chat_broadcast,
                )
                    .run_if(is_playtesting),
            )
            .add_systems(Update, sync_brick_studs_to_material)
            .add_systems(Update, cleanup_orphaned_visuals)
            .add_systems(Update, reset_playtest_transient_state)
            .add_systems(
                PostUpdate,
                cursor::apply_client_cursor
                    .after(bevy_egui::EguiPostUpdateSet::ProcessOutput),
            );
        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            (debug_cameras, debug_players).run_if(is_playtesting),
        );
        app.add_systems(
            bevy_egui::EguiPrimaryContextPass,
            (
                ui::configure_client_visuals,
                ui::draw_scoreboard,
                ui::draw_chatbox,
                ui::draw_health_bar,
                ui::draw_chat_container,
            )
                .run_if(is_playtesting),
        )
        .add_observer(on_client_connected)
        .add_observer(on_player_added)
        .add_observer(on_player_removed)
        .add_observer(on_brick_added)
        .add_observer(on_network_transform_added);
    }
}

fn setup_physics_initializer(
    mut commands: Commands,
    mut egui_global_settings: ResMut<bevy_egui::EguiGlobalSettings>,
    graphics_settings: Option<Res<crate::common::core::performance::GraphicsSettings>>,
) {
    if app_mode() == "studio" {
        return;
    }

    egui_global_settings.auto_create_primary_context = false;

    commands.spawn(ClientPhysicsInitializer);
    commands.insert_resource(crate::scripting::vm::client_vm::ClientScriptVM::new());

    let startup_msaa = graphics_settings
        .as_ref()
        .map(|settings| settings.msaa.to_msaa())
        .unwrap_or(Msaa::Sample4);

    commands.spawn((
        Camera3d::default(),
        Camera::default(),
        StartupCamera,
        Transform::from_xyz(0.0, 15.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        startup_msaa,
        bevy_egui::PrimaryEguiContext,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        bevy::ui::prelude::IsDefaultUiCamera,
        startup_msaa,
    ));
}

pub fn setup_player_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(player::loader::AvatarBodyAssets::load(
        &mut meshes,
        &mut materials,
    ));
}

fn initialize_client_physics(
    mut time_physics: ResMut<Time<Physics>>,
    mut state: ResMut<PhysicsSimulationState>,
    mut commands: Commands,
    query: Query<Entity, With<ClientPhysicsInitializer>>,
) {
    if query.is_empty() {
        return;
    }
    *state = PhysicsSimulationState::Running;
    time_physics.unpause();
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn sync_network_transforms_to_client(
    mut query: Query<
        (&NetworkTransform, &mut Transform),
        (
            Without<Replicate>,
            Without<crate::common::net::components::Player>,
            Without<Brick>,
        ),
    >,
) {
    for (net_transform, mut transform) in &mut query {
        if transform.translation == net_transform.translation
            && transform.rotation == net_transform.rotation
            && transform.scale == net_transform.scale
        {
            continue;
        }
        transform.translation = net_transform.translation;
        transform.scale = net_transform.scale;
        transform.rotation = net_transform.rotation;
    }
}

const LOCAL_SERVER_SAMPLE_MAX_AGE: f64 = 1.0;
const LOCAL_SNAP_THRESHOLD: f32 = 1.5;
const REMOTE_INTERP_MIN_DELAY_SECS: f64 = 0.066;
const REMOTE_INTERP_MAX_BUFFER_SECS: f64 = 0.3;
const REMOTE_INTERP_MAX_AGE_SECS: f64 = 2.0;
#[derive(Resource)]
pub struct ReplicationStats {
    pub inter_arrival_ema: f64,
    pub rtt: std::time::Duration,
    pub jitter: std::time::Duration,
    last_change: Option<f64>,
}
impl Default for ReplicationStats {
    fn default() -> Self {
        Self {
            inter_arrival_ema: 1.0 / 30.0,
            rtt: std::time::Duration::ZERO,
            jitter: std::time::Duration::ZERO,
            last_change: None,
        }
    }
}
#[derive(Resource, Default)]
struct LocalPredictionState {
    movement: crate::common::game::movement::CharacterMoveState,
    server_sample: Option<ServerSample>,
}
#[derive(Clone, Copy)]
struct ServerSample {
    received_at: f64,
    translation: Vec3,
    velocity: Vec3,
}

#[derive(Component, Clone, Copy, Default)]
pub(crate) struct LocalInterpState {
    prev: Transform,
    curr: Transform,
    initialized: bool,
}
pub(crate) fn interpolate_local_player_transform(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(&mut Transform, &LocalInterpState), (With<LocalPlayer>, Without<Replicate>)>,
) {
    let alpha = fixed_time.overstep_fraction().clamp(0.0, 1.0);
    for (mut transform, interp) in &mut query {
        if !interp.initialized {
            continue;
        }
        transform.translation = interp.prev.translation.lerp(interp.curr.translation, alpha);
        transform.rotation = interp.prev.rotation.slerp(interp.curr.rotation, alpha);
    }
}

fn update_replication_stats(
    connected: Query<&PingManager, With<Connected>>,
    confirmed_players: Query<Ref<NetworkTransform>, With<crate::common::net::components::Player>>,
    mut stats: ResMut<ReplicationStats>,
    time: Res<Time>,
    mut last_log: Local<f32>,
) {
    let now = time.elapsed_secs_f64();
    for net_transform in &confirmed_players {
        if net_transform.is_changed() {
            if let Some(prev) = stats.last_change {
                let delta = now - prev;
                if delta > 0.001 {
                    stats.inter_arrival_ema = stats.inter_arrival_ema * 0.9 + delta * 0.1;
                }
            }
            stats.last_change = Some(now);
        }
    }

    for ping in &connected {
        stats.rtt = ping.rtt();
        stats.jitter = ping.jitter();
        if now as f32 - *last_log >= 3.0 {
            *last_log = now as f32;
            info!(
                "PING: {:.1} ms (jitter: {:.1} ms)",
                stats.rtt.as_secs_f64() * 1000.0,
                stats.jitter.as_secs_f64() * 1000.0
            );
        }
    }
}

fn snap_to_server(
    server_position: Vec3,
    output: &mut crate::common::game::movement::CharacterMoveOutput,
) {
    output.position_xz = Some(Vec2::new(server_position.x, server_position.z));
    output.position_y = Some(server_position.y);
}
pub const SNAP_SPEED_MARGIN_SECS: f32 = 0.1;

fn should_snap_to_server(
    sample: Option<&ServerSample>,
    current: Vec3,
    now: f64,
    threshold: f32,
    max_age: f64,
    rtt: std::time::Duration,
    current_speed: f32,
) -> bool {
    let Some(sample) = sample else {
        return false;
    };
    if now - sample.received_at > max_age {
        return false;
    }
    let lag = (now - sample.received_at) as f32 + rtt.as_secs_f32();
    let expected = sample.translation + sample.velocity * lag;
    let effective_threshold = threshold + current_speed.max(0.0) * SNAP_SPEED_MARGIN_SECS;
    (expected - current).length() > effective_threshold
}

fn predict_local_player_transform(
    mut prediction: ResMut<LocalPredictionState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut contexts: EguiContexts,
    camera_query: Query<&player::CameraSettings, With<player::PlayerCamera>>,
    mut local_query: Query<
        (
            Entity,
            &crate::common::net::components::Player,
            &Collider,
            Ref<NetworkTransform>,
            &Position,
            &Rotation,
            &LinearVelocity,
            &mut crate::common::game::movement::PlayerMovementPlan,
        ),
        (With<LocalPlayer>, Without<Replicate>),
    >,
    support_velocities: Query<&LinearVelocity>,
    move_and_slide: MoveAndSlide,
    time: Res<Time>,
    stats: Res<ReplicationStats>,
    mut sim_clock: Local<f64>,
) {
    use crate::common::game::movement::*;

    let Some((
        player_entity,
        player,
        collider,
        net_transform,
        position,
        rotation,
        lin_vel,
        mut plan,
    )) = local_query.iter_mut().next()
    else {
        return;
    };
    let Some(camera_settings) = camera_query.iter().next() else {
        return;
    };

    let wants_keyboard = if let Ok(ctx) = contexts.ctx_mut() {
        ctx.egui_wants_keyboard_input()
    } else {
        false
    };

    let dt = time.delta_secs().min(0.1);
    *sim_clock += dt as f64;
    let elapsed = time.elapsed_secs();

    let w = !wants_keyboard && keys.pressed(KeyCode::KeyW);
    let a = !wants_keyboard && keys.pressed(KeyCode::KeyA);
    let s = !wants_keyboard && keys.pressed(KeyCode::KeyS);
    let d = !wants_keyboard && keys.pressed(KeyCode::KeyD);
    let jump_held = !wants_keyboard && keys.pressed(KeyCode::Space);

    let rotation_y = Quat::from_rotation_y(camera_settings.yaw);
    let forward = rotation_y * Vec3::NEG_Z;
    let right = rotation_y * Vec3::X;

    let mut move_direction = Vec3::ZERO;
    if w {
        move_direction += forward;
    }
    if s {
        move_direction -= forward;
    }
    if a {
        move_direction -= right;
    }
    if d {
        move_direction += right;
    }

    let direction = if move_direction.length_squared() > 0.001 {
        move_direction.normalize()
    } else {
        Vec3::ZERO
    };
    let has_input = direction != Vec3::ZERO;

    let support_velocity = prediction
        .movement
        .support
        .and_then(|support| support_velocities.get(support).ok())
        .map(|velocity| velocity.0);

    let filter = SpatialQueryFilter::default()
        .with_excluded_entities([player_entity])
        .with_mask(0b0011);

    let params = CharacterMoveParams {
        wish_direction: Vec2::new(direction.x, direction.z),
        jump_held,
        speed: player.speed,
        jump_power: player.jump_power,
        gravity: player.gravity,
        speed_response: player.speed_response,
        dt,
        elapsed,
    };
    let yaw = rotation.0.to_euler(EulerRot::YXZ).0;
    let mut output = character_move(
        position.0,
        lin_vel.0,
        yaw,
        &mut prediction.movement,
        &params,
        collider,
        &move_and_slide,
        &filter,
        support_velocity,
    );

    let mut predicted_pos = position.0;
    if let Some(y) = output.position_y {
        predicted_pos.y = y;
    }
    if let Some(xz) = output.position_xz {
        predicted_pos.x = xz.x;
        predicted_pos.z = xz.y;
    }

    if net_transform.is_changed() {
        prediction.server_sample = Some(ServerSample {
            received_at: *sim_clock,
            translation: net_transform.translation,
            velocity: net_transform.velocity,
        });
    }

    let mut final_pos = predicted_pos;
    if let Some(y) = output.position_y {
        final_pos.y = y;
    }
    if let Some(xz) = output.position_xz {
        final_pos.x = xz.x;
        final_pos.z = xz.y;
    }

    if should_snap_to_server(
        prediction.server_sample.as_ref(),
        final_pos,
        *sim_clock,
        LOCAL_SNAP_THRESHOLD,
        LOCAL_SERVER_SAMPLE_MAX_AGE,
        stats.rtt,
        output.velocity.length(),
    ) {
        snap_to_server(
            prediction.server_sample.as_ref().unwrap().translation,
            &mut output,
        );
    }

    let in_first_person = camera_settings.current_distance <= 0.6;
    let rotation_out = if in_first_person {
        Some(Quat::from_rotation_y(
            camera_settings.yaw + std::f32::consts::PI,
        ))
    } else if has_input {
        let target_angle = direction.z.atan2(direction.x);
        let target_rotation = Quat::from_rotation_y(-target_angle + std::f32::consts::FRAC_PI_2);
        let turn_factor = 1.0 - (-TURN_SPEED * dt).exp();
        Some(rotation.0.slerp(target_rotation, turn_factor))
    } else {
        None
    };

    plan.velocity = output.velocity;
    plan.position_y = output.position_y;
    plan.position_xz = output.position_xz;
    plan.rotation = rotation_out;
    plan.grounded = output.grounded;
}

fn apply_local_player_movement(
    mut local_query: Query<
        (
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            &mut Transform,
            &crate::common::game::movement::PlayerMovementPlan,
            &CollidingEntities,
            &mut LocalInterpState,
        ),
        (With<LocalPlayer>, Without<Replicate>),
    >,
    mut dynamic_bodies: Query<
        (
            Entity,
            &Transform,
            &RigidBody,
            &mut LinearVelocity,
            &mut AngularVelocity,
            &ComputedMass,
            &Collider,
        ),
        Without<crate::common::game::movement::PlayerMovementPlan>,
    >,
    time: Res<Time<Physics>>,
) {
    for (mut position, mut rotation, mut lin_vel, mut transform, plan, colliding, mut interp) in
        &mut local_query
    {
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
        interp.prev = interp.curr;
        interp.curr = Transform {
            translation: position.0,
            rotation: plan.rotation.unwrap_or(rotation.0),
            scale: transform.scale,
        };
        interp.initialized = true;
    }
}

#[derive(Clone, Copy)]
struct RemoteSample {
    time: f64,
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

#[derive(Default)]
struct RemoteInterpBuffer {
    samples: std::collections::VecDeque<RemoteSample>,
    last_seen: f64,
}

fn sample_interpolated(buffer: &RemoteInterpBuffer, target: f64) -> Option<RemoteSample> {
    if buffer.samples.is_empty() {
        return None;
    }
    if buffer.samples.len() == 1 || target <= buffer.samples[0].time {
        return buffer.samples.front().copied();
    }
    for i in 0..buffer.samples.len() - 1 {
        let a = buffer.samples[i];
        let b = buffer.samples[i + 1];
        if a.translation.distance_squared(b.translation) > 25.0 {
            return Some(b);
        }
        if target >= a.time && target <= b.time {
            let t = if b.time > a.time {
                ((target - a.time) / (b.time - a.time)) as f32
            } else {
                1.0
            };
            return Some(RemoteSample {
                time: target,
                translation: a.translation.lerp(b.translation, t),
                rotation: a.rotation.slerp(b.rotation, t),
                scale: a.scale.lerp(b.scale, t),
            });
        }
    }
    buffer.samples.back().copied()
}

fn interpolate_remote_transforms(
    mut query: Query<
        (Entity, Ref<NetworkTransform>, &mut Transform),
        (Without<LocalPlayer>, Without<Replicate>),
    >,
    mut buffers: Local<std::collections::HashMap<Entity, RemoteInterpBuffer>>,
    stats: Res<ReplicationStats>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    let adaptive_delay = (stats.inter_arrival_ema * 2.0 + stats.rtt.as_secs_f64() * 0.5)
        .max(REMOTE_INTERP_MIN_DELAY_SECS);
    let target_time = now - adaptive_delay;
    let prune_before = now - REMOTE_INTERP_MAX_BUFFER_SECS;

    buffers.retain(|_, buffer| {
        while let Some(front) = buffer.samples.front() {
            if front.time < prune_before {
                buffer.samples.pop_front();
            } else {
                break;
            }
        }
        now - buffer.last_seen < REMOTE_INTERP_MAX_AGE_SECS
    });

    for (entity, net_transform, mut transform) in &mut query {
        let buffer = buffers.entry(entity).or_default();
        buffer.last_seen = now;
        if net_transform.is_changed() {
            buffer.samples.push_back(RemoteSample {
                time: now,
                translation: net_transform.translation,
                rotation: net_transform.rotation,
                scale: net_transform.scale,
            });
            while let Some(front) = buffer.samples.front() {
                if front.time < prune_before {
                    buffer.samples.pop_front();
                } else {
                    break;
                }
            }
        }
        let Some(sample) = sample_interpolated(buffer, target_time) else {
            continue;
        };
        if transform.translation != sample.translation
            || transform.rotation != sample.rotation
            || transform.scale != sample.scale
        {
            transform.translation = sample.translation;
            transform.rotation = sample.rotation;
            transform.scale = sample.scale;
        }
    }
}

fn send_player_moves(
    local_query: Query<
        (
            &Position,
            &LinearVelocity,
            &crate::common::game::movement::PlayerMovementPlan,
        ),
        (With<LocalPlayer>, Without<Replicate>),
    >,
    camera_query: Query<&player::CameraSettings, With<player::PlayerCamera>>,
    mut sender_query: Query<&mut MessageSender<crate::common::net::messages::PlayerMoveMessage>>,
    mut last_sent: Local<
        Option<(
            crate::common::net::messages::PlayerMoveMessage,
            std::time::Instant,
        )>,
    >,
) {
    let Some((position, lin_vel, plan)) = local_query.iter().next() else {
        return;
    };
    let Some(camera_settings) = camera_query.iter().next() else {
        return;
    };
    let Some(mut sender) = sender_query.iter_mut().next() else {
        return;
    };

    let message = crate::common::net::messages::PlayerMoveMessage {
        position: position.0,
        velocity: lin_vel.0,
        grounded: plan.grounded,
        yaw: camera_settings.yaw,
        in_first_person: camera_settings.current_distance <= 0.6,
    };

    let now = std::time::Instant::now();
    let send = match &*last_sent {
        Some((last, at)) => {
            *last != message || now.duration_since(*at) >= std::time::Duration::from_millis(100)
        }
        None => true,
    };
    if send {
        *last_sent = Some((message.clone(), now));
        let _ = sender.send::<crate::common::net::messages::InputChannel>(message);
    }
}

fn on_client_connected(
    trigger: On<Add, Connected>,
    query: Query<&LocalId>,
    mut commands: Commands,
) {
    debug!(
        "on_client_connected observer triggered for entity: {:?}",
        trigger.entity
    );
    if let Ok(local_id) = query.get(trigger.entity) {
        let client_id = local_id.0.to_bits();
        info!(
            "Client connected successfully! Mapped Local Client ID: {}",
            client_id
        );
        commands.insert_resource(LocalClientId(client_id));
    } else {
        warn!("on_client_connected failed: LocalId component missing on target entity");
    }
}

fn on_player_added(
    trigger: On<Add, crate::common::net::components::Player>,
    mut commands: Commands,
    query: Query<(
        Option<&Predicted>,
        Option<&Interpolated>,
        Option<&Replicate>,
    )>,
    player_query: Query<&crate::common::net::components::Player>,
    local_client_id: Option<Res<LocalClientId>>,
) {
    let entity = trigger.entity;
    let (pred, interp, rep) = query
        .get(entity)
        .map(|(p, i, r)| (p.is_some(), i.is_some(), r.is_some()))
        .unwrap_or((false, false, false));
    info!(
        "PLAYER ADDED OBSERVER: {:?} (predicted={}, interpolated={}, replicated={})",
        entity, pred, interp, rep
    );
    if rep {
        return;
    }
    commands.entity(entity).insert(NeedsCharacterVisuals);

    if let Ok(player) = player_query.get(entity) {
        let is_local = local_client_id.map_or(false, |local| local.0 == player.client_id);
        if !is_local {
            commands.entity(entity).insert((
                RigidBody::Static,
                Collider::cuboid(2.0 * 0.28, 5.0 * 0.28, 1.17 * 0.28),
                CollisionLayers::from_bits(0b0010, 0b0011),
            ));
        }
    }
}

fn on_player_removed(
    trigger: On<Remove, crate::common::net::components::Player>,
    mut commands: Commands,
) {
    debug!(
        "CLIENT PLAYER REMOVED: {:?}, performing recursive despawn of children",
        trigger.entity
    );
    if let Ok(mut entity_cmd) = commands.get_entity(trigger.entity) {
        entity_cmd.despawn();
    }
}

fn cleanup_orphaned_visuals(
    mut commands: Commands,
    query_visuals: Query<(Entity, &PlayerVisualChild)>,
    query_parents: Query<Entity, With<crate::common::net::components::Player>>,
) {
    for (entity, visual_child) in &query_visuals {
        if query_parents.get(visual_child.parent).is_err() {
            debug!(
                "CLIENT: Despawning orphaned player visual child {:?} as its parent has been despawned",
                entity
            );
            if let Ok(mut entity_cmd) = commands.get_entity(entity) {
                entity_cmd.despawn();
            }
        }
    }
}

fn attach_character_visuals(
    mut commands: Commands,
    character_assets: Option<Res<player::loader::AvatarBodyAssets>>,
    query: Query<
        (
            Entity,
            &crate::common::net::components::Player,
            Option<&LocalPlayer>,
        ),
        (
            With<NeedsCharacterVisuals>,
            Without<CharacterVisualsSpawned>,
            Without<Replicate>,
        ),
    >,
    local_client_id: Option<Res<LocalClientId>>,
) {
    let Some(assets) = character_assets else {
        return;
    };

    let local_id = local_client_id.map(|id| id.0);

    for (entity, player_comp, local_player_opt) in &query {
        let is_local = (local_id == Some(player_comp.client_id)) || local_player_opt.is_some();
        info!(
            "ATTACHING CHARACTER VISUALS TO entity={:?}, client_id={}, is_local={}",
            entity, player_comp.client_id, is_local
        );

        let mut root_cmd = commands.spawn((
            Transform::from_translation(Vec3::new(
                0.0,
                player::animation::AVATAR_ROOT_OFFSET_Y,
                0.0,
            ))
            .with_scale(Vec3::splat(player::animation::AVATAR_STUD_SCALE)),
            GlobalTransform::default(),
            Visibility::Inherited,
            PlayerVisualChild { parent: entity },
            player::animation::AvatarAnimState::default(),
        ));

        if is_local {
            root_cmd.insert(UniqueLocalMaterial);
        }

        let root_id = root_cmd.id();
        let mut rig_limbs = [Entity::PLACEHOLDER; 5];
        for part in &assets.parts {
            let Some(index) = player::animation::AvatarLimbKind::ALL
                .iter()
                .position(|kind| kind.part_name() == part.name)
            else {
                let part_id = commands
                    .spawn((
                        Name::new(part.name),
                        Mesh3d(part.mesh.clone()),
                        MeshMaterial3d(part.material.clone()),
                        Transform::default(),
                        GlobalTransform::default(),
                        Visibility::Inherited,
                    ))
                    .id();
                commands.entity(root_id).add_child(part_id);
                continue;
            };
            let kind = player::animation::AvatarLimbKind::ALL[index];
            let pivot = kind.pivot();
            let pivot_id = commands
                .spawn((
                    Name::new(part.name),
                    Transform::from_translation(pivot),
                    GlobalTransform::default(),
                    Visibility::Inherited,
                    player::animation::AvatarLimb {
                        kind,
                        current: Vec2::ZERO,
                    },
                ))
                .id();
            let part_id = commands
                .spawn((
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(part.material.clone()),
                    Transform::from_translation(-pivot),
                    GlobalTransform::default(),
                    Visibility::Inherited,
                ))
                .id();
            commands.entity(pivot_id).add_child(part_id);
            commands.entity(root_id).add_child(pivot_id);
            rig_limbs[index] = pivot_id;
        }
        commands.entity(root_id).insert(player::animation::AvatarRig {
            limbs: rig_limbs,
        });
        commands.entity(entity).add_child(root_id);

        commands
            .entity(entity)
            .remove::<NeedsCharacterVisuals>()
            .insert(CharacterVisualsSpawned);
    }
}

fn sync_local_player(
    mut commands: Commands,
    query: Query<
        (Entity, &crate::common::net::components::Player),
        (Without<LocalPlayer>, Without<Replicate>),
    >,
    local_client_id: Option<Res<LocalClientId>>,
    startup_cameras: Query<Entity, With<StartupCamera>>,
    graphics_settings: Option<Res<crate::common::core::performance::GraphicsSettings>>,
    gravity: Res<Gravity>,
) {
    let Some(local_id) = local_client_id else {
        return;
    };
    let local_client_id = local_id.0;
    for (entity, player) in &query {
        trace!(
            "sync_local_player checking entity={:?}, player client_id={}, expected client_id={}",
            entity, player.client_id, local_client_id
        );
        if player.client_id == local_client_id {
            debug!(
                "Local player match verified! Inserting LocalPlayer and spawning camera on entity: {:?}",
                entity
            );
            commands.entity(entity).insert(LocalPlayer);
            commands.entity(entity).insert((
                RigidBody::Kinematic,
                Collider::cuboid(2.0 * 0.28, 5.0 * 0.28, 1.17 * 0.28),
                CollisionLayers::from_bits(0b0010, 0b0011),
                LockedAxes::ROTATION_LOCKED,
                CustomPositionIntegration,
                Friction::new(player.friction),
                Restitution::new(player.bounciness),
                GravityScale(crate::server::player::gravity_scale_for_avian(
                    player.gravity,
                    gravity.0.y.abs(),
                )),
                CollidingEntities::default(),
                SleepingDisabled,
                crate::common::game::movement::PlayerMovementPlan::default(),
                TransformInterpolation,
                LocalInterpState::default(),
            ));

            for camera_entity in &startup_cameras {
                commands.entity(camera_entity).despawn();
            }

            let player_msaa = graphics_settings
                .as_ref()
                .map(|settings| settings.msaa.to_msaa())
                .unwrap_or(Msaa::Sample4);
            let mut cam_cmd = commands.spawn((
                Camera3d::default(),
                Camera::default(),
                Projection::Perspective(PerspectiveProjection {
                    far: 3000.0,
                    fov: 70.0f32.to_radians(),
                    ..default()
                }),
                player::PlayerCamera,
                bevy::audio::SpatialListener::default(),
                player::CameraSettings {
                    yaw: 0.0,
                    pitch: -0.35,
                    distance: 4.5,
                    current_distance: 4.5,
                    target_offset: Vec3::new(0.0, 0.55, 0.0),
                },
                Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
                player_msaa,
            ));

            if app_mode() == "client" {
                cam_cmd.insert(bevy_egui::PrimaryEguiContext);
            }
        }
    }
}

fn sync_local_player_properties(
    gravity: Res<Gravity>,
    mut query: Query<
        (
            &crate::common::net::components::Player,
            &mut GravityScale,
            &mut Friction,
            &mut Restitution,
        ),
        (With<LocalPlayer>, Without<Replicate>),
    >,
) {
    let world_gravity_mag = gravity.0.y.abs();
    for (player, mut gravity_scale, mut friction, mut restitution) in &mut query {
        let new_gravity_scale = GravityScale(crate::server::player::gravity_scale_for_avian(
            player.gravity,
            world_gravity_mag,
        ));
        if *gravity_scale != new_gravity_scale {
            *gravity_scale = new_gravity_scale;
        }
        let new_friction = Friction::new(player.friction);
        if *friction != new_friction {
            *friction = new_friction;
        }
        let new_restitution = Restitution::new(player.bounciness);
        if *restitution != new_restitution {
            *restitution = new_restitution;
        }
    }
}

fn update_local_player_transparency(
    camera_query: Query<&player::CameraSettings, With<player::PlayerCamera>>,
    local_player_query: Query<(&Transform, &Children), With<LocalPlayer>>,
    child_query: Query<Entity, With<UniqueLocalMaterial>>,
    mut visibility_query: Query<&mut Visibility>,
) {
    let Some(camera_settings) = camera_query.iter().next() else {
        return;
    };
    let Some((_player_transform, children)) = local_player_query.iter().next() else {
        return;
    };

    let show = camera_settings.distance > 0.6;

    for child in children.iter() {
        if let Ok(child_entity) = child_query.get(child) {
            if let Ok(mut visibility) = visibility_query.get_mut(child_entity) {
                let target = if show {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != target {
                    *visibility = target;
                }
            }
        }
    }
}

fn on_brick_added(
    trigger: On<Add, Brick>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
    mut plain_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>,
    >,
    studs_assets: Res<StudsAssets>,
    name_query: Query<&Name>,
    shape_query: Query<&BrickShapeComponent>,
    transform_query: Query<&Transform>,
    color_query: Query<&crate::common::game::bricks::components::BrickColor>,
    studs_query: Query<&BrickStuds>,
    workspace_studs: Option<Res<crate::common::game::bricks::WorkspaceShowStuds>>,
    mut cache: ResMut<crate::common::game::bricks::BrickMaterialCache>,
) {
    let entity = trigger.entity;
    trace!("Brick added to scene: {:?}", entity);
    let shape = shape_query
        .get(entity)
        .map(|s| s.shape)
        .unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
    let scale = transform_query
        .get(entity)
        .map(|t| t.scale)
        .unwrap_or(Vec3::ONE);

    let mesh_handle = match shape {
        crate::common::game::bricks::components::BrickShape::Block => {
            crate::common::game::bricks::block_mesh_for_scale(&mut cache, &mut meshes, scale)
        }
        crate::common::game::bricks::components::BrickShape::Sphere => {
            if cache.sphere_mesh.is_none() {
                cache.sphere_mesh = Some(meshes.add(Sphere::new(1.0 * 0.28)));
            }
            cache.sphere_mesh.clone().unwrap()
        }
        crate::common::game::bricks::components::BrickShape::Cylinder => {
            if cache.cylinder_mesh.is_none() {
                cache.cylinder_mesh = Some(
                    meshes.add(crate::common::game::bricks::cylinder_brick_mesh()),
                );
            }
            cache.cylinder_mesh.clone().unwrap()
        }
        crate::common::game::bricks::components::BrickShape::Wedge => {
            if cache.wedge_mesh.is_none() {
                cache.wedge_mesh =
                    Some(meshes.add(crate::common::game::bricks::wedge_brick_mesh()));
            }
            cache.wedge_mesh.clone().unwrap()
        }
        crate::common::game::bricks::components::BrickShape::CornerWedge => {
            if cache.corner_wedge_mesh.is_none() {
                cache.corner_wedge_mesh = Some(
                    meshes.add(crate::common::game::bricks::corner_wedge_brick_mesh()),
                );
            }
            cache.corner_wedge_mesh.clone().unwrap()
        }
    };

    let base_color = if let Ok(brick_color) = color_query.get(entity) {
        brick_color.color
    } else {
        let name_opt = name_query.get(entity).ok().map(|n| n.as_str());
        if name_opt == Some("Baseplate") {
            Color::srgb(0.18, 0.38, 0.18)
        } else {
            Color::srgb(0.84, 0.24, 0.16)
        }
    };

    let show_studs = studs_query.get(entity).map(|s| s.enabled).unwrap_or(true)
        && workspace_studs.as_ref().map(|w| w.enabled).unwrap_or(true);

    commands.entity(entity).insert(Mesh3d(mesh_handle));
    commands
        .entity(entity)
        .insert(crate::common::game::bricks::components::BrickMeshKey {
            shape,
            scale_key: crate::common::game::bricks::brick_scale_key(scale),
        });
    crate::common::game::bricks::swap_brick_material(
        &mut commands,
        entity,
        show_studs,
        &mut cache,
        &mut studs_materials,
        &mut plain_materials,
        &studs_assets,
        base_color,
    );
}

fn sync_brick_studs_to_material(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            Ref<BrickStuds>,
            &crate::common::game::bricks::components::BrickColor,
            Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
            Option<
                &MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>,
            >,
        ),
        With<Brick>,
    >,
    workspace_studs: Option<Res<crate::common::game::bricks::WorkspaceShowStuds>>,
    mut cache: ResMut<crate::common::game::bricks::BrickMaterialCache>,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
    mut plain_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>,
    >,
    studs_assets: Option<Res<StudsAssets>>,
) {
    let Some(studs_assets) = studs_assets else {
        return;
    };
    let show_studs_globally = workspace_studs
        .as_ref()
        .map(|w| w.enabled)
        .unwrap_or(true);
    let workspace_changed = workspace_studs
        .as_ref()
        .map(|w| w.is_changed())
        .unwrap_or(false);
    for (entity, studs, brick_color, studs_material, _plain_material) in &query {
        if !workspace_changed && !studs.is_changed() {
            continue;
        }
        let want_studs = show_studs_globally && studs.enabled;
        if want_studs == studs_material.is_some() {
            continue;
        }

        crate::common::game::bricks::swap_brick_material(
            &mut commands,
            entity,
            want_studs,
            &mut cache,
            &mut studs_materials,
            &mut plain_materials,
            &studs_assets,
            brick_color.color,
        );
    }
}

fn on_network_transform_added(
    trigger: On<Add, NetworkTransform>,
    mut commands: Commands,
    query: Query<&NetworkTransform>,
) {
    let entity = trigger.entity;
    let Ok(net_transform) = query.get(entity) else {
        return;
    };
    commands.entity(entity).insert((
        Transform {
            translation: net_transform.translation,
            rotation: net_transform.rotation,
            scale: net_transform.scale,
        },
        GlobalTransform::default(),
        Visibility::Inherited,
    ));
}

fn index_confirmed_transforms<'a, T>(
    index: &mut std::collections::HashMap<u64, Transform>,
    transforms: impl Iterator<Item = (&'a crate::common::net::components::Player, T)>,
) where
    T: std::ops::Deref<Target = Transform>,
{
    index.clear();
    for (player, transform) in transforms {
        index.entry(player.client_id).or_insert(*transform);
    }
}

fn update_studio_playtest_physics(
    active: bool,
    time_physics: &mut Time<Physics>,
    state: &mut PhysicsSimulationState,
    playtest_physics: &mut StudioPlaytestPhysicsState,
) {
    if active {
        if playtest_physics.previous.is_none() {
            playtest_physics.previous = Some((*state, time_physics.is_paused()));
        }
        *state = PhysicsSimulationState::Running;
        time_physics.unpause();
    } else if let Some((previous_state, was_paused)) = playtest_physics.previous.take() {
        *state = previous_state;
        if was_paused {
            time_physics.pause();
        } else {
            time_physics.unpause();
        }
    }
}

fn sync_studio_playtest_physics(
    playtest: Res<PlaytestState>,
    mut time_physics: ResMut<Time<Physics>>,
    mut state: ResMut<PhysicsSimulationState>,
    mut playtest_physics: ResMut<StudioPlaytestPhysicsState>,
) {
    if app_mode() != "studio" {
        return;
    }
    update_studio_playtest_physics(
        playtest.active,
        &mut time_physics,
        &mut state,
        &mut playtest_physics,
    );
}

fn reset_playtest_transient_state(
    playtest: Option<Res<PlaytestState>>,
    mut prediction: ResMut<LocalPredictionState>,
    mut stats: ResMut<ReplicationStats>,
    mut chat: ResMut<ui::chat_container::ChatContState>,
    mut chatbox: ResMut<ui::ChatboxState>,
    mut local_username: ResMut<ui::chat_container::LocalUsername>,
) {
    if is_playtesting(playtest) {
        return;
    }
    if prediction.server_sample.is_some() {
        *prediction = LocalPredictionState::default();
    }
    if stats.last_change.is_some() {
        *stats = ReplicationStats::default();
    }
    if !chat.messages.is_empty() {
        chat.messages.clear();
    }
    if !chatbox.text.is_empty() || chatbox.cooldown_until != 0.0 {
        *chatbox = ui::ChatboxState::default();
    }
    if !local_username.0.is_empty() {
        local_username.0.clear();
    }
}

fn sync_predicted_interpolated_transforms(
    mut predicted_interpolated_query: Query<
        (&crate::common::net::components::Player, &mut Transform),
        (
            Or<(With<Predicted>, With<Interpolated>)>,
            Without<LocalPlayer>,
        ),
    >,
    confirmed_query: Query<
        (&crate::common::net::components::Player, Ref<Transform>),
        (
            Without<Predicted>,
            Without<Interpolated>,
            Without<Replicate>,
        ),
    >,
    mut confirmed_transforms: Local<std::collections::HashMap<u64, Transform>>,
    mut last_confirmed_count: Local<usize>,
) {
    let mut changed = false;
    let mut count = 0;
    for (_player, transform) in &confirmed_query {
        count += 1;
        changed |= transform.is_changed();
    }
    if changed || count != *last_confirmed_count {
        *last_confirmed_count = count;
        index_confirmed_transforms(&mut confirmed_transforms, confirmed_query.iter());
    }
    for (player, mut transform) in &mut predicted_interpolated_query {
        if let Some(confirmed) = confirmed_transforms.get(&player.client_id) {
            if *transform != *confirmed {
                *transform = *confirmed;
            }
        }
    }
}

fn sync_brick_color_to_material(
    mut commands: Commands,
    query: Query<
        (Entity, &crate::common::game::bricks::components::BrickColor),
        Changed<crate::common::game::bricks::components::BrickColor>,
    >,
    studs_query: Query<&BrickStuds>,
    workspace_studs: Option<Res<crate::common::game::bricks::WorkspaceShowStuds>>,
    mut cache: ResMut<crate::common::game::bricks::BrickMaterialCache>,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
    mut plain_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>,
    >,
    studs_assets: Option<Res<StudsAssets>>,
) {
    let Some(studs_assets) = studs_assets else {
        return;
    };
    let show_studs_globally = workspace_studs
        .as_ref()
        .map(|w| w.enabled)
        .unwrap_or(true);
    for (entity, brick_color) in &query {
        let base_color = brick_color.color;
        let show_studs =
            show_studs_globally && studs_query.get(entity).map(|s| s.enabled).unwrap_or(true);

        if show_studs {
            crate::common::game::bricks::swap_brick_material(
                &mut commands,
                entity,
                true,
                &mut cache,
                &mut studs_materials,
                &mut plain_materials,
                &studs_assets,
                base_color,
            );
        } else {
            crate::common::game::bricks::swap_brick_material(
                &mut commands,
                entity,
                false,
                &mut cache,
                &mut studs_materials,
                &mut plain_materials,
                &studs_assets,
                base_color,
            );
        }
    }
}

fn hide_confirmed_player_visuals(
    predicted_interpolated_query: Query<
        &crate::common::net::components::Player,
        Or<(With<Predicted>, With<Interpolated>)>,
    >,
    mut confirmed_query: Query<
        (&crate::common::net::components::Player, &mut Visibility),
        (
            Without<Predicted>,
            Without<Interpolated>,
            Without<Replicate>,
        ),
    >,
    mut cached_ids: Local<std::collections::HashSet<u64>>,
) {
    cached_ids.clear();
    for player in predicted_interpolated_query.iter() {
        cached_ids.insert(player.client_id);
    }

    for (conf_player, mut visibility) in &mut confirmed_query {
        if cached_ids.contains(&conf_player.client_id) {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        } else {
            if *visibility != Visibility::Inherited {
                *visibility = Visibility::Inherited;
            }
        }
    }
}

const REMOTE_AVATAR_HIDE_DISTANCE: f32 = 160.0;

fn update_avatar_visual_lod(
    camera_query: Query<&GlobalTransform, With<player::PlayerCamera>>,
    local_player_query: Query<
        (),
        (
            With<LocalPlayer>,
            With<crate::common::net::components::Player>,
        ),
    >,
    player_query: Query<&GlobalTransform, With<crate::common::net::components::Player>>,
    mut visual_query: Query<(&PlayerVisualChild, &mut Visibility), Without<UniqueLocalMaterial>>,
) {
    let Some(camera_transform) = camera_query.iter().next() else {
        return;
    };
    for (visual_child, mut visibility) in &mut visual_query {
        if local_player_query.get(visual_child.parent).is_ok() {
            continue;
        }
        let Ok(player_transform) = player_query.get(visual_child.parent) else {
            continue;
        };
        let distance = camera_transform
            .translation()
            .distance(player_transform.translation());
        let target = if distance > REMOTE_AVATAR_HIDE_DISTANCE {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != target {
            *visibility = target;
        }
    }
}

fn send_hello_message(
    mut commands: Commands,
    mut client_query: Query<
        (
            Entity,
            &mut MessageSender<crate::common::net::messages::HelloMessage>,
        ),
        (With<Connected>, Without<HelloSent>),
    >,
    ukey_res: Option<Res<crate::client::ClientUkey>>,
) {
    let Some(ukey) = ukey_res else {
        return;
    };
    if ukey.0.is_empty() {
        return;
    }
    for (entity, mut sender) in &mut client_query {
        info!("Sending HelloMessage with ukey to server...");
        let _ = sender.send::<crate::common::net::messages::GameChannel>(
            crate::common::net::messages::HelloMessage {
                ukey: ukey.0.clone(),
            },
        );
        commands.entity(entity).insert(HelloSent);
    }
}

fn handle_chat_broadcast(
    mut receivers: Query<&mut MessageReceiver<crate::common::net::messages::ChatBroadcastMessage>>,
    mut chat: ResMut<ui::chat_container::ChatContState>,
) {
    for mut receiver in &mut receivers {
        for broadcast in receiver.receive() {
            let Some(text) =
                crate::common::net::messages::sanitize_chat_text(&broadcast.text)
            else {
                continue;
            };
            if broadcast.username.trim().is_empty() {
                continue;
            }
            chat.push_entry(broadcast.username.clone(), text);
        }
    }
}

fn handle_kick_message(
    mut commands: Commands,
    mut receivers: Query<(
        Entity,
        &mut MessageReceiver<crate::common::net::messages::KickMessage>,
    )>,
) {
    for (entity, mut receiver) in &mut receivers {
        for kick in receiver.receive() {
            warn!("KICKED from server! Reason: {}", kick.reason);
            commands.trigger(lightyear::prelude::client::Disconnect { entity });
        }
    }
}

fn handle_auth_success(
    mut receivers: Query<&mut MessageReceiver<crate::common::net::messages::AuthSuccessMessage>>,
    mut local_username: ResMut<ui::chat_container::LocalUsername>,
) {
    for mut receiver in &mut receivers {
        for success in receiver.receive() {
            info!(
                "Successfully authenticated! User ID: {}, Username: {}",
                success.uid, success.username
            );
            local_username.0 = success.username.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(client_id: u64) -> crate::common::net::components::Player {
        crate::common::net::components::Player {
            client_id,
            ..default()
        }
    }

    #[test]
    fn indexes_confirmed_transforms_by_client() {
        let players = [player(1), player(2)];
        let transforms = [
            Transform::from_xyz(1.0, 2.0, 3.0),
            Transform::from_xyz(4.0, 5.0, 6.0),
        ];
        let mut index = std::collections::HashMap::new();

        index_confirmed_transforms(&mut index, players.iter().zip(transforms.iter()));

        assert_eq!(index.get(&1), Some(&transforms[0]));
        assert_eq!(index.get(&2), Some(&transforms[1]));
        assert_eq!(index.get(&3), None);
    }

    #[test]
    fn preserves_the_first_duplicate_transform() {
        let players = [player(1), player(1)];
        let transforms = [
            Transform::from_xyz(1.0, 0.0, 0.0),
            Transform::from_xyz(2.0, 0.0, 0.0),
        ];
        let mut index = std::collections::HashMap::new();

        index_confirmed_transforms(&mut index, players.iter().zip(transforms.iter()));

        assert_eq!(index.get(&1), Some(&transforms[0]));
    }

    fn sample_at(time: f64, translation: Vec3) -> ServerSample {
        ServerSample {
            received_at: time,
            translation,
            velocity: Vec3::ZERO,
        }
    }

    fn sample_moving(time: f64, translation: Vec3, velocity: Vec3) -> ServerSample {
        ServerSample {
            received_at: time,
            translation,
            velocity,
        }
    }

    fn movement_output(velocity: Vec3) -> crate::common::game::movement::CharacterMoveOutput {
        crate::common::game::movement::CharacterMoveOutput {
            velocity,
            position_y: Some(0.0),
            position_xz: Some(Vec2::ZERO),
            grounded: true,
        }
    }

    #[test]
    fn snap_triggers_on_large_disagreement() {
        let sample = sample_at(10.0, Vec3::new(0.0, 0.7, 0.0));
        assert!(should_snap_to_server(
            Some(&sample),
            Vec3::new(2.0, 0.7, 0.0),
            10.0,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
    }

    #[test]
    fn snap_ignores_small_disagreement() {
        //dude its just a small disagreement, dont worry about it

        let sample = sample_at(10.0, Vec3::new(0.0, 0.7, -0.2));
        assert!(!should_snap_to_server(
            Some(&sample),
            Vec3::new(0.0, 0.7, 0.0),
            10.0,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
    }

    #[test]
    fn snap_ignores_stale_samples() {
        let sample = sample_at(10.0, Vec3::new(100.0, 0.7, 0.0));
        assert!(!should_snap_to_server(
            Some(&sample),
            Vec3::new(0.0, 0.7, 0.0),
            10.0 + LOCAL_SERVER_SAMPLE_MAX_AGE + 1.0,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
    }

    #[test]
    fn snap_ignores_missing_samples() {
        assert!(!should_snap_to_server(
            None,
            Vec3::new(0.0, 0.7, 0.0),
            10.0,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
    }

    #[test]
    fn snap_threshold_is_scale_invariant() {
        let sample = sample_at(10.0, Vec3::new(0.0, 0.7, 0.0));
        assert!(!should_snap_to_server(
            Some(&sample),
            Vec3::new(0.0, 0.7, LOCAL_SNAP_THRESHOLD - 0.01),
            10.0,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
        assert!(should_snap_to_server(
            Some(&sample),
            Vec3::new(0.0, 0.7, LOCAL_SNAP_THRESHOLD + 0.01),
            10.0,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
    }

    #[test]
    fn snap_ignores_latency_gap_while_moving_fast() {
        let sample = sample_moving(10.0, Vec3::new(0.0, 0.7, 1.33), Vec3::new(0.0, 0.0, -40.0));
        assert!(!should_snap_to_server(
            Some(&sample),
            Vec3::new(0.0, 0.7, 0.0),
            10.0 + 0.033,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
    }

    #[test]
    fn snap_ignores_step_up_gap_while_moving_fast() {
        let sample = sample_moving(10.0, Vec3::new(0.0, 0.7, 1.33), Vec3::new(0.0, 0.0, -40.0));
        assert!(!should_snap_to_server(
            Some(&sample),
            Vec3::new(0.0, 1.26, 0.0),
            10.0 + 0.033,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
    }

    #[test]
    fn snap_still_fires_on_teleport_while_moving_fast() {
        let sample = sample_moving(10.0, Vec3::new(0.0, 0.7, 0.0), Vec3::new(0.0, 0.0, -40.0));
        assert!(should_snap_to_server(
            Some(&sample),
            Vec3::new(0.0, 0.7, -50.0),
            10.0 + 0.033,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
    }

    #[test]
    fn snap_tolerates_high_speed_extrapolation_gap() {
        let sample = sample_at(10.0, Vec3::new(0.0, 0.7, 0.0));
        let gap = Vec3::new(0.0, 0.7, LOCAL_SNAP_THRESHOLD + 1.0);
        assert!(should_snap_to_server(
            Some(&sample),
            gap,
            10.0,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            0.0
        ));
        assert!(!should_snap_to_server(
            Some(&sample),
            gap,
            10.0,
            LOCAL_SNAP_THRESHOLD,
            LOCAL_SERVER_SAMPLE_MAX_AGE,
            std::time::Duration::ZERO,
            40.0
        ));
    }

    #[test]
    fn snap_sets_output_position_from_server() {
        let mut output = movement_output(Vec3::ZERO);
        snap_to_server(Vec3::new(3.0, 1.5, -7.0), &mut output);
        assert_eq!(output.position_xz, Some(Vec2::new(3.0, -7.0)));
        assert_eq!(output.position_y, Some(1.5));
    }

    #[test]
    fn clears_stale_prediction_when_playtest_inactive() {
        let mut app = App::new();
        app.init_resource::<PlaytestState>();
        app.init_resource::<LocalPredictionState>();
        app.init_resource::<ReplicationStats>();
        app.init_resource::<ui::chat_container::ChatContState>();
        app.init_resource::<ui::ChatboxState>();
        app.init_resource::<ui::chat_container::LocalUsername>();
        app.world_mut()
            .resource_mut::<LocalPredictionState>()
            .server_sample = Some(ServerSample {
            received_at: 10.0,
            translation: Vec3::new(8.0, 0.7, 5.0),
            velocity: Vec3::ZERO,
        });
        app.add_systems(Update, reset_playtest_transient_state);
        app.update();
        assert!(
            app.world()
                .resource::<LocalPredictionState>()
                .server_sample
                .is_none()
        );
    }

    #[test]
    fn keeps_prediction_while_playtest_active() {
        let mut app = App::new();
        app.init_resource::<PlaytestState>();
        app.init_resource::<LocalPredictionState>();
        app.init_resource::<ReplicationStats>();
        app.init_resource::<ui::chat_container::ChatContState>();
        app.init_resource::<ui::ChatboxState>();
        app.init_resource::<ui::chat_container::LocalUsername>();
        app.world_mut().resource_mut::<PlaytestState>().active = true;
        app.world_mut()
            .resource_mut::<LocalPredictionState>()
            .server_sample = Some(ServerSample {
            received_at: 10.0,
            translation: Vec3::new(8.0, 0.7, 5.0),
            velocity: Vec3::ZERO,
        });
        app.add_systems(Update, reset_playtest_transient_state);
        app.update();
        assert!(
            app.world()
                .resource::<LocalPredictionState>()
                .server_sample
                .is_some()
        );
    }

    #[test]
    fn restores_studio_physics_after_playtest() {
        let mut time_physics = Time::<Physics>::default();
        time_physics.pause();
        let mut state = PhysicsSimulationState::Stopped;
        let mut playtest_physics = StudioPlaytestPhysicsState::default();

        update_studio_playtest_physics(true, &mut time_physics, &mut state, &mut playtest_physics);

        assert_eq!(state, PhysicsSimulationState::Running);
        assert!(!time_physics.is_paused());

        update_studio_playtest_physics(false, &mut time_physics, &mut state, &mut playtest_physics);

        assert_eq!(state, PhysicsSimulationState::Stopped);
        assert!(time_physics.is_paused());
    }

    #[test]
    fn interpolates_remote_brick_transforms_between_samples() {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
        app.init_resource::<ReplicationStats>();
        app.add_systems(Update, interpolate_remote_transforms);

        let brick = app
            .world_mut()
            .spawn((
                NetworkTransform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                    velocity: Vec3::ZERO,
                },
                Transform::default(),
                crate::common::game::bricks::components::Brick,
            ))
            .id();

        for step in 1..=10u32 {
            app.world_mut()
                .entity_mut(brick)
                .get_mut::<NetworkTransform>()
                .unwrap()
                .translation
                .x = step as f32 * 0.1;
            app.update();
        }

        let transform = app.world().get::<Transform>(brick).unwrap();
        assert!(
            transform.translation.x > 0.1 && transform.translation.x < 1.0,
            "brick transform should lag smoothly between network samples instead of snapping, got x: {}",
            transform.translation.x
        );
    }

    #[test]
    fn preserves_running_studio_physics_after_playtest() {
        let mut time_physics = Time::<Physics>::default();
        let mut state = PhysicsSimulationState::Running;
        let mut playtest_physics = StudioPlaytestPhysicsState::default();

        update_studio_playtest_physics(true, &mut time_physics, &mut state, &mut playtest_physics);
        update_studio_playtest_physics(false, &mut time_physics, &mut state, &mut playtest_physics);

        assert_eq!(state, PhysicsSimulationState::Running);
        assert!(!time_physics.is_paused());
    }

    #[cfg(feature = "bench")]
    #[test]
    fn client_benchmark_spawns_deterministic_players() {
        let mut app = App::new();
        app.add_systems(Startup, spawn_client_benchmark);
        app.update();

        let player_count = app
            .world_mut()
            .query::<&crate::common::net::components::Player>()
            .iter(app.world())
            .count();
        assert_eq!(player_count, 300);
    }
}

#[cfg(debug_assertions)]
fn debug_cameras(
    query: Query<(
        Entity,
        &Camera,
        Option<&bevy::camera::RenderTarget>,
        Option<&Name>,
        Option<&bevy::camera_controller::free_camera::FreeCamera>,
        Option<&crate::client::player::PlayerCamera>,
    )>,
    mut last_log: Local<f32>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    if now - *last_log < 1.0 {
        return;
    }
    *last_log = now;
    for (entity, camera, target_opt, name_opt, free_opt, player_opt) in &query {
        let name = name_opt.map(|n| n.as_str()).unwrap_or("No Name");
        let camera_type = if free_opt.is_some() {
            "FreeCamera"
        } else if player_opt.is_some() {
            "PlayerCamera"
        } else {
            "Other"
        };
        let has_egui = match target_opt {
            Some(bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Primary)) => {
                "PrimaryWindow"
            }
            Some(_) => "OtherTarget",
            None => "None",
        };
        info!(
            "CAMERA_DEBUG: Entity {:?} ({}) - type={}, active={}, order={}, clear_color={:?}, target={}",
            entity, name, camera_type, camera.is_active, camera.order, camera.clear_color, has_egui
        );
    }
}

#[cfg(debug_assertions)]
fn debug_players(
    query: Query<
        (
            Entity,
            Option<&Predicted>,
            Option<&Interpolated>,
            Option<&Replicate>,
            Option<&LocalPlayer>,
            &Transform,
        ),
        With<crate::common::net::components::Player>,
    >,
    mut last_log: Local<f32>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    if now - *last_log < 1.0 {
        return;
    }
    *last_log = now;
    for (entity, pred, interp, rep, local, transform) in &query {
        info!(
            "DEBUG_PLAYERS: {:?}: pred={} interp={} repl={} local={} pos={:?}",
            entity,
            pred.is_some(),
            interp.is_some(),
            rep.is_some(),
            local.is_some(),
            transform.translation,
        );
    }
}

#[cfg(feature = "bench")]
fn spawn_client_benchmark(mut commands: Commands) {
    use avian3d::prelude::*;

    commands.spawn((
        Transform::from_xyz(0.0, -0.14, 0.0),
        RigidBody::Static,
        Collider::cuboid(120.0, 0.28, 120.0),
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
    ));
    for index in 0..100u64 {
        let x = (index % 10) as f32 * 2.0;
        let z = (index / 10) as f32 * 2.0;
        let player = crate::common::net::components::Player {
            client_id: index,
            username: format!("BenchPlayer{index}"),
            ..default()
        };
        let transform = Transform::from_xyz(x, 0.84, z);
        commands.spawn((player.clone(), transform, Visibility::Inherited));
        commands.spawn((player.clone(), transform, Predicted));
        commands.spawn((player, transform, Interpolated));
    }
}

#[cfg(feature = "bench")]
pub fn add_client_benchmark(app: &mut App) {
    app.add_systems(Startup, spawn_client_benchmark)
        .add_systems(
            Update,
            (
                sync_predicted_interpolated_transforms,
                hide_confirmed_player_visuals,
            )
                .chain(),
        );
}
