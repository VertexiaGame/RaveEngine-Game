use bevy::prelude::*;
use bevy::animation::{AnimatedBy, AnimationTargetId};
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use crate::client::player::model::PlayerGltfHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimbPart {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

#[derive(Component)]
pub struct LimbComponent {
    pub part_type: LimbPart,
}

#[derive(Component)]
pub struct LimbPlayerMarker {
    pub part_type: LimbPart,
    pub player_entity: Entity,
}

#[derive(Resource)]
pub struct LimbAnimations {
    pub graphs: std::collections::HashMap<LimbPart, Handle<AnimationGraph>>,
    pub indices: std::collections::HashMap<LimbPart, Vec<AnimationNodeIndex>>,
}

#[derive(Resource, Default)]
pub struct PlayerAnimationGraphLoaded {
    pub graph: Option<Handle<AnimationGraph>>,
    pub indices: Vec<AnimationNodeIndex>,
}

#[derive(Resource, Default)]
pub struct AvatarAnimationsRetargeted {
    pub retargeted_clips: std::collections::HashSet<Handle<AnimationClip>>,
}

#[derive(Component)]
pub struct PlayerAnimationMarker {
    pub player_entity: Entity,
    pub current_index: Option<AnimationNodeIndex>,
}

#[derive(Component, Default)]
pub struct PlayerVelocityTracker {
    pub last_position: Vec3,
    pub velocity: Vec3,
    pub is_grounded: bool,
}

pub fn add_missing_animation_players(
    mut commands: Commands,
    query: Query<(Entity, &Name), (Without<AnimationPlayer>, With<ChildOf>)>,
    parent_query: Query<&ChildOf>,
    players: Query<Entity, With<crate::common::net::components::Player>>,
) {
    for (entity, name) in &query {
        if name.as_str().contains("Armature") {
            let mut current = entity;
            let mut under_player = false;
            while let Ok(child_of) = parent_query.get(current) {
                let parent = child_of.parent();
                if players.get(parent).is_ok() {
                    under_player = true;
                    break;
                }
                current = parent;
            }

            if under_player {
                commands.entity(entity).insert(AnimationPlayer::default());
                info!("PLAYER_LOG: Manually attached AnimationPlayer to 'Armature' Entity {:?}", entity);
            }
        }
    }
}

pub fn build_avatar_animation_graph(
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    gltf_handle: Option<Res<PlayerGltfHandle>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut graph_loaded: ResMut<PlayerAnimationGraphLoaded>,
) {
    if graph_loaded.graph.is_some() {
        return;
    }
    let Some(handle) = gltf_handle.as_ref() else {
        return;
    };
    let Some(gltf) = gltf_assets.get(&handle.0) else {
        return;
    };

    let find_clip_robust = |gltf: &bevy::gltf::Gltf, targets: &[&str], fallback_idx: usize| -> Option<Handle<AnimationClip>> {
        for &target in targets {
            if let Some(clip) = gltf.named_animations.get(target) {
                return Some(clip.clone());
            }
            for (anim_name, clip) in &gltf.named_animations {
                let anim_name_str = &**anim_name;
                if anim_name_str.contains(target) {
                    return Some(clip.clone());
                }
            }
            let armature_format = format!("Armature|{}", target);
            if let Some(clip) = gltf.named_animations.get(armature_format.as_str()) {
                return Some(clip.clone());
            }
        }
        if fallback_idx < gltf.animations.len() {
            return Some(gltf.animations[fallback_idx].clone());
        }
        None
    };

    let idle = find_clip_robust(&gltf, &["Scene.001", "Idle", "idle"], 0);
    let walk = find_clip_robust(&gltf, &["Scene.002", "Walk", "walk"], 1);
    let jump = find_clip_robust(&gltf, &["Scene.003", "Jump", "jump"], 2);
    let fall = find_clip_robust(&gltf, &["Scene.004", "Falling", "fall", "falling"], 3);

    let Some(idle) = idle else {
        return;
    };
    let walk = walk.unwrap_or_else(|| idle.clone());
    let jump = jump.unwrap_or_else(|| idle.clone());
    let fall = fall.unwrap_or_else(|| idle.clone());

    let (graph, indices) = AnimationGraph::from_clips([idle, walk, jump, fall]);
    let graph_handle = graphs.add(graph);

    graph_loaded.graph = Some(graph_handle);
    graph_loaded.indices = indices;
}

pub fn retarget_avatar_clips(
    mut retargeted: ResMut<AvatarAnimationsRetargeted>,
    gltf_handle: Option<Res<PlayerGltfHandle>>,
    gltf_assets: Res<Assets<bevy::gltf::Gltf>>,
    mut clips: ResMut<Assets<AnimationClip>>,
    armatures: Query<(Entity, &Name), With<ChildOf>>,
    children_query: Query<&Children>,
    names_query: Query<&Name>,
) {
    let Some(handle) = gltf_handle.as_ref() else {
        return;
    };
    let Some(gltf) = gltf_assets.get(&handle.0) else {
        return;
    };

    let mut armature_root = None;
    for (entity, name) in &armatures {
        if name.as_str().contains("Armature") {
            armature_root = Some(entity);
            break;
        }
    }

    let Some(root_entity) = armature_root else {
        return;
    };

    let mut mappings = std::collections::HashMap::new();

    let mut current_path = Vec::new();
    collect_bone_paths(
        root_entity,
        &mut current_path,
        &mut mappings,
        &children_query,
        &names_query,
    );

    if mappings.len() < 10 {
        return;
    }

    let mut retargeted_any = false;
    for clip_handle in &gltf.animations {
        if retargeted.retargeted_clips.contains(clip_handle) {
            continue;
        }
        if let Some(mut clip) = clips.get_mut(clip_handle) {
            retarget_clip_in_place(&mut *clip, &mappings);
            retargeted.retargeted_clips.insert(clip_handle.clone());
            retargeted_any = true;
        }
    }

    if retargeted_any {
        info!("PLAYER_LOG: Successfully retargeted {} animation clips in-place! Total retargeted: {}", 
            retargeted.retargeted_clips.len(), gltf.animations.len());
    }
}

fn collect_bone_paths(
    entity: Entity,
    current_path: &mut Vec<Name>,
    mappings: &mut std::collections::HashMap<AnimationTargetId, AnimationTargetId>,
    children_query: &Query<&Children>,
    names_query: &Query<&Name>,
) {
    let Some(name) = names_query.get(entity).ok() else {
        return;
    };

    let is_root = name.as_str().contains("Armature");
    if !is_root {
        current_path.push(name.clone());
    }

    if !current_path.is_empty() {
        let standard_id = AnimationTargetId::from_names(current_path.iter());

        let alternative_roots = ["Armature", "Armature.001", "Armature.002", "Armature.003", "Armature.004", "armature"];
        let intermediate_armatures = ["Armature", "Armature.001", "Armature.002", "Armature.003", "Armature.004", "Armature.005", "Armature.006", "Armature.007", "armature"];

        mappings.insert(standard_id, standard_id);

        for &alt_root in &alternative_roots {
            let mut alt_path1 = Vec::new();
            alt_path1.push(Name::new(alt_root.to_string()));
            for name_part in current_path.iter() {
                alt_path1.push(name_part.clone());
            }
            let alt_id1 = AnimationTargetId::from_names(alt_path1.iter());
            mappings.insert(alt_id1, standard_id);

            for &inter in &intermediate_armatures {
                let mut alt_path2 = Vec::new();
                alt_path2.push(Name::new(alt_root.to_string()));
                alt_path2.push(Name::new(inter.to_string()));
                for name_part in current_path.iter() {
                    alt_path2.push(name_part.clone());
                }
                let alt_id2 = AnimationTargetId::from_names(alt_path2.iter());
                mappings.insert(alt_id2, standard_id);
            }
        }
    }

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            collect_bone_paths(child, current_path, mappings, children_query, names_query);
        }
    }

    if !is_root {
        current_path.pop();
    }
}

