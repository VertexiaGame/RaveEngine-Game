use bevy::prelude::*;
use bevy::picking::mesh_picking::ray_cast::SimplifiedMesh;
use crate::common::game::bricks::components::Brick;
use crate::studio::tools::{Selection, ToolState, HoverState, DragState};

#[derive(Component)]
pub struct ToolGizmo {
    pub axis: Vec3,
    pub tool: ToolState,
    pub target: Entity,
}

pub struct GizmoAssets {
    materials: [Handle<StandardMaterial>; 3],
    move_mesh: Handle<Mesh>,
    size_mesh: Handle<Mesh>,
    rotate_mesh: Handle<Mesh>,
    rotate_pick_mesh: Handle<Mesh>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct GizmoConfiguration {
    selected_entity: Option<Entity>,
    tool: ToolState,
    physics_state: crate::common::game::physics::PhysicsSimulationState,
    playtesting: bool,
}

fn configuration_changed(
    previous: &mut Option<GizmoConfiguration>,
    current: GizmoConfiguration,
) -> bool {
    if previous.as_ref() == Some(&current) {
        return false;
    }
    *previous = Some(current);
    true
}

fn ensure_gizmo_assets<'a>(
    cache: &'a mut Option<GizmoAssets>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> &'a GizmoAssets {
    cache.get_or_insert_with(|| GizmoAssets {
        materials: [
            materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.0, 0.0), unlit: true, ..default() }),
            materials.add(StandardMaterial { base_color: Color::srgb(0.0, 1.0, 0.0), unlit: true, ..default() }),
            materials.add(StandardMaterial { base_color: Color::srgb(0.0, 0.0, 1.0), unlit: true, ..default() }),
        ],
        move_mesh: meshes.add(Cone { radius: 0.4, height: 1.0 }),
        size_mesh: meshes.add(Sphere::new(0.4)),
        rotate_mesh: meshes.add(Torus { minor_radius: 0.1, major_radius: 3.5 }),
        rotate_pick_mesh: meshes.add(Torus { minor_radius: 0.4, major_radius: 3.5 }),
    })
}

pub(crate) fn update_gizmos(
    mut commands: Commands,
    selection: Res<Selection>,
    tool_state: Res<State<ToolState>>,
    physics_state: Res<crate::common::game::physics::PhysicsSimulationState>,
    playtest: Option<Res<crate::client::PlaytestState>>,
    gizmos: Query<Entity, With<ToolGizmo>>,
    image_children: Query<Option<&ChildOf>, With<crate::common::game::assets::components::Image>>,
    bricks: Query<(), With<Brick>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmo_assets: Local<Option<GizmoAssets>>,
    mut previous_configuration: Local<Option<GizmoConfiguration>>,
) {
    let playtesting_active = playtest.map_or(false, |p| p.active);
    let configuration = GizmoConfiguration {
        selected_entity: selection.entity,
        tool: *tool_state.get(),
        physics_state: *physics_state,
        playtesting: playtesting_active,
    };
    if !configuration_changed(&mut previous_configuration, configuration) {
        return;
    }

    if playtesting_active {
        for entity in &gizmos {
            commands.entity(entity).despawn();
        }
        return;
    }

    for entity in &gizmos {
        commands.entity(entity).despawn();
    }

    if *physics_state == crate::common::game::physics::PhysicsSimulationState::Running {
        return;
    }

    let Some(selected_entity) = selection.entity else { return };
    let tool = *tool_state.get();

    if tool == ToolState::None { return; }

    if let Ok(Some(child_of)) = image_children.get(selected_entity) {
        if bricks.get(child_of.parent()).is_ok() {
            return;
        }
    }

    let assets = ensure_gizmo_assets(&mut gizmo_assets, &mut meshes, &mut materials);
    let [mat_x, mat_y, mat_z] = &assets.materials;

    let axes = [
        (Vec3::X, mat_x.clone()), (-Vec3::X, mat_x.clone()),
        (Vec3::Y, mat_y.clone()), (-Vec3::Y, mat_y.clone()),
        (Vec3::Z, mat_z.clone()), (-Vec3::Z, mat_z.clone()),
    ];

    match tool {
        ToolState::Move => {
            for (axis, mat) in axes {
                commands.spawn((
                    Mesh3d(assets.move_mesh.clone()),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    ToolGizmo { axis, tool, target: selected_entity },
                    Pickable::default(),
                    bevy::camera::visibility::RenderLayers::layer(1),
                ));
            }
        }
        ToolState::Size => {
            for (axis, mat) in axes {
                commands.spawn((
                    Mesh3d(assets.size_mesh.clone()),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    ToolGizmo { axis, tool, target: selected_entity },
                    Pickable::default(),
                    bevy::camera::visibility::RenderLayers::layer(1),
                ));
            }
        }
        ToolState::Rotate => {
            let rot_axes = [(Vec3::X, mat_x.clone()), (Vec3::Y, mat_y.clone()), (Vec3::Z, mat_z.clone())];
            for (axis, mat) in rot_axes {
                commands.spawn((
                    Mesh3d(assets.rotate_mesh.clone()),
                    SimplifiedMesh(assets.rotate_pick_mesh.clone()),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    ToolGizmo { axis, tool, target: selected_entity },
                    Pickable::default(),
                    bevy::camera::visibility::RenderLayers::layer(1),
                ));
            }
        }
        _ => {}
    }
}

