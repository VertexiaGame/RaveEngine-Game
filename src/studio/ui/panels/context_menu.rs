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
            let mut spawned = commands.spawn((
                newtransform,
                Name::new(format!("{} - Copy", name)),
                Pickable::default(),
            ));
            if let Some(ref mesh) = copiedbuffer.mesh {
                spawned.insert(mesh.clone());
            }
            if let Some(ref mat) = copiedbuffer.material {
                spawned.insert(mat.clone());
            }
            if let Some(ref studs_mat) = copiedbuffer.studs_material {
                spawned.insert(studs_mat.clone());
            }
            if copiedbuffer.is_brick {
                spawned.insert(Brick);
            }
            ui.close();
            closed = true;
        }
    }
    if ui.button("Duplicate").clicked() {
        if let Ok((_, transform, name, _, _, brick_opt, _, mesh_opt, mat_opt, studs_mat_opt)) = entities_query.get(entity) {
            let newtransform = *transform;
            let mut spawned = commands.spawn((
                newtransform,
                Name::new(format!("{} - Copy", name.as_str())),
                Pickable::default(),
            ));
            if let Some(mesh) = mesh_opt {
                spawned.insert(mesh.clone());
            }
            if let Some(mat) = mat_opt {
                spawned.insert(mat.clone());
            }
            if let Some(studs_mat) = studs_mat_opt {
                spawned.insert(studs_mat.clone());
            }
            if brick_opt.is_some() {
                spawned.insert(Brick);
            }
            ui.close();
            closed = true;
        }
    }
    if ui.button("Delete").clicked() {
        commands.entity(entity).despawn();
        if selection.entity == Some(entity) {
            selection.entity = None;
        }
        ui.close();
        closed = true;
    }
    closed
}