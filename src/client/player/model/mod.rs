use bevy::prelude::*;
use crate::client::LocalPlayer;
use lightyear::prelude::Replicate;

#[derive(Resource)]
pub struct PlayerGltfHandle(pub Handle<bevy::gltf::Gltf>);

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