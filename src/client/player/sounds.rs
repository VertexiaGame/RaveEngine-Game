use bevy::audio::{
    AudioPlayer, AudioSource, PlaybackSettings, SpatialListener, SpatialScale, Volume,
};
use bevy::prelude::*;
use lightyear::prelude::*;

use crate::common::game::sounds::{
    SPATIAL_SCALE, STEP_LOOP_HEARTBEAT_SECS, STEP_SOUND_PATH, is_jump_start, is_walking,
    jump_sound_path, sound_volume, within_hearing_range,
};
use crate::common::net::messages::{
    GameChannel, PlayerSoundBroadcast, PlayerSoundKind, PlayerSoundRequest,
};

#[derive(Resource, Default)]
pub struct PlayerSoundAssets {
    pub step: Option<Handle<AudioSource>>,
    pub jump: Option<Handle<AudioSource>>,
}

#[derive(Resource, Default)]
struct LocalSoundClock {
    was_grounded: bool,
    initialized: bool,
    was_walking: bool,
    last_heartbeat: f32,
}

#[derive(Resource, Default)]
struct ActiveStepLoops {
    local: Option<Entity>,
    remote: std::collections::HashMap<u64, Entity>,
}

pub struct PlayerSoundsPlugin;

impl Plugin for PlayerSoundsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSoundAssets>()
            .init_resource::<LocalSoundClock>()
            .init_resource::<ActiveStepLoops>()
            .add_systems(Startup, load_player_sounds)
            .add_systems(
                Update,
                (
                    ensure_player_camera_listener,
                    manage_local_step_loop,
                    handle_sound_broadcasts,
                    sync_step_loops,
                )
                    .run_if(crate::client::is_playtesting),
            );
    }
}

fn load_player_sounds(
    asset_server: Option<Res<AssetServer>>,
    mut assets: ResMut<PlayerSoundAssets>,
) {
    let Some(asset_server) = asset_server else {
        return;
    };
    if assets.step.is_none() {
        assets.step = Some(asset_server.load(STEP_SOUND_PATH));
    }
    if assets.jump.is_none() {
        assets.jump = Some(asset_server.load(jump_sound_path()));
    }
}

fn ensure_player_camera_listener(
    mut commands: Commands,
    query: Query<Entity, (With<crate::client::player::PlayerCamera>, Without<SpatialListener>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(SpatialListener::default());
    }
}

fn sound_handle(
    assets: &PlayerSoundAssets,
    kind: PlayerSoundKind,
) -> Option<Handle<AudioSource>> {
    match kind {
        PlayerSoundKind::StepStart | PlayerSoundKind::StepStop => assets.step.clone(),
        PlayerSoundKind::Jump => assets.jump.clone(),
    }
}

fn spatial_playback(kind: PlayerSoundKind) -> PlaybackSettings {
    PlaybackSettings::DESPAWN
        .with_spatial(true)
        .with_spatial_scale(SpatialScale::new(SPATIAL_SCALE))
        .with_volume(Volume::Linear(sound_volume(kind)))
}

fn spawn_spatial_sound(
    commands: &mut Commands,
    assets: &PlayerSoundAssets,
    kind: PlayerSoundKind,
    position: Vec3,
) {
    let Some(handle) = sound_handle(assets, kind) else {
        return;
    };
    if !position.is_finite() {
        return;
    }
    commands.spawn((
        AudioPlayer::new(handle),
        spatial_playback(kind),
        Transform::from_translation(position),
        GlobalTransform::from(Transform::from_translation(position)),
    ));
}

fn spawn_step_loop(
    commands: &mut Commands,
    assets: &PlayerSoundAssets,
    position: Vec3,
) -> Option<Entity> {
    let handle = assets.step.clone()?;
    if !position.is_finite() {
        return None;
    }
    Some(
        commands
            .spawn((
                AudioPlayer::new(handle),
                PlaybackSettings::LOOP
                    .with_spatial(true)
                    .with_spatial_scale(SpatialScale::new(SPATIAL_SCALE))
                    .with_volume(Volume::Linear(sound_volume(
                        PlayerSoundKind::StepStart,
                    ))),
                Transform::from_translation(position),
                GlobalTransform::from(Transform::from_translation(position)),
            ))
            .id(),
    )
}

