use bevy::prelude::*;
use bevy::picking::mesh_picking::ray_cast::SimplifiedMesh;
use crate::common::components::Brick;
use crate::studio::tools::{Selection, ToolState, HoverState, DragState};

#[derive(Component)]
pub struct ToolGizmo {
    pub axis: Vec3,
    pub tool: ToolState,
    pub target: Entity,
}

pub fn update_gizmos(
    mut commands: Commands,
    selection: Res<Selection>,
    tool_state: Res<State<ToolState>>,
    gizmos: Query<Entity, With<ToolGizmo>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !selection.is_changed() && !tool_state.is_changed() {
        return;
    }

    for entity in &gizmos {
        commands.entity(entity).despawn();
    }

    let Some(selected_entity) = selection.entity else { return };
    let tool = *tool_state.get();

    if tool == ToolState::None { return; }

    let mat_x = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.0, 0.0), unlit: true, ..default() });
    let mat_y = materials.add(StandardMaterial { base_color: Color::srgb(0.0, 1.0, 0.0), unlit: true, ..default() });
    let mat_z = materials.add(StandardMaterial { base_color: Color::srgb(0.0, 0.0, 1.0), unlit: true, ..default() });

    let axes = [
        (Vec3::X, mat_x.clone()), (-Vec3::X, mat_x.clone()),
        (Vec3::Y, mat_y.clone()), (-Vec3::Y, mat_y.clone()),
        (Vec3::Z, mat_z.clone()), (-Vec3::Z, mat_z.clone()),
    ];

    match tool {
        ToolState::Move => {
            let mesh = meshes.add(Cone { radius: 0.4, height: 1.0 });
            for (axis, mat) in axes {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    ToolGizmo { axis, tool, target: selected_entity },
                    Pickable::default(),
                    bevy::camera::visibility::RenderLayers::layer(1),
                ));
            }
        }
        ToolState::Size => {
            let mesh = meshes.add(Sphere::new(0.4));
            for (axis, mat) in axes {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    ToolGizmo { axis, tool, target: selected_entity },
                    Pickable::default(),
                    bevy::camera::visibility::RenderLayers::layer(1),
                ));
            }
        }
        ToolState::Rotate => {
            let mesh = meshes.add(Torus { minor_radius: 0.1, major_radius: 3.5 });
            let picking_mesh = meshes.add(Torus { minor_radius: 0.4, major_radius: 3.5 });
            let rot_axes = [(Vec3::X, mat_x), (Vec3::Y, mat_y), (Vec3::Z, mat_z)];
            for (axis, mat) in rot_axes {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    SimplifiedMesh(picking_mesh.clone()),
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
    bricks: Query<&GlobalTransform, (With<Brick>, Without<ToolGizmo>)>,
    camera_query: Query<&GlobalTransform, (With<Camera3d>, Without<ToolGizmo>, Without<Brick>)>,
    hover_state: Res<HoverState>,
    drag_state: Res<DragState>,
) {
    let camera_pos = camera_query.iter().next().map(|t| t.translation()).unwrap_or(Vec3::ZERO);

    for (entity, mut transform, gizmo) in &mut gizmos {
        if let Ok(brick_global) = bricks.get(gizmo.target) {
            let base_extents = Vec3::new(2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28);
            let global_scale = brick_global.scale();
            let scaled_extents = base_extents * global_scale;
            let face_offset = gizmo.axis.abs().dot(scaled_extents);

            let global_translation = brick_global.translation();
            let global_rotation = brick_global.rotation();

            let dist = camera_pos.distance(global_translation);
            let distance_scale = (dist / 17.32).min(2.5);
            let base_scale = distance_scale;

            if gizmo.tool == ToolState::Rotate {
                transform.translation = global_translation;
            } else {
                let offset = face_offset + 0.6 * distance_scale;
                transform.translation = global_translation + global_rotation.mul_vec3(gizmo.axis * offset);
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
    }
}

fn draw_outline_recursive(
    entity: Entity,
    bricks: &Query<(&GlobalTransform, Option<&Children>), With<Brick>>,
    gizmos: &mut Gizmos,
) {
    if let Ok((global_transform, children_opt)) = bricks.get(entity) {
        let (scale, rotation, translation) = global_transform.to_scale_rotation_translation();
        let outline_scale = scale * Vec3::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28);
        let outline_transform = Transform {
            translation,
            rotation,
            scale: outline_scale,
        };
        gizmos.cube(outline_transform, Color::srgb(1.0, 1.0, 1.0));

        if let Some(children) = children_opt {
            for child in children.iter() {
                draw_outline_recursive(child, bricks, gizmos);
            }
        }
    }
}

pub fn draw_selection_outline(
    selection: Res<Selection>,
    bricks: Query<(&GlobalTransform, Option<&Children>), With<Brick>>,
    mut gizmos: Gizmos,
) {
    let Some(selected_entity) = selection.entity else { return };
    draw_outline_recursive(selected_entity, &bricks, &mut gizmos);
}