pub mod player;
pub mod ui;

use bevy::prelude::*;
use bevy::pbr::ExtendedMaterial;
use bevy::light::ShadowFilteringMethod;
use avian3d::prelude::Physics;
use avian3d::schedule::PhysicsTime;
use lightyear::prelude::*;
use crate::common::game::bricks::components::{Brick, BrickShapeComponent};
use crate::common::game::bricks::components;
use crate::common::game::bricks::studs::{StudsAssets, StudsExtension};
use crate::common::net::components::NetworkTransform;
use crate::common::game::physics::PhysicsSimulationState;
use bevy_egui::EguiContexts;
use bevy::camera::Hdr;

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

#[derive(Component)]
pub struct HelloSent;

pub fn is_playtesting(playtest: Option<Res<PlaytestState>>) -> bool {
    if std::env::var("VERTIGO_APP").unwrap_or_default() == "client" {
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
            .init_resource::<PlaytestState>()
            .add_plugins(player::PlayerPlugin)
            .add_plugins(crate::common::net::ProtocolPlugin)
            .add_systems(Startup, (
                setup_physics_initializer,
                setup_player_assets,
            ))
            .add_systems(PreUpdate, initialize_client_physics)
            .add_systems(Update, (
                sync_network_transforms_to_client,
                sync_predicted_interpolated_transforms,
                sync_brick_color_to_material,
                send_player_inputs,
                sync_local_player,
                attach_character_visuals.after(sync_local_player),
                update_local_player_transparency,
                hide_confirmed_player_visuals.after(update_local_player_transparency),
                send_hello_message,
                handle_kick_message,
                handle_auth_success,
            ).run_if(is_playtesting))
            .add_systems(Update, cleanup_orphaned_visuals);
            #[cfg(debug_assertions)]
            app.add_systems(Update, (
                debug_cameras,
                debug_players,
                debug_deep_hierarchy,
            ).run_if(is_playtesting));
            app.add_systems(bevy_egui::EguiPrimaryContextPass, (
                ui::configure_client_visuals,
                ui::draw_scoreboard,
                ui::draw_chatbox,
                ui::draw_health_bar,
                ui::draw_chat_container,
            ).run_if(is_playtesting))
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
) {
    if std::env::var("VERTIGO_APP").unwrap_or_default() == "studio" {
        return;
    }

    egui_global_settings.auto_create_primary_context = false;

    commands.spawn(ClientPhysicsInitializer);
    commands.insert_resource(crate::scripting::vm::client_vm::ClientScriptVM::new());

    commands.spawn((
        Camera3d::default(),
        Camera::default(),
        StartupCamera,
        Transform::from_xyz(0.0, 15.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Sample4,
        Hdr,
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
        ShadowFilteringMethod::Gaussian,
        bevy_egui::PrimaryEguiContext,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Hdr,
        bevy::ui::prelude::IsDefaultUiCamera,
    ));
}

pub fn setup_player_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let avatar_scene = asset_server.load("content/game/character/Legacy/Av.glb#Scene0");
    let gltf_handle = asset_server.load("content/game/character/Legacy/Av.glb");

    commands.insert_resource(player::loader::PlayerCharacterAssets {
        avatar_scene,
    });
    commands.insert_resource(player::model::PlayerGltfHandle(gltf_handle));
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
    time: Res<Time>,
    mut query: Query<(
        &NetworkTransform,
        &mut Transform,
        Option<&crate::common::net::components::Player>,
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
    mut sender_query: Query<&mut MessageSender<crate::common::net::messages::PlayerInputMessage>>,
    mut contexts: EguiContexts,
) {
    let wants_keyboard = if let Ok(ctx) = contexts.ctx_mut() {
        ctx.egui_wants_keyboard_input()
    } else {
        false
    };

    let Some((_camera_transform, camera_settings)) = camera_query.iter().next() else {
        trace!("send_player_inputs skipped: PlayerCamera query empty");
        return;
    };
    let Some(mut sender) = sender_query.iter_mut().next() else {
        trace!("send_player_inputs skipped: MessageSender query empty");
        return;
    };

    let w = !wants_keyboard && keys.pressed(KeyCode::KeyW);
    let a = !wants_keyboard && keys.pressed(KeyCode::KeyA);
    let s = !wants_keyboard && keys.pressed(KeyCode::KeyS);
    let d = !wants_keyboard && keys.pressed(KeyCode::KeyD);
    let jump = !wants_keyboard && keys.pressed(KeyCode::Space);
    let in_first_person = camera_settings.distance <= 0.6;

    if w || a || s || d || jump {
        trace!("Client transmitting PlayerInputMessage: w={}, a={}, s={}, d={}, jump={}, yaw={}, in_first_person={}",
            w, a, s, d, jump, camera_settings.yaw, in_first_person);
    }

    let message = crate::common::net::messages::PlayerInputMessage {
        w,
        a,
        s,
        d,
        jump,
        yaw: camera_settings.yaw,
        in_first_person,
    };

    let _ = sender.send::<crate::common::net::messages::GameChannel>(message);
}

fn on_client_connected(
    trigger: On<Add, Connected>,
    query: Query<&LocalId>,
    mut commands: Commands,
) {
    debug!("on_client_connected observer triggered for entity: {:?}", trigger.entity);
    if let Ok(local_id) = query.get(trigger.entity) {
        let client_id = local_id.0.to_bits();
        info!("Client connected successfully! Mapped Local Client ID: {}", client_id);
        commands.insert_resource(LocalClientId(client_id));
    } else {
        warn!("on_client_connected failed: LocalId component missing on target entity");
    }
}

fn on_player_added(
    trigger: On<Add, crate::common::net::components::Player>,
    mut commands: Commands,
    query: Query<(Option<&Predicted>, Option<&Interpolated>, Option<&Replicate>)>,
) {
    let entity = trigger.entity;
    let (pred, interp, rep) = query.get(entity)
        .map(|(p, i, r)| (p.is_some(), i.is_some(), r.is_some()))
        .unwrap_or((false, false, false));
    info!("PLAYER ADDED OBSERVER: {:?} (predicted={}, interpolated={}, replicated={})", entity, pred, interp, rep);
    if rep {
        return;
    }
    commands.entity(entity).insert(NeedsCharacterVisuals);
}

fn on_player_removed(
    trigger: On<Remove, crate::common::net::components::Player>,
    mut commands: Commands,
) {
    debug!("CLIENT PLAYER REMOVED: {:?}, performing recursive despawn of children", trigger.entity);
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
            debug!("CLIENT: Despawning orphaned player visual child {:?} as its parent has been despawned", entity);
            if let Ok(mut entity_cmd) = commands.get_entity(entity) {
                entity_cmd.despawn();
            }
        }
    }
}