pub fn sync_gizmos(
    mut gizmos: Query<(Entity, &mut Transform, &ToolGizmo)>,
    bricks: Query<(&GlobalTransform, Option<&crate::common::game::bricks::components::BrickShapeComponent>), With<Brick>>,
    images: Query<(&GlobalTransform, Option<&ChildOf>), (With<crate::common::game::assets::components::Image>, Without<Brick>)>,
    camera_query: Query<&GlobalTransform, (With<Camera3d>, Without<ToolGizmo>, Without<Brick>)>,
    selection: Res<Selection>,
    hover_state: Res<HoverState>,
    drag_state: Res<DragState>,
) {
    let camera_pos = camera_query.iter().next().map(|t| t.translation()).unwrap_or(Vec3::ZERO);

    for (entity, mut transform, gizmo) in &mut gizmos {
        if let Ok((brick_global, _)) = bricks.get(gizmo.target) {
            let (group_center, group_half) = match selection_bounds(&selection.entities, &bricks) {
                Some((min, max)) => ((min + max) * 0.5, (max - min) * 0.5),
                None => (brick_global.translation(), Vec3::ZERO),
            };
            place_gizmo(
                &mut transform,
                gizmo,
                group_center,
                group_half,
                brick_global.rotation(),
                camera_pos,
                entity,
                &hover_state,
                &drag_state,
            );
            continue;
        }
        if let Ok((image_global, image_child_of)) = images.get(gizmo.target) {
            if image_child_of.is_some_and(|co| bricks.contains(co.parent())) {
                continue;
            }
            place_gizmo(
                &mut transform,
                gizmo,
                image_global.translation(),
                image_world_half_extents(image_global),
                image_global.rotation(),
                camera_pos,
                entity,
                &hover_state,
                &drag_state,
            );
        }
    }
}

fn place_gizmo(
    transform: &mut Transform,
    gizmo: &ToolGizmo,
    group_center: Vec3,
    group_half: Vec3,
    global_rotation: Quat,
    camera_pos: Vec3,
    entity: Entity,
    hover_state: &HoverState,
    drag_state: &DragState,
) {
    let face_offset = gizmo.axis.abs().dot(group_half);

    let dist = camera_pos.distance(group_center);
    let distance_scale = (dist / 17.32).min(2.5);
    let base_scale = distance_scale;

    if gizmo.tool == ToolState::Rotate {
        transform.translation = group_center;
    } else {
        let offset = face_offset + 0.6 * distance_scale;
        transform.translation = group_center + global_rotation.mul_vec3(gizmo.axis * offset);
    }

    transform.rotation = global_rotation * Quat::from_rotation_arc(Vec3::Y, gizmo.axis);

    let is_hovered = hover_state.hovered_gizmo == Some(entity);
    let is_dragged = drag_state.active && drag_state.gizmo_entity == Some(entity);
    let state_multiplier = if is_hovered || is_dragged {
        if gizmo.tool == ToolState::Rotate {
            1.02
        } else {
            1.3
        }
    } else {
        1.0
    };

    transform.scale = Vec3::splat(base_scale * state_multiplier);
}