fn despawn_step_loop(commands: &mut Commands, entity: Entity) {
    if let Ok(mut entity_commands) = commands.get_entity(entity) {
        entity_commands.despawn();
    }
}

fn manage_local_step_loop(
    mut commands: Commands,
    assets: Res<PlayerSoundAssets>,
    time: Res<Time>,
    mut clock: ResMut<LocalSoundClock>,
    mut loops: ResMut<ActiveStepLoops>,
    local_query: Query<
        (&Transform, &crate::common::game::movement::PlayerMovementPlan),
        (With<crate::client::LocalPlayer>, Without<Replicate>),
    >,
    mut sender_query: Query<&mut MessageSender<PlayerSoundRequest>>,
) {
    let Some((transform, plan)) = local_query.iter().next() else {
        if let Some(entity) = loops.local.take() {
            despawn_step_loop(&mut commands, entity);
        }
        if clock.initialized || clock.was_walking {
            *clock = LocalSoundClock::default();
        }
        return;
    };
    let position = transform.translation;
    let horizontal_speed = Vec2::new(plan.velocity.x, plan.velocity.z).length();
    let walking = is_walking(plan.grounded, horizontal_speed);
    let elapsed = time.elapsed_secs();

    if walking {
        if loops.local.is_none() {
            loops.local = spawn_step_loop(&mut commands, &assets, position);
        }
        if !clock.was_walking || elapsed - clock.last_heartbeat >= STEP_LOOP_HEARTBEAT_SECS {
            send_sound_request(&mut sender_query, PlayerSoundKind::StepStart);
            clock.last_heartbeat = elapsed;
        }
    } else {
        if let Some(entity) = loops.local.take() {
            despawn_step_loop(&mut commands, entity);
        }
        if clock.was_walking {
            send_sound_request(&mut sender_query, PlayerSoundKind::StepStop);
        }
    }
    clock.was_walking = walking;

    if !clock.initialized {
        clock.was_grounded = plan.grounded;
        clock.initialized = true;
        return;
    }
    if is_jump_start(clock.was_grounded, plan.grounded, plan.velocity.y) {
        spawn_spatial_sound(
            &mut commands,
            &assets,
            PlayerSoundKind::Jump,
            position,
        );
        send_sound_request(&mut sender_query, PlayerSoundKind::Jump);
    }
    clock.was_grounded = plan.grounded;
}

fn send_sound_request(
    sender_query: &mut Query<&mut MessageSender<PlayerSoundRequest>>,
    kind: PlayerSoundKind,
) {
    let Some(mut sender) = sender_query.iter_mut().next() else {
        return;
    };
    let _ = sender.send::<GameChannel>(PlayerSoundRequest { kind });
}

fn kill_remote_step_loop(
    commands: &mut Commands,
    loops: &mut ActiveStepLoops,
    source_client_id: u64,
) {
    if let Some(entity) = loops.remote.remove(&source_client_id) {
        despawn_step_loop(commands, entity);
    }
}

fn handle_sound_broadcasts(
    mut commands: Commands,
    assets: Res<PlayerSoundAssets>,
    mut receivers: Query<&mut MessageReceiver<PlayerSoundBroadcast>>,
    mut loops: ResMut<ActiveStepLoops>,
    local_id: Option<Res<crate::client::LocalClientId>>,
    camera_query: Query<&Transform, With<crate::client::player::PlayerCamera>>,
) {
    let local_client_id = local_id.map(|id| id.0);
    let camera_position = camera_query.iter().next().map(|t| t.translation);
    for mut receiver in &mut receivers {
        for broadcast in receiver.receive() {
            if Some(broadcast.source_client_id) == local_client_id {
                continue;
            }
            if !broadcast.position.is_finite() {
                continue;
            }
            match broadcast.kind {
                PlayerSoundKind::StepStart => {
                    if loops.remote.contains_key(&broadcast.source_client_id) {
                        continue;
                    }
                    if let Some(listener) = camera_position {
                        if !within_hearing_range(listener, broadcast.position) {
                            continue;
                        }
                    }
                    if let Some(entity) =
                        spawn_step_loop(&mut commands, &assets, broadcast.position)
                    {
                        loops.remote.insert(broadcast.source_client_id, entity);
                    }
                }
                PlayerSoundKind::StepStop => {
                    kill_remote_step_loop(&mut commands, &mut loops, broadcast.source_client_id);
                }
                PlayerSoundKind::Jump => {
                    kill_remote_step_loop(&mut commands, &mut loops, broadcast.source_client_id);
                    spawn_spatial_sound(
                        &mut commands,
                        &assets,
                        broadcast.kind,
                        broadcast.position,
                    );
                }
            }
        }
    }
}

