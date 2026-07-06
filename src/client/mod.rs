pub mod player;

use bevy::prelude::*;
use bevy::pbr::ExtendedMaterial;
use bevy::light::ShadowFilteringMethod;
use avian3d::prelude::Physics;
use avian3d::schedule::PhysicsTime;
use lightyear::prelude::*;
use crate::common::bricks::components::{Brick, BrickShapeComponent};
use crate::common::bricks::studs::{StudsAssets, StudsExtension};
use crate::common::components::NetworkTransform;
use crate::common::physics::PhysicsSimulationState;

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

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin)
            .add_plugins(crate::common::network::ProtocolPlugin)
            .add_systems(Startup, (
                setup_physics_initializer,
                setup_player_assets,
            ))
            .add_systems(PreUpdate, initialize_client_physics)
            .add_systems(Update, (
                sync_network_transforms_to_client,
                send_player_inputs,
                sync_local_player,
                attach_character_visuals,
                update_local_player_transparency,
            ))
            .add_observer(on_client_connected)
            .add_observer(on_player_added)
            .add_observer(on_brick_added)
            .add_observer(on_network_transform_added);
    }
}

fn setup_physics_initializer(mut commands: Commands) {
    commands.spawn(ClientPhysicsInitializer);
    commands.spawn((
        Camera3d::default(),
        StartupCamera,
        Transform::from_xyz(0.0, 15.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Sample4,
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
        ShadowFilteringMethod::Gaussian,
    ));
}

pub fn setup_player_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let avatar_scene = asset_server.load("content/game/character/Avatar.glb#Scene0");

    commands.insert_resource(player::loader::PlayerCharacterAssets {
        avatar_scene,
    });
}

