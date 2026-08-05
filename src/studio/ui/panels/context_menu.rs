use bevy::prelude::*;
use bevy_egui::egui;
use crate::studio::tools::Selection;
use crate::common::game::bricks::components::Brick;
use crate::studio::ui::CopiedEntityBuffer;
use bevy::pbr::ExtendedMaterial;

pub fn draw_entity_context_menu(
    ui: &mut egui::Ui,
    entity: Entity,
    commands: &mut Commands,
    selection: &mut ResMut<Selection>,
    copiedbuffer: &mut CopiedEntityBuffer,
    entities_query: &Query<(
        Entity,
        &mut Transform,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&mut crate::common::game::bricks::components::BrickShapeComponent>,
        &GlobalTransform,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
        Option<&mut crate::common::game::bricks::components::BrickPhysics>,
    ), Without<Camera3d>>,
    history: &mut ResMut<crate::studio::tools::UndoRedoHistory>,
    studs_query: &Query<&crate::common::game::bricks::components::BrickStuds>,
) -> bool {
    let mut closed = false;
    if ui.button("Copy").clicked() {
        if let Ok((_, transform, name, _, _, brick_opt, shape_opt, _, mesh_opt, mat_opt, studs_mat_opt, phys_opt)) = entities_query.get(entity) {
            copiedbuffer.transform = Some(*transform);
            copiedbuffer.mesh = mesh_opt.cloned();
            copiedbuffer.material = mat_opt.cloned();
            copiedbuffer.studs_material = studs_mat_opt.cloned();
            copiedbuffer.name = Some(name.to_string());
            copiedbuffer.is_brick = brick_opt.is_some();
            copiedbuffer.shape = shape_opt.as_ref().map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
            copiedbuffer.physics = phys_opt.cloned();
            copiedbuffer.show_studs = studs_query.get(entity).map(|s| s.enabled).unwrap_or(true);
        }
        ui.close();
        closed = true;
    }
    if copiedbuffer.transform.is_some() {
        if ui.button("Paste").clicked() {
            let transform = copiedbuffer.transform.unwrap();
            let name = copiedbuffer.name.clone().unwrap();
            let mut newtransform = transform;
            newtransform.translation += Vec3::new(2.0, 0.0, 2.0);

            let new_entity = commands.spawn((
                newtransform,
                Name::new(format!("{} - Copy", name)),
                Pickable::default(),
            )).id();

            if let Some(ref mesh) = copiedbuffer.mesh {
                commands.entity(new_entity).insert(mesh.clone());
            }
            if let Some(ref mat) = copiedbuffer.material {
                commands.entity(new_entity).insert(mat.clone());
            }
            if let Some(ref studs_mat) = copiedbuffer.studs_material {
                commands.entity(new_entity).insert(studs_mat.clone());
            }
            if copiedbuffer.is_brick {
                commands.entity(new_entity).insert((
                    Brick,
                    crate::common::game::bricks::components::BrickShapeComponent { shape: copiedbuffer.shape },
                    crate::common::game::bricks::components::BrickStuds { enabled: copiedbuffer.show_studs },
                ));
            }
            if let Some(phys) = copiedbuffer.physics {
                commands.entity(new_entity).insert(phys.clone());
            } else if copiedbuffer.is_brick {
                commands.entity(new_entity).insert(crate::common::game::bricks::components::BrickPhysics::default());
            }

            let data = crate::common::game::bricks::data::BrickData {
                transform: newtransform,
                name: format!("{} - Copy", name),
                is_brick: copiedbuffer.is_brick,
                shape: copiedbuffer.shape,
                mesh: copiedbuffer.mesh.clone(),
                standard_material: copiedbuffer.material.clone(),
                studs_material: copiedbuffer.studs_material.clone(),
                parent: None,
                physics: copiedbuffer.physics.clone(),
                studs: copiedbuffer.show_studs,
            };

            history.push_command(crate::studio::tools::UndoCommand::Spawn {
                entity: new_entity,
                data,
            });

            ui.close();
            closed = true;
        }
    }
    if ui.button("Duplicate").clicked() {
        if let Ok((_, transform, name, child_of_opt, _, brick_opt, shape_opt, _, mesh_opt, mat_opt, studs_mat_opt, phys_opt)) = entities_query.get(entity) {
            let newtransform = *transform;

            let new_entity = commands.spawn((
                newtransform,
                Name::new(format!("{} - Copy", name.as_str())),
                Pickable::default(),
            )).id();

            if let Some(mesh) = mesh_opt {
                commands.entity(new_entity).insert(mesh.clone());
            }
            if let Some(mat) = mat_opt {
                commands.entity(new_entity).insert(mat.clone());
            }
            if let Some(studs_mat) = studs_mat_opt {
                commands.entity(new_entity).insert(studs_mat.clone());
            }
            let shape = shape_opt.as_ref().map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
            if brick_opt.is_some() {
                commands.entity(new_entity).insert((
                    Brick,
                    crate::common::game::bricks::components::BrickShapeComponent { shape },
                    crate::common::game::bricks::components::BrickStuds { enabled: studs_query.get(entity).map(|s| s.enabled).unwrap_or(true) },
                ));
            }
            if let Some(phys) = phys_opt {
                commands.entity(new_entity).insert(phys.clone());
            } else if brick_opt.is_some() {
                commands.entity(new_entity).insert(crate::common::game::bricks::components::BrickPhysics::default());
            }

            let parent_entity = child_of_opt.map(|co| co.parent());
            if let Some(p) = parent_entity {
                if let Ok(mut p_cmd) = commands.get_entity(p) {
                    p_cmd.add_child(new_entity);
                }
            }

            let data = crate::common::game::bricks::data::BrickData {
                transform: newtransform,
                name: format!("{} - Copy", name.as_str()),
                is_brick: brick_opt.is_some(),
                shape,
                mesh: mesh_opt.cloned(),
                standard_material: mat_opt.cloned(),
                studs_material: studs_mat_opt.cloned(),
                parent: parent_entity,
                physics: phys_opt.cloned(),
                studs: studs_query.get(entity).map(|s| s.enabled).unwrap_or(true),
            };

            history.push_command(crate::studio::tools::UndoCommand::Spawn {
                entity: new_entity,
                data,
            });

            ui.close();
            closed = true;
        }
    }
    if ui.button("Delete").clicked() {
        if let Some(data) = crate::common::game::bricks::data::capture_brick_data(entity, entities_query, studs_query) {
            history.push_command(crate::studio::tools::UndoCommand::Delete {
                entity,
                data,
            });
        }
        commands.entity(entity).try_despawn();
        if selection.entity == Some(entity) {
            selection.entity = None;
            selection.entities.clear();
        }
        ui.close();
        closed = true;
    }
    closed
}