use bevy::prelude::*;
use bevy_egui::egui;
use crate::studio::tools::Selection;
use crate::common::components::Brick;
use crate::studio::ui::CopiedEntityBuffer;
use crate::studio::ui::HierarchyDraggedEntity;
use crate::studio::ui::panels::context_menu::draw_entity_context_menu;
use bevy::pbr::ExtendedMaterial;

fn is_managed_entity(
    entity: Entity,
    query: &Query<(
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
    if let Ok((_, _, name, _, _, brick_opt, _, _, _, _)) = query.get(entity) {
        name.as_str() == "Baseplate" || brick_opt.is_some()
    } else {
        false
    }
}

fn is_descendant(
    child: Entity,
    parent: Entity,
    query: &Query<(
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
    let mut current = child;
    let mut depth = 0;
    while let Ok((_, _, _, Some(parent_comp), _, _, _, _, _, _)) = query.get(current) {
        let parent_entity = parent_comp.parent();
        if parent_entity == parent {
            return true;
        }
        current = parent_entity;
        depth += 1;
        if depth > 1000 {
            break;
        }
    }
    false
}

fn draw_entity_node(
    ui: &mut egui::Ui,
    entity: Entity,
    commands: &mut Commands,
    selection: &mut ResMut<Selection>,
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
    copiedbuffer: &mut CopiedEntityBuffer,
    dragged_entity: &mut ResMut<HierarchyDraggedEntity>,
    history: &mut ResMut<crate::studio::tools::UndoRedoHistory>,
) {
    let Ok((_, _, name, _, children_opt, _, _, _, _, _)) = entities_query.get(entity) else { return };
    let name_str = name.as_str().to_string();

    let has_managed_children = if let Some(children_comp) = children_opt {
        children_comp.iter().any(|child| is_managed_entity(child, entities_query))
    } else {
        false
    };

    let is_selected = selection.entity == Some(entity);

    if has_managed_children {
        let id = egui::Id::new(entity);
        let collapsing_state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);

        let header_res = collapsing_state.show_header(ui, |ui| {
            ui.push_id(id, |ui| {
                let label_res = explorerlabel(ui, is_selected, &name_str);

                if label_res.clicked() {
                    selection.entity = Some(entity);
                }

                label_res.context_menu(|ui| {
                    draw_entity_context_menu(
                        ui,
                        entity,
                        commands,
                        selection,
                        copiedbuffer,
                        entities_query,
                        history,
                    );
                });

                if label_res.drag_started() {
                    dragged_entity.entity = Some(entity);
                }

                if let Some(dragged) = dragged_entity.entity {
                    if dragged != entity && !is_descendant(entity, dragged, entities_query) {
                        if label_res.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                        }
                        if ui.input(|i| i.pointer.any_released()) && label_res.hovered() {
                            if let (Ok((_, _, _, _, _, _, parent_global, _, _, _)), Ok((_, _, _, _, _, _, child_global, _, _, _))) = (
                                entities_query.get(entity),
                                entities_query.get(dragged)
                            ) {
                                let parent_rotation = parent_global.rotation();
                                let parent_translation = parent_global.translation();

                                let child_scale = child_global.scale();
                                let child_rotation = child_global.rotation();
                                let child_translation = child_global.translation();

                                let local_scale = child_scale;
                                let local_rotation = parent_rotation.inverse() * child_rotation;
                                let local_translation = parent_rotation.inverse().mul_vec3(child_translation - parent_translation);

                                let old_parent = entities_query.get(dragged).ok().and_then(|(_, _, _, child_of_opt, _, _, _, _, _, _)| child_of_opt.map(|co| co.parent()));
                                let old_transform = entities_query.get(dragged).ok().map(|(_, t, _, _, _, _, _, _, _, _)| *t).unwrap_or(Transform::IDENTITY);

                                let new_transform = Transform {
                                    translation: local_translation,
                                    rotation: local_rotation,
                                    scale: local_scale,
                                };

                                commands.entity(dragged).insert(new_transform);

                                history.push_command(crate::studio::tools::UndoCommand::ParentChange {
                                    entity: dragged,
                                    old_parent,
                                    new_parent: Some(entity),
                                    old_transform,
                                    new_transform,
                                });
                            }
                            commands.entity(entity).add_child(dragged);
                            dragged_entity.entity = None;
                        }
                    }
                }
            });
        });

        header_res.body(|ui| {
            if let Some(children_comp) = children_opt {
                let mut sorted_children: Vec<Entity> = children_comp
                    .iter()
                    .filter(|&child| is_managed_entity(child, entities_query))
                    .collect();
                sorted_children.sort_by(|&a, &b| {
                    let name_a = entities_query.get(a).map(|(_, _, n, _, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
                    let name_b = entities_query.get(b).map(|(_, _, n, _, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
                    name_a.cmp(name_b)
                });

                for child in sorted_children {
                    draw_entity_node(
                        ui,
                        child,
                        commands,
                        selection,
                        entities_query,
                        copiedbuffer,
                        dragged_entity,
                        history,
                    );
                }
            }
        });
    } else {
        let id = egui::Id::new(entity);
        let label_res = ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.push_id(id, |ui| {
                explorerlabel(ui, is_selected, &name_str)
            }).inner
        }).inner;

        if label_res.clicked() {
            selection.entity = Some(entity);
        }

        label_res.context_menu(|ui| {
            draw_entity_context_menu(
                ui,
                entity,
                commands,
                selection,
                copiedbuffer,
                entities_query,
                history,
            );
        });

        if label_res.drag_started() {
            dragged_entity.entity = Some(entity);
        }

        if let Some(dragged) = dragged_entity.entity {
            if dragged != entity && !is_descendant(entity, dragged, entities_query) {
                if label_res.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                if ui.input(|i| i.pointer.any_released()) && label_res.hovered() {
                    if let (Ok((_, _, _, _, _, _, parent_global, _, _, _)), Ok((_, _, _, _, _, _, child_global, _, _, _))) = (
                        entities_query.get(entity),
                        entities_query.get(dragged)
                    ) {
                        let parent_rotation = parent_global.rotation();
                        let parent_translation = parent_global.translation();

                        let child_scale = child_global.scale();
                        let child_rotation = child_global.rotation();
                        let child_translation = child_global.translation();

                        let local_scale = child_scale;
                        let local_rotation = parent_rotation.inverse() * child_rotation;
                        let local_translation = parent_rotation.inverse().mul_vec3(child_translation - parent_translation);

                        let old_parent = entities_query.get(dragged).ok().and_then(|(_, _, _, child_of_opt, _, _, _, _, _, _)| child_of_opt.map(|co| co.parent()));
                        let old_transform = entities_query.get(dragged).ok().map(|(_, t, _, _, _, _, _, _, _, _)| *t).unwrap_or(Transform::IDENTITY);

                        let new_transform = Transform {
                            translation: local_translation,
                            rotation: local_rotation,
                            scale: local_scale,
                        };

                        commands.entity(dragged).insert(new_transform);

                        history.push_command(crate::studio::tools::UndoCommand::ParentChange {
                            entity: dragged,
                            old_parent,
                            new_parent: Some(entity),
                            old_transform,
                            new_transform,
                        });
                    }
                    commands.entity(entity).add_child(dragged);
                    dragged_entity.entity = None;
                }
            }
        }
    }
}

pub fn draw_explorer(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    selection: &mut ResMut<Selection>,
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
    copiedbuffer: &mut CopiedEntityBuffer,
    dragged_entity: &mut ResMut<HierarchyDraggedEntity>,
    history: &mut ResMut<crate::studio::tools::UndoRedoHistory>,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Explorer").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(16.0));
    });

    ui.add_space(8.0);
    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
    ui.add_space(8.0);

    let mut roots = Vec::new();
    for (entity, _, name, parent_opt, _, _, _, _, _, _) in entities_query {
        if is_managed_entity(entity, entities_query) {
            let is_root = if let Some(parent_comp) = parent_opt {
                !is_managed_entity(parent_comp.parent(), entities_query)
            } else {
                true
            };
            if is_root {
                roots.push((entity, name.as_str().to_string()));
            }
        }
    }

    roots.sort_by(|a, b| {
        if a.1 == "Baseplate" {
            std::cmp::Ordering::Less
        } else if b.1 == "Baseplate" {
            std::cmp::Ordering::Greater
        } else {
            a.1.cmp(&b.1)
        }
    });

    let workspace_res = egui::CollapsingHeader::new(egui::RichText::new("Workspace").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(14.0))
        .default_open(true)
        .show(ui, |ui| {
            for (entity, _) in roots {
                draw_entity_node(
                    ui,
                    entity,
                    commands,
                    selection,
                    entities_query,
                    copiedbuffer,
                    dragged_entity,
                    history,
                );
            }
        });

    let header_res = workspace_res.header_response;
    if let Some(dragged) = dragged_entity.entity {
        if header_res.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if ui.input(|i| i.pointer.any_released()) && header_res.hovered() {
            if let Ok((_, _, _, child_of_opt, _, _, child_global, _, _, _)) = entities_query.get(dragged) {
                let old_parent = child_of_opt.map(|co| co.parent());
                let old_transform = entities_query.get(dragged).ok().map(|(_, t, _, _, _, _, _, _, _, _)| *t).unwrap_or(Transform::IDENTITY);

                let new_transform = Transform {
                    translation: child_global.translation(),
                    rotation: child_global.rotation(),
                    scale: child_global.scale(),
                };

                commands.entity(dragged).insert(new_transform);
                commands.entity(dragged).remove::<ChildOf>();

                history.push_command(crate::studio::tools::UndoCommand::ParentChange {
                    entity: dragged,
                    old_parent,
                    new_parent: None,
                    old_transform,
                    new_transform,
                });
            }
            dragged_entity.entity = None;
        }
    }

    if ui.input(|i| i.pointer.any_released()) {
        dragged_entity.entity = None;
    }

    let right_x = ui.max_rect().right() + 12.0;
    let top_y = ui.max_rect().top() - 12.0;
    let bottom_y = ui.max_rect().bottom() + 12.0;
    ui.painter().line_segment(
        [egui::pos2(right_x, top_y), egui::pos2(right_x, bottom_y)],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 180))
    );
}

fn explorerlabel(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
) -> egui::Response {
    let size = egui::vec2(ui.available_width(), 20.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

    if selected {
        ui.painter().rect_filled(
            rect,
            2.0,
            egui::Color32::from_rgb(204, 232, 255),
        );
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(153, 209, 255)),
            egui::StrokeKind::Inside,
        );
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            2.0,
            egui::Color32::from_rgb(224, 238, 249),
        );
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 220, 240)),
            egui::StrokeKind::Inside,
        );
    }

    let text_color = if selected {
        egui::Color32::from_rgb(0, 0, 0)
    } else if response.hovered() {
        egui::Color32::from_rgb(20, 20, 20)
    } else {
        egui::Color32::from_rgb(60, 60, 60)
    };

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.add(egui::Label::new(egui::RichText::new(label).color(text_color).size(13.5)).selectable(false));
        });
    });

    response
}