fn sync_step_loops(
    mut commands: Commands,
    mut loops: ResMut<ActiveStepLoops>,
    mut targets: ParamSet<(
        Query<&mut Transform>,
        Query<
            &Transform,
            (
                With<crate::client::LocalPlayer>,
                With<crate::common::net::components::Player>,
                Without<Replicate>,
            ),
        >,
        Query<(
            &crate::common::net::components::Player,
            &Transform,
            Option<&Predicted>,
            Option<&Interpolated>,
        )>,
    )>,
) {
    let local_position = targets
        .p1()
        .iter()
        .next()
        .map(|transform| transform.translation);
    let mut remote_positions: std::collections::HashMap<u64, Vec3> =
        std::collections::HashMap::new();
    for (player, transform, predicted, interpolated) in targets.p2().iter() {
        if predicted.is_some() || interpolated.is_some() {
            remote_positions.insert(player.client_id, transform.translation);
        } else {
            remote_positions
                .entry(player.client_id)
                .or_insert(transform.translation);
        }
    }
    if let Some(entity) = loops.local {
        match local_position {
            Some(target) => {
                if let Ok(mut loop_transform) = targets.p0().get_mut(entity) {
                    loop_transform.translation = target;
                }
            }
            None => {
                despawn_step_loop(&mut commands, entity);
                loops.local = None;
            }
        }
    }
    let mut missing = Vec::new();
    for (&source_client_id, &entity) in loops.remote.iter() {
        match remote_positions.get(&source_client_id) {
            Some(target) => {
                if let Ok(mut loop_transform) = targets.p0().get_mut(entity) {
                    loop_transform.translation = *target;
                }
            }
            None => missing.push(source_client_id),
        }
    }
    for source_client_id in missing {
        kill_remote_step_loop(&mut commands, &mut loops, source_client_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::game::movement::PlayerMovementPlan;

    fn sound_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PlayerSoundsPlugin);
        app.insert_resource(crate::client::PlaytestState { active: true });
        app
    }

    #[test]
    fn step_loops_follow_their_players_without_query_conflicts() {
        let mut app = sound_test_app();
        let local_loop = app
            .world_mut()
            .spawn(Transform::from_xyz(99.0, 99.0, 99.0))
            .id();
        app.world_mut().spawn((
            crate::client::LocalPlayer,
            crate::common::net::components::Player {
                client_id: 1,
                ..Default::default()
            },
            Transform::from_xyz(1.0, 2.0, 3.0),
            PlayerMovementPlan {
                velocity: Vec3::new(4.0, 0.0, 0.0),
                grounded: true,
                ..Default::default()
            },
        ));
        let remote_loop = app.world_mut().spawn(Transform::default()).id();
        app.world_mut().spawn((
            crate::common::net::components::Player {
                client_id: 7,
                ..Default::default()
            },
            Transform::from_xyz(4.0, 5.0, 6.0),
        ));
        {
            let mut loops = app.world_mut().resource_mut::<ActiveStepLoops>();
            loops.local = Some(local_loop);
            loops.remote.insert(7, remote_loop);
        }
        app.update();
        app.update();
        let local_position = app
            .world()
            .get::<Transform>(local_loop)
            .expect("local loop was despawned")
            .translation;
        assert_eq!(local_position, Vec3::new(1.0, 2.0, 3.0));
        let remote_position = app
            .world()
            .get::<Transform>(remote_loop)
            .expect("remote loop was despawned")
            .translation;
        assert_eq!(remote_position, Vec3::new(4.0, 5.0, 6.0));
    }
}