fn retarget_clip_in_place(
    clip: &mut AnimationClip,
    mappings: &std::collections::HashMap<AnimationTargetId, AnimationTargetId>,
) {
    let old_curves = std::mem::take(clip.curves_mut());
    let mut_curves = clip.curves_mut();
    for (old_target_id, curves) in old_curves {
        let new_target_id = mappings.get(&old_target_id).copied().unwrap_or(old_target_id);
        mut_curves.insert(new_target_id, curves);
    }
}

pub fn init_player_animations(
    mut commands: Commands,
    mut anim_players: Query<(Entity, &ChildOf), (With<AnimationPlayer>, Without<AnimationGraphHandle>)>,
    parent_query: Query<&ChildOf>,
    players: Query<Entity, With<crate::common::net::components::Player>>,
    player_anims: Res<PlayerAnimationGraphLoaded>,
    children_query: Query<&Children>,
    names_query: Query<&Name>,
) {
    let Some(graph_handle) = &player_anims.graph else {
        return;
    };

    for (player_entity, child_of) in &mut anim_players {
        let mut current = child_of.parent();
        let mut player_entity_opt = None;

        loop {
            if players.get(current).is_ok() {
                player_entity_opt = Some(current);
                break;
            }
            if let Ok(next_parent) = parent_query.get(current) {
                current = next_parent.parent();
            } else {
                break;
            }
        }

        if let Some(p_entity) = player_entity_opt {
            commands.entity(player_entity).insert((
                AnimationGraphHandle(graph_handle.clone()),
                AnimationTransitions::default(),
                PlayerAnimationMarker { player_entity: p_entity, current_index: None },
            ));
            info!("PLAYER_LOG: Successfully linked unified AnimationPlayer {:?} to player {:?}.", player_entity, p_entity);

            let mut current_path = Vec::new();
            insert_animation_targets_recursive(
                &mut commands,
                player_entity,
                player_entity,
                &mut current_path,
                &children_query,
                &names_query,
            );
            info!("PLAYER_LOG: Successfully populated AnimationTargetId and AnimatedBy hierarchy under {:?}", player_entity);
        }
    }
}