fn attach_character_visuals(
    mut commands: Commands,
    character_assets: Option<Res<player::loader::PlayerCharacterAssets>>,
    query: Query<(Entity, &crate::common::net::components::Player, Option<&LocalPlayer>), (With<NeedsCharacterVisuals>, Without<CharacterVisualsSpawned>, Without<Replicate>)>,
    local_client_id: Option<Res<LocalClientId>>,
) {
    let Some(assets) = character_assets else {
        return;
    };

    let local_id = local_client_id.map(|id| id.0);

    for (entity, player_comp, local_player_opt) in &query {
        let is_local = (local_id == Some(player_comp.client_id)) || local_player_opt.is_some();
        info!("ATTACHING CHARACTER VISUALS TO entity={:?}, client_id={}, is_local={}", entity, player_comp.client_id, is_local);

        let mut child_cmd = commands.spawn((
            WorldAssetRoot(assets.avatar_scene.clone()),
            Transform::from_translation(Vec3::new(0.0, -0.7, 0.0))
                .with_scale(Vec3::splat(0.28)),
            GlobalTransform::default(),
            Visibility::Inherited,
            PlayerVisualChild { parent: entity },
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
    query: Query<(Entity, &crate::common::net::components::Player), (Without<LocalPlayer>, Without<Replicate>)>,
    local_client_id: Option<Res<LocalClientId>>,
    startup_cameras: Query<Entity, With<StartupCamera>>,
) {
    let Some(local_id) = local_client_id else {
        return;
    };
    let local_client_id = local_id.0;
    for (entity, player) in &query {
        trace!("sync_local_player checking entity={:?}, player client_id={}, expected client_id={}",
            entity, player.client_id, local_client_id);
        if player.client_id == local_client_id {
            debug!("Local player match verified! Inserting LocalPlayer and spawning camera on entity: {:?}", entity);
            commands.entity(entity).insert(LocalPlayer);

            for camera_entity in &startup_cameras {
                commands.entity(camera_entity).despawn();
            }

            let mut cam_cmd = commands.spawn((
                Camera3d::default(),
                Camera::default(),
                Projection::Perspective(PerspectiveProjection {
                    far: 3000.0,
                    fov: 70.0f32.to_radians(),
                    ..default()
                }),
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
                Hdr,
                bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
                ShadowFilteringMethod::Gaussian,
            ));

            if std::env::var("VERTIGO_APP").unwrap_or_default() == "client" {
                cam_cmd.insert(bevy_egui::PrimaryEguiContext);
            }
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
    color_query: Query<&crate::common::game::bricks::components::BrickColor>,
    mut cache: ResMut<crate::common::game::bricks::BrickMaterialCache>,
) {
    let entity = trigger.entity;
    trace!("Brick added to scene: {:?}", entity);
    let shape = shape_query.get(entity).map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);

    let mesh_handle = match shape {
        crate::common::game::bricks::components::BrickShape::Block => {
            if cache.block_mesh.is_none() {
                cache.block_mesh = Some(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)));
            }
            cache.block_mesh.clone().unwrap()
        }
        crate::common::game::bricks::components::BrickShape::Sphere => {
            if cache.sphere_mesh.is_none() {
                cache.sphere_mesh = Some(meshes.add(Sphere::new(1.0 * 0.28)));
            }
            cache.sphere_mesh.clone().unwrap()
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

    let srgba = base_color.to_srgba();
    let cache_key = [
        srgba.red.to_bits(),
        srgba.green.to_bits(),
        srgba.blue.to_bits(),
        srgba.alpha.to_bits(),
    ];

    let material_handle = if let Some(existing) = cache.studs_materials.get(&cache_key) {
        existing.clone()
    } else {
        let new_mat = studs_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color,
                perceptual_roughness: 0.9,
                alpha_mode: if base_color.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
                ..default()
            },
            extension: StudsExtension {
                stud_texture: studs_assets.stud.clone(),
                inlet_texture: studs_assets.inlet.clone(),
            },
        });
        cache.studs_materials.insert(cache_key, new_mat.clone());
        new_mat
    };

    commands.entity(entity).insert((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
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

fn sync_predicted_interpolated_transforms(
    mut predicted_interpolated_query: Query<(&crate::common::net::components::Player, &mut Transform), Or<(With<Predicted>, With<Interpolated>)>>,
    confirmed_query: Query<(&crate::common::net::components::Player, &Transform), (Without<Predicted>, Without<Interpolated>, Without<Replicate>)>,
) {
    for (player, mut transform) in &mut predicted_interpolated_query {
        for (conf_player, conf_transform) in &confirmed_query {
            if player.client_id == conf_player.client_id {
                transform.translation = conf_transform.translation;
                transform.rotation = conf_transform.rotation;
                transform.scale = conf_transform.scale;
                break;
            }
        }
    }
}

fn sync_brick_color_to_material(
    mut commands: Commands,
    query: Query<(Entity, &crate::common::game::bricks::components::BrickColor), Changed<crate::common::game::bricks::components::BrickColor>>,
    mut cache: ResMut<crate::common::game::bricks::BrickMaterialCache>,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
    studs_assets: Res<StudsAssets>,
) {
    for (entity, brick_color) in &query {
        let base_color = brick_color.color;
        let srgba = base_color.to_srgba();
        let cache_key = [
            srgba.red.to_bits(),
            srgba.green.to_bits(),
            srgba.blue.to_bits(),
            srgba.alpha.to_bits(),
        ];

        let material_handle = if let Some(existing) = cache.studs_materials.get(&cache_key) {
            existing.clone()
        } else {
            let new_mat = studs_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color,
                    perceptual_roughness: 0.9,
                    alpha_mode: if base_color.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
                    ..default()
                },
                extension: StudsExtension {
                    stud_texture: studs_assets.stud.clone(),
                    inlet_texture: studs_assets.inlet.clone(),
                },
            });
            cache.studs_materials.insert(cache_key, new_mat.clone());
            new_mat
        };
        commands.entity(entity).insert(MeshMaterial3d(material_handle));
    }
}

fn hide_confirmed_player_visuals(
    predicted_interpolated_query: Query<&crate::common::net::components::Player, Or<(With<Predicted>, With<Interpolated>)>>,
    mut confirmed_query: Query<(&crate::common::net::components::Player, &mut Visibility), (Without<Predicted>, Without<Interpolated>, Without<Replicate>)>,
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

fn send_hello_message(
    mut commands: Commands,
    mut client_query: Query<(Entity, &mut MessageSender<crate::common::net::messages::HelloMessage>), (With<Connected>, Without<HelloSent>)>,
    ukey_res: Option<Res<crate::client::ClientUkey>>,
) {
    let Some(ukey) = ukey_res else { return; };
    if ukey.0.is_empty() { return; }
    for (entity, mut sender) in &mut client_query {
        info!("Sending HelloMessage with ukey to server...");
        let _ = sender.send::<crate::common::net::messages::GameChannel>(crate::common::net::messages::HelloMessage {
            ukey: ukey.0.clone(),
        });
        commands.entity(entity).insert(HelloSent);
    }
}

fn handle_kick_message(
    mut commands: Commands,
    mut receivers: Query<(Entity, &mut MessageReceiver<crate::common::net::messages::KickMessage>)>,
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
) {
    for mut receiver in &mut receivers {
        for success in receiver.receive() {
            info!("Successfully authenticated! User ID: {}, Username: {}", success.uid, success.username);
        }
    }
}

fn links_optimizer_system() {}

#[cfg(debug_assertions)]
fn debug_cameras(
    query: Query<(Entity, &Camera, Option<&bevy::camera::RenderTarget>, Option<&Name>, Option<&bevy::camera_controller::free_camera::FreeCamera>, Option<&crate::client::player::PlayerCamera>)>,
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
            Some(bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Primary)) => "PrimaryWindow",
            Some(_) => "OtherTarget",
            None => "None",
        };
        info!("CAMERA_DEBUG: Entity {:?} ({}) - type={}, active={}, order={}, clear_color={:?}, target={}",
            entity, name, camera_type, camera.is_active, camera.order, camera.clear_color, has_egui);
    }
}

