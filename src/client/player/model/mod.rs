use bevy::prelude::*;
use crate::client::LocalPlayer;
use lightyear::prelude::Replicate;

#[derive(Resource)]
pub struct PlayerGltfHandle(pub Handle<bevy::gltf::Gltf>);

#[derive(Resource)]
pub struct DiagnosticGltfHandle(pub Handle<bevy::gltf::Gltf>);

#[derive(Component)]
pub struct NeedsCharacterVisuals;

#[derive(Component)]
pub struct CharacterVisualsSpawned;

#[derive(Component)]
pub struct PlayerVisualChild {
    pub parent: Entity,
}

#[derive(Component)]
pub struct UniqueLocalMaterial;

pub fn attach_character_visuals(
    mut commands: Commands,
    character_assets: Option<Res<crate::client::player::loader::PlayerCharacterAssets>>,
    query: Query<(Entity, &crate::common::net::components::Player, Option<&LocalPlayer>), (With<NeedsCharacterVisuals>, Without<CharacterVisualsSpawned>, Without<Replicate>)>,
    local_client_id: Option<Res<crate::client::LocalClientId>>,
) {
    let Some(assets) = character_assets else {
        warn!("PLAYER_LOG: Cannot attach visuals - PlayerCharacterAssets resource is missing!");
        return;
    };

    let local_id = local_client_id.map(|id| id.0);

    for (entity, player_comp, local_player_opt) in &query {
        info!("PLAYER_LOG: Attaching unified Av.glb visuals to player entity: {:?}", entity);
        let is_local = (local_id == Some(player_comp.client_id)) || local_player_opt.is_some();

        let mut visual_root = commands.spawn((
            WorldAssetRoot(assets.avatar_scene.clone()),
            Transform::from_translation(Vec3::new(0.0, -0.7, 0.0))
                .with_scale(Vec3::splat(0.28)),
            GlobalTransform::default(),
            Visibility::Inherited,
            PlayerVisualChild { parent: entity },
        ));

        if is_local {
            visual_root.insert(UniqueLocalMaterial);
        }

        let visual_root_entity = visual_root.id();
        commands.entity(entity).add_child(visual_root_entity);
        info!("PLAYER_LOG: Successfully linked unified visual_root {:?} to player {:?}.", visual_root_entity, entity);

        commands.entity(entity)
            .remove::<NeedsCharacterVisuals>()
            .insert(CharacterVisualsSpawned);
    }
}

pub fn cleanup_orphaned_visuals(
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

pub fn update_local_player_transparency(
    camera_query: Query<(&Transform, &crate::client::player::CameraSettings), With<crate::client::player::PlayerCamera>>,
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

#[cfg(debug_assertions)]
pub fn log_player_loading_precision(
    asset_server: Res<AssetServer>,
    character_assets: Option<Res<crate::client::player::loader::PlayerCharacterAssets>>,
    player_anims: Res<crate::client::player::animation::PlayerAnimationGraphLoaded>,
    mut last_log: Local<f32>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    if now - *last_log < 1.0 {
        return;
    }
    *last_log = now;

    let Some(assets) = &character_assets else {
        info!("PLAYER_LOG: PlayerCharacterAssets resource is missing!");
        return;
    };

    let avatar_state = asset_server.load_state(&assets.avatar_scene);
    info!("PLAYER_LOG: Unified Avatar GLB state: {:?}", avatar_state);

    if let Some(graph_handle) = &player_anims.graph {
        let anim_state = asset_server.load_state(graph_handle);
        info!("PLAYER_LOG: Player animations graph load state: {:?}", anim_state);
    } else {
        info!("PLAYER_LOG: PlayerAnimationGraphLoaded is currently empty.");
    }
}

#[cfg(debug_assertions)]
pub fn inspect_hierarchy_deep(
    world: &World,
    mut last_log: Local<f32>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    if now - *last_log < 3.0 {
        return;
    }
    *last_log = now;

    let mut visual_root_entities = Vec::new();
    for archetype in world.archetypes().iter() {
        for entity in archetype.entities() {
            let entity = entity.id();
            if world.get::<PlayerVisualChild>(entity).is_some() {
                visual_root_entities.push(entity);
            }
        }
    }

    for visual_root in visual_root_entities {
        info!("PLAYER_LOG: Deep inspection of Visual Root Entity {:?}", visual_root);
        print_entity_recursive(world, visual_root, 0);
    }
}

#[cfg(debug_assertions)]
fn print_entity_recursive(world: &World, entity: Entity, depth: usize) {
    let indent = "  ".repeat(depth);
    let name = world.get::<Name>(entity).map(|n| n.as_str().to_string()).unwrap_or_else(|| "Instance".to_string());
    let vis = world.get::<Visibility>(entity);
    
    let mut comp_names = Vec::new();
    let entity_ref = world.entity(entity);
    let archetype = entity_ref.archetype();
    for component_id in archetype.components() {
        if let Some(info) = world.components().get_info(*component_id) {
            comp_names.push(info.name().to_string());
        }
    }

    info!("PLAYER_LOG: {}└─ Entity {:?} '{}': vis={:?}, components={:?}",
        indent, entity, name, vis, comp_names);

    if let Some(children) = world.get::<Children>(entity) {
        for child in children.iter() {
            print_entity_recursive(world, child, depth + 1);
        }
    }
}

#[cfg(debug_assertions)]
pub fn inspect_meshes(
    query: Query<(Entity, &Name, &GlobalTransform, Option<&Visibility>), With<Mesh3d>>,
    mut last_log: Local<f32>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    if now - *last_log < 2.0 {
        return;
    }
    *last_log = now;

    let mut count = 0;
    for (entity, name, global_transform, vis_opt) in &query {
        count += 1;
        let (scale, _rotation, translation) = global_transform.to_scale_rotation_translation();
        info!("PLAYER_LOG: Mesh3d Entity '{}' ({:?}): translation={:?}, scale={:?}, vis={:?}",
            name.as_str(), entity, translation, scale, vis_opt);
    }
    info!("PLAYER_LOG: Total entities with Mesh3d component: {}", count);
}

pub fn inspect_gltf_container(
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    gltf_handle: Option<Res<DiagnosticGltfHandle>>,
    mut logged: Local<bool>,
) {
    if *logged {
        return;
    }
    let Some(handle) = gltf_handle.as_ref() else {
        return;
    };
    if let Some(gltf) = gltf_assets.get(&handle.0) {
        info!("PLAYER_LOG: --- GLTF CONTAINER INSPECTION ---");
        info!("PLAYER_LOG: Scenes count: {}", gltf.scenes.len());
        info!("PLAYER_LOG: Named scenes: {:?}", gltf.named_scenes.keys().collect::<Vec<_>>());
        info!("PLAYER_LOG: Named meshes: {:?}", gltf.named_meshes.keys().collect::<Vec<_>>());
        info!("PLAYER_LOG: Named nodes: {:?}", gltf.named_nodes.keys().collect::<Vec<_>>());
        info!("PLAYER_LOG: Named animations: {:?}", gltf.named_animations.keys().collect::<Vec<_>>());
        info!("PLAYER_LOG: Animations count: {}", gltf.animations.len());
        *logged = true;
    }
}