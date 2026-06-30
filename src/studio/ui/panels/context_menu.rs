use bevy::prelude::*;
use bevy_egui::egui;
use crate::studio::tools::Selection;
use crate::common::components::Brick;
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
        &mut Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        &GlobalTransform,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>>,
    ), Without<Camera3d>>,
    history: &mut ResMut<crate::studio::tools::UndoRedoHistory>,
) -> bool {
    let mut closed = false;
    if ui.button("Copy").clicked() {
        if let Ok((_, transform, name, _, _, brick_opt, _, mesh_opt, mat_opt, studs_mat_opt)) = entities_query.get(entity) {
            copiedbuffer.transform = Some(*transform);
            copiedbuffer.mesh = mesh_opt.cloned();
            copiedbuffer.material = mat_opt.cloned();
            copiedbuffer.studs_material = studs_mat_opt.cloned();
            copiedbuffer.name = Some(name.to_string());
            copiedbuffer.is_brick = brick_opt.is_some();
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
                commands.entity(new_entity).insert(Brick);
            }

            let data = crate::studio::tools::BrickData {
                transform: newtransform,
                name: format!("{} - Copy", name),
                is_brick: copiedbuffer.is_brick,
                mesh: copiedbuffer.mesh.clone(),
                standard_material: copiedbuffer.material.clone(),
                studs_material: copiedbuffer.studs_material.clone(),
                parent: None,
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
        if let Ok((_, transform, name, child_of_opt, _, brick_opt, _, mesh_opt, mat_opt, studs_mat_opt)) = entities_query.get(entity) {
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
            if brick_opt.is_some() {
                commands.entity(new_entity).insert(Brick);
            }

            let parent_entity = child_of_opt.map(|co| co.parent());
            if let Some(p) = parent_entity {
                commands.entity(p).add_child(new_entity);
            }

            let data = crate::studio::tools::BrickData {
                transform: newtransform,
                name: format!("{} - Copy", name.as_str()),
                is_brick: brick_opt.is_some(),
                mesh: mesh_opt.cloned(),
                standard_material: mat_opt.cloned(),
                studs_material: studs_mat_opt.cloned(),
                parent: parent_entity,
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
        if let Some(data) = crate::studio::tools::capture_brick_data(entity, entities_query) {
            history.push_command(crate::studio::tools::UndoCommand::Delete {
                entity,
                data,
            });
        }
        commands.entity(entity).despawn();
        if selection.entity == Some(entity) {
            selection.entity = None;
        }
        ui.close();
        closed = true;
    }
    closed
}