#[cfg(debug_assertions)]
fn debug_players(
    query: Query<(
        Entity,
        Option<&Predicted>,
        Option<&Interpolated>,
        Option<&Replicate>,
        Option<&LocalPlayer>,
        &Transform,
    ), With<crate::common::net::components::Player>>,
    mut last_log: Local<f32>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    if now - *last_log < 1.0 {
        return;
    }
    *last_log = now;
    for (entity, pred, interp, rep, local, transform) in &query {
        info!("DEBUG_PLAYERS: {:?}: pred={} interp={} repl={} local={} pos={:?}",
            entity,
            pred.is_some(),
            interp.is_some(),
            rep.is_some(),
            local.is_some(),
            transform.translation,
        );
    }
}

#[cfg(debug_assertions)]
fn debug_deep_hierarchy(
    world: &World,
    mut last_log: Local<f32>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    if now - *last_log < 2.0 {
        return;
    }
    *last_log = now;

    info!("DEEP HIERARCHY INSPECTION:");
    for archetype in world.archetypes().iter() {
        for entity in archetype.entities() {
            let entity = entity.id();
            if world.get::<WorldAssetRoot>(entity).is_some() {
                info!("Entity {:?} has WorldAssetRoot! parent={:?}, visibility={:?}",
                    entity,
                    world.get::<ChildOf>(entity).map(|co| co.parent()),
                    world.get::<Visibility>(entity),
                );
                print_hierarchy_from_root(world, entity, 1);
            }
        }
    }
}

#[cfg(debug_assertions)]
fn print_hierarchy_from_root(world: &World, entity: Entity, depth: usize) {
    let indent = "  ".repeat(depth);
    let name = world.get::<Name>(entity).map(|n| n.as_str().to_string()).unwrap_or_else(|| "Instance".to_string());
    let vis = world.get::<Visibility>(entity);
    let transform = world.get::<Transform>(entity);
    
    let mut comp_names = Vec::new();
    if let Ok(entity_ref) = world.get_entity(entity) {
        let archetype = entity_ref.archetype();
        for component_id in archetype.components() {
            if let Some(info) = world.components().get_info(*component_id) {
                comp_names.push(info.name().split("::").last().unwrap_or("").to_string());
            }
        }
    }

    info!("{}└─ Entity {:?} '{}': vis={:?}, transform={:?}, components={:?}",
        indent, entity, name, vis, transform.map(|t| t.translation), comp_names);

    if let Some(children) = world.get::<Children>(entity) {
        for child in children.iter() {
            print_hierarchy_from_root(world, child, depth + 1);
        }
    }
}