fn initialize_client_physics(
    mut time_physics: ResMut<Time<Physics>>,
    mut state: ResMut<PhysicsSimulationState>,
    mut commands: Commands,
    query: Query<Entity, With<ClientPhysicsInitializer>>,
) {
    *state = PhysicsSimulationState::Running;
    time_physics.unpause();
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn sync_network_transforms_to_client(
    time: Res<Time>,
    mut query: Query<(
        &NetworkTransform,
        &mut Transform,
        Option<&crate::common::components::Player>,
        Option<&LocalPlayer>,
    ), Without<Replicate>>,
    camera_query: Query<&player::CameraSettings, With<player::PlayerCamera>>,
) {
    let lerp_factor = (25.0 * time.delta_secs()).min(1.0);
    let in_first_person = camera_query.iter().next().map(|s| s.distance <= 0.6).unwrap_or(false);

    for (net_transform, mut transform, player_opt, local_opt) in &mut query {
        transform.translation = net_transform.translation;
        transform.scale = net_transform.scale;

        if player_opt.is_some() {
            if local_opt.is_some() {
                if !in_first_person {
                    transform.rotation = transform.rotation.slerp(net_transform.rotation, lerp_factor);
                }
            } else {
                transform.rotation = transform.rotation.slerp(net_transform.rotation, lerp_factor);
            }
        } else {
            transform.rotation = net_transform.rotation;
        }
    }
}

fn send_player_inputs(
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<(&Transform, &player::CameraSettings), With<player::PlayerCamera>>,
    mut sender_query: Query<&mut MessageSender<crate::common::network::PlayerInputMessage>>,
) {
    let Some((_camera_transform, camera_settings)) = camera_query.iter().next() else {
        trace!("send_player_inputs skipped: PlayerCamera query empty");
        return;
    };
    let Some(mut sender) = sender_query.iter_mut().next() else {
        trace!("send_player_inputs skipped: MessageSender query empty");
        return;
    };

    let w = keys.pressed(KeyCode::KeyW);
    let a = keys.pressed(KeyCode::KeyA);
    let s = keys.pressed(KeyCode::KeyS);
    let d = keys.pressed(KeyCode::KeyD);
    let jump = keys.pressed(KeyCode::Space);
    let in_first_person = camera_settings.distance <= 0.6;

    if w || a || s || d || jump {
        info!("Client transmitting PlayerInputMessage: w={}, a={}, s={}, d={}, jump={}, yaw={}, in_first_person={}",
            w, a, s, d, jump, camera_settings.yaw, in_first_person);
    }

    let message = crate::common::network::PlayerInputMessage {
        w,
        a,
        s,
        d,
        jump,
        yaw: camera_settings.yaw,
        in_first_person,
    };

    let _ = sender.send::<crate::common::network::GameChannel>(message);
}

fn on_client_connected(
    trigger: On<Add, Connected>,
    query: Query<&LocalId>,
    mut commands: Commands,
) {
    info!("on_client_connected observer triggered for entity: {:?}", trigger.entity);
    if let Ok(local_id) = query.get(trigger.entity) {
        let client_id = local_id.0.to_bits();
        info!("Client connected successfully! Mapped Local Client ID: {}", client_id);
        commands.insert_resource(LocalClientId(client_id));
    } else {
        warn!("on_client_connected failed: LocalId component missing on target entity");
    }
}

fn on_player_added(
    trigger: On<Add, crate::common::components::Player>,
    mut commands: Commands,
    query: Query<Entity, Without<Replicate>>,
) {
    let entity = trigger.entity;
    if query.get(entity).is_err() {
        return;
    }
    info!("REMOTE PLAYER ADDED: {:?}", entity);
    commands.entity(entity).insert(NeedsCharacterVisuals);
}

fn attach_character_visuals(
    mut commands: Commands,
    character_assets: Option<Res<player::loader::PlayerCharacterAssets>>,
    query: Query<(Entity, &crate::common::components::Player, Option<&LocalPlayer>), (With<NeedsCharacterVisuals>, Without<CharacterVisualsSpawned>, Without<Replicate>)>,
    local_client_id: Option<Res<LocalClientId>>,
) {
    let Some(assets) = character_assets else {
        return;
    };

    let local_id = local_client_id.map(|id| id.0);

    for (entity, player_comp, local_player_opt) in &query {
        info!("ATTACHING CHARACTER VISUALS TO {:?}", entity);
        let is_local = (local_id == Some(player_comp.client_id)) || local_player_opt.is_some();

        let mut child_cmd = commands.spawn((
            WorldAssetRoot(assets.avatar_scene.clone()),
            Transform::from_translation(Vec3::new(0.0, -0.7, 0.0))
                .with_scale(Vec3::splat(0.28)),
            GlobalTransform::default(),
        ));

        if is_local {
            child_cmd.insert(UniqueLocalMaterial);
        }

        let child_id = child_cmd.id();
        commands.entity(entity).add_child(child_id);

        commands.entity(entity)
            .remove::<NeedsCharacterVisuals>()
            .insert(CharacterVisualsSpawned);
    }
}

fn sync_local_player(
    mut commands: Commands,
    query: Query<(Entity, &crate::common::components::Player), (Without<LocalPlayer>, Without<Replicate>)>,
    client_query: Query<&LocalId, With<lightyear::prelude::client::Client>>,
    startup_cameras: Query<Entity, With<StartupCamera>>,
) {
    let Ok(local_id) = client_query.single() else {
        return;
    };
    let local_client_id = local_id.0.to_bits();
    for (entity, player) in &query {
        trace!("sync_local_player checking entity={:?}, player client_id={}, expected client_id={}",
            entity, player.client_id, local_client_id);
        if player.client_id == local_client_id {
            info!("Local player match verified! Inserting LocalPlayer and spawning camera on entity: {:?}", entity);
            commands.entity(entity).insert(LocalPlayer);

            for camera_entity in &startup_cameras {
                commands.entity(camera_entity).despawn();
            }

            commands.spawn((
                Camera3d::default(),
                player::PlayerCamera,
                player::CameraSettings {
                    yaw: 0.0,
                    pitch: -0.35,
                    distance: 4.5,
                    current_distance: 4.5,
                    target_offset: Vec3::new(0.0, 0.55, 0.0),
                },
                Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
                Msaa::Sample4,
                bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
                ShadowFilteringMethod::Gaussian,
            ));
        }
    }
}

fn update_local_player_transparency(
    camera_query: Query<(&Transform, &player::CameraSettings), With<player::PlayerCamera>>,
    local_player_query: Query<(&Transform, &Children), With<LocalPlayer>>,
    child_query: Query<Entity, With<UniqueLocalMaterial>>,
    mut visibility_query: Query<&mut Visibility>,
) {
    let Some((camera_transform, camera_settings)) = camera_query.iter().next() else {
        return;
    };
    let Some((player_transform, children)) = local_player_query.iter().next() else {
        return;
    };

    let player_target = player_transform.translation + camera_settings.target_offset;
    let distance = camera_transform.translation.distance(player_target);

    let show = distance > 0.6;

    for child in children.iter() {
        if let Ok(child_entity) = child_query.get(child) {
            if let Ok(mut visibility) = visibility_query.get_mut(child_entity) {
                if show {
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
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
    studs_assets: Res<StudsAssets>,
    name_query: Query<&Name>,
    shape_query: Query<&BrickShapeComponent>,
    color_query: Query<&crate::common::bricks::components::BrickColor>,
) {
    let entity = trigger.entity;
    info!("Brick added to scene: {:?}", entity);
    let shape = shape_query.get(entity).map(|s| s.shape).unwrap_or(crate::common::bricks::components::BrickShape::Block);

    let mesh_handle = match shape {
        crate::common::bricks::components::BrickShape::Block => {
            meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))
        }
        crate::common::bricks::components::BrickShape::Sphere => {
            meshes.add(Sphere::new(1.0 * 0.28))
        }
    };

    let base_color = if let Ok(brick_color) = color_query.get(entity) {
        brick_color.color
    } else {
        let name_opt = name_query.get(entity).ok().map(|n| n.as_str());
        if name_opt == Some("Baseplate") {
            Color::srgb(0.28, 0.62, 0.32)
        } else {
            Color::srgb(0.84, 0.24, 0.16)
        }
    };

    commands.entity(entity).insert((
        Mesh3d(mesh_handle),
        MeshMaterial3d(studs_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color,
                perceptual_roughness: 0.9,
                ..default()
            },
            extension: StudsExtension {
                stud_texture: studs_assets.stud.clone(),
                inlet_texture: studs_assets.inlet.clone(),
            },
        })),
    ));
}

fn on_network_transform_added(
    trigger: On<Add, NetworkTransform>,
    mut commands: Commands,
    query: Query<&NetworkTransform>,
) {
    let entity = trigger.entity;
    let Ok(net_transform) = query.get(entity) else { return };
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