fn insert_animation_targets_recursive(
    commands: &mut Commands,
    entity: Entity,
    armature_entity: Entity,
    current_path: &mut Vec<Name>,
    children_query: &Query<&Children>,
    names_query: &Query<&Name>,
) {
    let Some(name) = names_query.get(entity).ok() else {
        return;
    };

    let is_root = name.as_str().contains("Armature");
    if !is_root {
        current_path.push(name.clone());
    }

    if !current_path.is_empty() {
        let target_id = AnimationTargetId::from_names(current_path.iter());
        commands.entity(entity).insert((
            AnimatedBy(armature_entity),
            target_id,
        ));
    } else {
        commands.entity(entity).insert(AnimatedBy(armature_entity));
    }

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            insert_animation_targets_recursive(
                commands,
                child,
                armature_entity,
                current_path,
                children_query,
                names_query,
            );
        }
    }

    if !is_root {
        current_path.pop();
    }
}

type PlayerGrid = std::collections::HashMap<(i32, i32), Vec<(Entity, Transform)>>;

fn grid_cell(position: Vec3) -> (i32, i32) {
    (position.x.floor() as i32, position.z.floor() as i32)
}

fn rebuild_player_grid(grid: &mut PlayerGrid, players: impl Iterator<Item = (Entity, Transform)>) {
    grid.clear();
    for (entity, transform) in players {
        grid.entry(grid_cell(transform.translation)).or_default().push((entity, transform));
    }
}

fn supported_by_player(entity: Entity, transform: &Transform, grid: &PlayerGrid) -> bool {
    let (cell_x, cell_z) = grid_cell(transform.translation);
    for x in (cell_x - 1)..=(cell_x + 1) {
        for z in (cell_z - 1)..=(cell_z + 1) {
            let Some(players) = grid.get(&(x, z)) else { continue };
            for &(other_entity, other_transform) in players {
                if other_entity == entity {
                    continue;
                }

                let local = other_transform.rotation.inverse().mul_vec3(transform.translation - other_transform.translation);
                let within_x = local.x.abs() <= 1.0 * 0.28 + 2.0 * 0.28 * 0.5;
                let within_z = local.z.abs() <= 0.5 * 0.28 + 1.0 * 0.28 * 0.5;
                let max_height = 2.5 * 0.28 + 2.5 * 0.28 + 0.15;
                if within_x && within_z && local.y >= 0.0 && local.y <= max_height {
                    return true;
                }
            }
        }
    }
    false
}