fn image_world_half_extents(global: &GlobalTransform) -> Vec3 {
    let rotation = global.rotation();
    let scaled = Vec3::splat(0.5) * global.scale();
    let local_x = rotation.mul_vec3(Vec3::X);
    let local_y = rotation.mul_vec3(Vec3::Y);
    let local_z = rotation.mul_vec3(Vec3::Z);
    Vec3::new(
        local_x.x.abs() * scaled.x + local_y.x.abs() * scaled.y + local_z.x.abs() * scaled.z,
        local_x.y.abs() * scaled.x + local_y.y.abs() * scaled.y + local_z.y.abs() * scaled.z,
        local_x.z.abs() * scaled.x + local_y.z.abs() * scaled.y + local_z.z.abs() * scaled.z,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuses_gizmo_assets() {
        let mut cache = None;
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();

        ensure_gizmo_assets(&mut cache, &mut meshes, &mut materials);
        let mesh_count = meshes.len();
        let material_count = materials.len();
        ensure_gizmo_assets(&mut cache, &mut meshes, &mut materials);

        assert_eq!(meshes.len(), mesh_count);
        assert_eq!(materials.len(), material_count);
        assert_eq!(mesh_count, 4);
        assert_eq!(material_count, 3);
    }

    #[test]
    fn rebuilds_only_when_configuration_changes() {
        let configuration = GizmoConfiguration {
            selected_entity: Some(Entity::from_bits(1)),
            tool: ToolState::Move,
            physics_state: crate::common::game::physics::PhysicsSimulationState::Stopped,
            playtesting: false,
        };
        let mut previous = None;

        assert!(configuration_changed(&mut previous, configuration));
        assert!(!configuration_changed(&mut previous, configuration));
        assert!(configuration_changed(
            &mut previous,
            GizmoConfiguration {
                tool: ToolState::Rotate,
                ..configuration
            },
        ));
    }
}

fn draw_outline_recursive(
    entity: Entity,
    bricks: &Query<(&GlobalTransform, Option<&crate::common::game::bricks::components::BrickShapeComponent>, Option<&Children>), With<Brick>>,
    gizmos: &mut Gizmos,
) {
    if let Ok((global_transform, shape_opt, children_opt)) = bricks.get(entity) {
        let (scale, rotation, translation) = global_transform.to_scale_rotation_translation();
        let shape = shape_opt.map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);

        match shape {
            crate::common::game::bricks::components::BrickShape::Block => {
                let outline_scale = scale * Vec3::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28);
                let outline_transform = Transform {
                    translation,
                    rotation,
                    scale: outline_scale,
                };
                gizmos.cube(outline_transform, Color::srgb(1.0, 1.0, 1.0));
            }
            crate::common::game::bricks::components::BrickShape::Sphere => {
                let base_radius = 1.0 * 0.28;
                
                let half_size_xy = Vec2::new(scale.x * base_radius, scale.y * base_radius);
                let isometry_xy = Isometry3d::new(translation, rotation);
                gizmos.ellipse(isometry_xy, half_size_xy, Color::srgb(1.0, 1.0, 1.0));

                let half_size_yz = Vec2::new(scale.z * base_radius, scale.y * base_radius);
                let isometry_yz = Isometry3d::new(translation, rotation * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
                gizmos.ellipse(isometry_yz, half_size_yz, Color::srgb(1.0, 1.0, 1.0));

                let half_size_xz = Vec2::new(scale.x * base_radius, scale.z * base_radius);
                let isometry_xz = Isometry3d::new(translation, rotation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
                gizmos.ellipse(isometry_xz, half_size_xz, Color::srgb(1.0, 1.0, 1.0));
            }
        }

        if let Some(children) = children_opt {
            for child in children.iter() {
                draw_outline_recursive(child, bricks, gizmos);
            }
        }
    }
}

pub fn draw_selection_outline(
    selection: Res<Selection>,
    physics_state: Res<crate::common::game::physics::PhysicsSimulationState>,
    playtest: Option<Res<crate::client::PlaytestState>>,
    bricks: Query<(&GlobalTransform, Option<&crate::common::game::bricks::components::BrickShapeComponent>, Option<&Children>), With<Brick>>,
    bounds_query: Query<(&GlobalTransform, Option<&crate::common::game::bricks::components::BrickShapeComponent>), With<Brick>>,
    mut gizmos: Gizmos,
) {
    if *physics_state == crate::common::game::physics::PhysicsSimulationState::Running {
        return;
    }
    let playtesting_active = playtest.map_or(false, |p| p.active);
    if playtesting_active {
        return;
    }
    for &selected_entity in &selection.entities {
        draw_outline_recursive(selected_entity, &bricks, &mut gizmos);
    }
    if selection.entities.len() > 1 {
        if let Some((min, max)) = selection_bounds(&selection.entities, &bounds_query) {
            let size = max - min;
            if size.length_squared() > 1.0e-6 {
                let transform = Transform {
                    translation: (min + max) * 0.5,
                    rotation: Quat::IDENTITY,
                    scale: size,
                };
                gizmos.cube(transform, Color::srgb(0.20, 0.55, 1.0));
            }
        }
    }
}

pub(crate) fn selection_bounds(
    entities: &[Entity],
    bricks: &Query<(&GlobalTransform, Option<&crate::common::game::bricks::components::BrickShapeComponent>), With<Brick>>,
) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut any = false;
    for &entity in entities {
        if let Ok((global, shape_opt)) = bricks.get(entity) {
            let shape = shape_opt.map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
            let base_extents = match shape {
                crate::common::game::bricks::components::BrickShape::Block => Vec3::new(2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28),
                crate::common::game::bricks::components::BrickShape::Sphere => Vec3::splat(1.0 * 0.28),
            };
            let pos = global.translation();
            let rot = global.rotation();
            let scale = global.scale();
            let scaled = base_extents * scale;
            let local_x = rot.mul_vec3(Vec3::X);
            let local_y = rot.mul_vec3(Vec3::Y);
            let local_z = rot.mul_vec3(Vec3::Z);
            let half = Vec3::new(
                local_x.x.abs() * scaled.x + local_y.x.abs() * scaled.y + local_z.x.abs() * scaled.z,
                local_x.y.abs() * scaled.x + local_y.y.abs() * scaled.y + local_z.y.abs() * scaled.z,
                local_x.z.abs() * scaled.x + local_y.z.abs() * scaled.y + local_z.z.abs() * scaled.z,
            );
            min = min.min(pos - half);
            max = max.max(pos + half);
            any = true;
        }
    }
    if any {
        Some((min, max))
    } else {
        None
    }
}