fn supported_by_world(entity: Entity, transform: &Transform, spatial_query: &SpatialQuery) -> bool {
    let filter = SpatialQueryFilter::default()
        .with_excluded_entities([entity])
        .with_mask(0b0001);
    spatial_query
        .cast_ray(
            transform.translation,
            Dir3::NEG_Y,
            2.5 * 0.28 + 0.15,
            true,
            &filter,
        )
        .is_some()
}

fn tracking_delta(delta_secs: f32) -> Option<f32> {
    (delta_secs > 0.0).then_some(delta_secs)
}

fn grounded_for_animation(tracker: &PlayerVelocityTracker) -> bool {
    tracker.is_grounded || tracker.velocity.y.abs() < 0.2
}

pub fn track_player_velocities(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, Option<&mut PlayerVelocityTracker>), With<crate::common::net::components::Player>>,
    spatial_query: SpatialQuery,
    time: Res<Time>,
    mut cached_players: Local<Vec<(Entity, Transform)>>,
    mut player_grid: Local<PlayerGrid>,
) {
    let Some(dt) = tracking_delta(time.delta_secs()) else { return };

    cached_players.clear();
    cached_players.extend(query.iter().map(|(e, t, _)| (e, *t)));
    rebuild_player_grid(&mut player_grid, cached_players.iter().copied());

    for (entity, transform, tracker_opt) in &mut query {
        let is_grounded = supported_by_world(entity, transform, &spatial_query)
            || supported_by_player(entity, transform, &player_grid);

        if let Some(mut tracker) = tracker_opt {
            let raw_velocity = (transform.translation - tracker.last_position) / dt;
            tracker.velocity = tracker.velocity.lerp(raw_velocity, 0.1);
            tracker.last_position = transform.translation;
            tracker.is_grounded = is_grounded;
        } else {
            commands.entity(entity).insert(PlayerVelocityTracker {
                last_position: transform.translation,
                velocity: Vec3::ZERO,
                is_grounded,
            });
        }
    }
}

pub fn animate_player(
    mut anim_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions, &mut PlayerAnimationMarker)>,
    players: Query<&PlayerVelocityTracker>,
    player_anims: Res<PlayerAnimationGraphLoaded>,
) {
    for (mut player, mut transitions, mut marker) in &mut anim_players {
        let Ok(tracker) = players.get(marker.player_entity) else {
            continue;
        };

        let velocity = tracker.velocity;
        let speed_xz = Vec2::new(velocity.x, velocity.z).length();

        if f32::is_nan(speed_xz) || player_anims.indices.len() < 4 {
            continue;
        }

        let jump_index = player_anims.indices[2];
        let fall_index = player_anims.indices[3];
        let walk_index = player_anims.indices[1];
        let idle_index = player_anims.indices[0];

        let is_jump_finished = player.animation(jump_index).map_or(false, |anim| anim.is_finished());

        let mut active_index = if !grounded_for_animation(tracker) {
            if velocity.y > 0.0 {
                jump_index
            } else {
                fall_index
            }
        } else if speed_xz > 0.5 {
            walk_index
        } else {
            idle_index
        };

        if marker.current_index == Some(jump_index) && !is_jump_finished {
            active_index = jump_index;
        }

        if marker.current_index != Some(active_index) {
            if active_index == jump_index {
                transitions.play(&mut player, active_index, std::time::Duration::from_millis(150)).replay();
            } else {
                transitions.play(&mut player, active_index, std::time::Duration::from_millis(250)).repeat();
            }
            marker.current_index = Some(active_index);
            info!("PLAYER_LOG: Animation state changed to NodeIndex {:?}", active_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avian3d::prelude::*;

    #[derive(Component)]
    struct ProbePlayer;

    #[derive(Resource, Default)]
    struct ProbeResult(bool);

    fn run_probe(
        spatial_query: SpatialQuery,
        player: Single<(Entity, &Transform), With<ProbePlayer>>,
        mut result: ResMut<ProbeResult>,
    ) {
        result.0 = supported_by_world(player.0, player.1, &spatial_query);
    }

    fn world_support(collider: Collider, transform: Transform, player_position: Vec3) -> bool {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), PhysicsPlugins::default()))
            .init_asset::<Mesh>()
            .init_resource::<ProbeResult>()
            .add_systems(Update, run_probe);
        app.world_mut().spawn((
            RigidBody::Static,
            collider,
            transform,
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
        ));
        app.world_mut().spawn((ProbePlayer, Transform::from_translation(player_position)));
        app.update();
        app.update();
        app.world().resource::<ProbeResult>().0
    }

    #[test]
    fn detects_stacked_players_in_neighboring_cells() {
        let mut world = World::new();
        let lower = world.spawn_empty().id();
        let upper = world.spawn_empty().id();
        let lower_transform = Transform::from_xyz(0.9, 0.0, 0.0);
        let upper_transform = Transform::from_xyz(0.9, 1.4, 0.0);
        let mut grid = PlayerGrid::default();
        rebuild_player_grid(&mut grid, [(lower, lower_transform), (upper, upper_transform)].into_iter());

        assert!(supported_by_player(upper, &upper_transform, &grid));
    }

    #[test]
    fn ignores_players_outside_the_support_area() {
        let mut world = World::new();
        let lower = world.spawn_empty().id();
        let upper = world.spawn_empty().id();
        let lower_transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let upper_transform = Transform::from_xyz(0.8, 1.4, 0.0);
        let mut grid = PlayerGrid::default();
        rebuild_player_grid(&mut grid, [(lower, lower_transform), (upper, upper_transform)].into_iter());

        assert!(!supported_by_player(upper, &upper_transform, &grid));
    }

    #[test]
    fn respects_rotated_player_support() {
        let mut world = World::new();
        let lower = world.spawn_empty().id();
        let upper = world.spawn_empty().id();
        let lower_transform = Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let upper_transform = Transform::from_xyz(0.0, 1.4, 0.4);
        let mut grid = PlayerGrid::default();
        rebuild_player_grid(&mut grid, [(lower, lower_transform), (upper, upper_transform)].into_iter());

        assert!(supported_by_player(upper, &upper_transform, &grid));
    }

    #[test]
    fn detects_flat_and_rotated_world_support() {
        assert!(world_support(
            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
            Transform::default(),
            Vec3::new(0.0, 2.5 * 0.28 + 0.5 * 0.28, 0.0),
        ));
        assert!(world_support(
            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
            Transform::from_rotation(Quat::from_rotation_z(0.2)),
            Vec3::new(0.0, 2.5 * 0.28 + 0.5 * 0.28, 0.0),
        ));
    }

    #[test]
    fn detects_sphere_support_and_airborne_players() {
        assert!(world_support(
            Collider::sphere(0.28),
            Transform::default(),
            Vec3::new(0.0, 2.5 * 0.28 + 0.28, 0.0),
        ));
        assert!(!world_support(
            Collider::sphere(0.28),
            Transform::default(),
            Vec3::new(0.0, 3.0, 0.0),
        ));
    }

    #[test]
    fn rejects_zero_delta_time() {
        assert_eq!(tracking_delta(0.0), None);
        assert_eq!(tracking_delta(-0.1), None);
        assert_eq!(tracking_delta(0.1), Some(0.1));
    }

    #[test]
    fn settles_animation_when_vertical_motion_stops() {
        let mut tracker = PlayerVelocityTracker {
            last_position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            is_grounded: false,
        };

        assert!(grounded_for_animation(&tracker));
        tracker.velocity.y = -0.3;
        assert!(!grounded_for_animation(&tracker));
        tracker.velocity.y = 0.3;
        assert!(!grounded_for_animation(&tracker));
        tracker.is_grounded = true;
        assert!(grounded_for_animation(&tracker));
    }
}
