use bevy::prelude::*;
use bevy_egui::egui;
use crate::studio::tools::Selection;
use crate::common::game::bricks::components::Brick;
use crate::studio::ui::CopiedEntityBuffer;
use crate::studio::ui::HierarchyDraggedEntity;
use crate::studio::ui::panels::context_menu::draw_entity_context_menu;
use crate::studio::ui::resources::ActiveScriptEditor;
use bevy::pbr::ExtendedMaterial;

fn is_managed_entity(
    entity: Entity,
    query: &Query<(
        Entity,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&crate::scripting::ecs::ServerScript>,
        Option<&crate::scripting::ecs::LocalScript>,
        Option<&crate::scripting::ecs::ModuleScript>,
    ), Without<Camera3d>>,
) -> bool {
    if let Ok((_, name, _, _, brick_opt, s_opt, l_opt, m_opt)) = query.get(entity) {
        name.as_str() == "Baseplate" || brick_opt.is_some() || s_opt.is_some() || l_opt.is_some() || m_opt.is_some()
    } else {
        false
    }
}

fn is_descendant(
    child: Entity,
    parent: Entity,
    query: &Query<(
        Entity,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&crate::scripting::ecs::ServerScript>,
        Option<&crate::scripting::ecs::LocalScript>,
        Option<&crate::scripting::ecs::ModuleScript>,
    ), Without<Camera3d>>,
) -> bool {
    let mut current = child;
    let mut depth = 0;
    while let Ok((_, _, parent_opt, _, _, _, _, _)) = query.get(current) {
        if let Some(parent_comp) = parent_opt {
            let parent_entity = parent_comp.parent();
            if parent_entity == parent {
                return true;
            }
            current = parent_entity;
        } else {
            break;
        }
        depth += 1;
        if depth > 1000 {
            break;
        }
    }
    false
}

fn get_flat_ordered_entities(
    explorer_query: &Query<(
        Entity,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&crate::scripting::ecs::ServerScript>,
        Option<&crate::scripting::ecs::LocalScript>,
        Option<&crate::scripting::ecs::ModuleScript>,
    ), Without<Camera3d>>,
) -> Vec<Entity> {
    let mut flat = Vec::new();
    let mut roots = Vec::new();
    for (entity, name, parent_opt, _, brick_opt, s_opt, l_opt, m_opt) in explorer_query {
        let is_managed = name.as_str() == "Baseplate" || brick_opt.is_some() || s_opt.is_some() || l_opt.is_some() || m_opt.is_some();
        if is_managed {
            let is_root = if let Some(parent_comp) = parent_opt {
                let parent = parent_comp.parent();
                if let Ok((_, p_name, _, _, p_brick_opt, ps_opt, pl_opt, pm_opt)) = explorer_query.get(parent) {
                    !(p_name.as_str() == "Baseplate" || p_brick_opt.is_some() || ps_opt.is_some() || pl_opt.is_some() || pm_opt.is_some())
                } else {
                    true
                }
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

    for (root_entity, _) in roots {
        traverse_node_recursive(root_entity, explorer_query, &mut flat);
    }
    flat
}

fn traverse_node_recursive(
    entity: Entity,
    explorer_query: &Query<(
        Entity,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&crate::scripting::ecs::ServerScript>,
        Option<&crate::scripting::ecs::LocalScript>,
        Option<&crate::scripting::ecs::ModuleScript>,
    ), Without<Camera3d>>,
    flat: &mut Vec<Entity>,
) {
    flat.push(entity);
    if let Ok((_, _, _, Some(children_comp), _, _, _, _)) = explorer_query.get(entity) {
        let mut sorted_children: Vec<Entity> = children_comp
            .iter()
            .filter(|&child| is_managed_entity(child, explorer_query))
            .collect();
        sorted_children.sort_by(|&a, &b| {
            let name_a = explorer_query.get(a).map(|(_, n, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
            let name_b = explorer_query.get(b).map(|(_, n, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
            name_a.cmp(name_b)
        });
        for child in sorted_children {
            traverse_node_recursive(child, explorer_query, flat);
        }
    }
}

fn perform_range_selection(
    entity: Entity,
    pool: &[Entity],
    selection: &mut ResMut<Selection>,
) {
    if pool.is_empty() {
        return;
    }
    let Some(target_idx) = pool.iter().position(|&e| e == entity) else {
        return;
    };
    let start_idx = if let Some(active) = selection.entity {
        pool.iter().position(|&e| e == active).unwrap_or(target_idx)
    } else {
        let mut found = None;
        for &selected in selection.entities.iter().rev() {
            if let Some(idx) = pool.iter().position(|&e| e == selected) {
                found = Some(idx);
                break;
            }
        }
        found.unwrap_or(target_idx)
    };

    let min_idx = start_idx.min(target_idx);
    let max_idx = start_idx.max(target_idx);

    selection.workspace_selected = false;
    selection.players_selected = false;
    selection.lighting_selected = false;
    selection.entities = pool[min_idx..=max_idx].to_vec();
    selection.entity = Some(entity);
}

fn draw_entity_node(
    ui: &mut egui::Ui,
    entity: Entity,
    commands: &mut Commands,
    selection: &mut ResMut<Selection>,
    explorer_query: &Query<(
        Entity,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&crate::scripting::ecs::ServerScript>,
        Option<&crate::scripting::ecs::LocalScript>,
        Option<&crate::scripting::ecs::ModuleScript>,
    ), Without<Camera3d>>,
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
    copiedbuffer: &mut CopiedEntityBuffer,
    dragged_entity: &mut ResMut<HierarchyDraggedEntity>,
    history: &mut ResMut<crate::studio::tools::UndoRedoHistory>,
    active_editor: &mut ResMut<ActiveScriptEditor>,
    studs_query: &Query<&crate::common::game::bricks::components::BrickStuds>,
    workspace_tex: egui::TextureId,
    brick_tex: egui::TextureId,
    script_tex: egui::TextureId,
    localscript_tex: egui::TextureId,
    modulescript_tex: egui::TextureId,
) {
    let Ok((_, name, _, children_opt, _, s_opt, l_opt, m_opt)) = explorer_query.get(entity) else { return };
    let name_str = name.as_str().to_string();

    let has_managed_children = if let Some(children_comp) = children_opt {
        children_comp.iter().any(|child| is_managed_entity(child, explorer_query))
    } else {
        false
    };

    let is_selected = selection.entities.contains(&entity);

    let icon_tex = if s_opt.is_some() {
        Some(script_tex)
    } else if l_opt.is_some() {
        Some(localscript_tex)
    } else if m_opt.is_some() {
        Some(modulescript_tex)
    } else {
        Some(brick_tex)
    };

    let is_script_disabled = if let Some(ref s) = s_opt {
        !s.enabled
    } else if let Some(ref l) = l_opt {
        !l.enabled
    } else {
        false
    };

    if has_managed_children {
        let id = egui::Id::new(entity);
        let mut collapsing_state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
        if ui.data_mut(|d| d.remove_temp::<bool>(id.with("should_toggle"))).unwrap_or(false) {
            let open = collapsing_state.is_open();
            collapsing_state.set_open(!open);
            collapsing_state.store(ui.ctx());
        }

        let header_res = collapsing_state.show_header(ui, |ui| {
            ui.push_id(id, |ui| {
                let label_res = explorerlabel(ui, is_selected, &name_str, icon_tex, is_script_disabled);

                if label_res.clicked() {
                    let ctrl_held = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                    let shift_held = ui.input(|i| i.modifiers.shift);
                    if ctrl_held {
                        selection.workspace_selected = false;
                        selection.players_selected = false;
                        selection.lighting_selected = false;
                        if selection.entities.contains(&entity) {
                            selection.entities.retain(|&e| e != entity);
                            if selection.entity == Some(entity) {
                                selection.entity = selection.entities.last().copied();
                            }
                        } else {
                            selection.entities.push(entity);
                            selection.entity = Some(entity);
                        }
                    } else if shift_held {
                        let pool = get_flat_ordered_entities(explorer_query);
                        perform_range_selection(entity, &pool, selection);
                    } else {
                        selection.entity = Some(entity);
                        selection.entities = vec![entity];
                        selection.workspace_selected = false;
                        selection.players_selected = false;
                        selection.lighting_selected = false;
                    }
                }

                if label_res.double_clicked() {
                    let mut is_script = false;
                    if let Ok((_, _, _, _, _, s, l, m)) = explorer_query.get(entity) {
                        if s.is_some() || l.is_some() || m.is_some() {
                            is_script = true;
                        }
                    }
                    if is_script {
                        if !active_editor.open_entities.contains(&entity) {
                            active_editor.open_entities.push(entity);
                        }
                        active_editor.entity = Some(entity);
                    } else {
                        ui.data_mut(|d| d.insert_temp(id.with("should_toggle"), true));
                    }
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
                        studs_query,
                    );
                });

                if label_res.drag_started() {
                    dragged_entity.entity = Some(entity);
                }

                if let Some(dragged) = dragged_entity.entity {
                    if dragged != entity && !is_descendant(entity, dragged, explorer_query) {
                        if label_res.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                        }
                        if ui.input(|i| i.pointer.any_released()) && label_res.hovered() {
                            if let (Ok((_, _, _, _, _, _, _, parent_global, _, _, _, _)), Ok((_, _, _, _, _, _, _, child_global, _, _, _, _))) = (
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

                                let old_parent = entities_query.get(dragged).ok().and_then(|(_, _, _, child_of_opt, _, _, _, _, _, _, _, _)| child_of_opt.map(|co| co.parent()));
                                let old_transform = entities_query.get(dragged).ok().map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t).unwrap_or(Transform::IDENTITY);

                                let new_transform = Transform {
                                    translation: local_translation,
                                    rotation: local_rotation,
                                    scale: local_scale,
                                };

                                if let Ok(mut d_cmd) = commands.get_entity(dragged) {
                                    d_cmd.insert(new_transform);
                                    d_cmd.remove::<ChildOf>();
                                }

                                history.push_command(crate::studio::tools::UndoCommand::ParentChange {
                                    entity: dragged,
                                    old_parent,
                                    new_parent: Some(entity),
                                    old_transform,
                                    new_transform,
                                });
                            }
                            if commands.get_entity(dragged).is_ok() {
                                if let Ok(mut p_cmd) = commands.get_entity(entity) {
                                    p_cmd.add_child(dragged);
                                }
                            }
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
                    .filter(|&child| is_managed_entity(child, explorer_query))
                    .collect();
                sorted_children.sort_by(|&a, &b| {
                    let name_a = explorer_query.get(a).map(|(_, n, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
                    let name_b = explorer_query.get(b).map(|(_, n, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
                    name_a.cmp(name_b)
                });

                for child in sorted_children {
                    draw_entity_node(
                        ui,
                        child,
                        commands,
                        selection,
                        explorer_query,
                        entities_query,
                        copiedbuffer,
                        dragged_entity,
                        history,
                        active_editor,
                        studs_query,
                        workspace_tex,
                        brick_tex,
                        script_tex,
                        localscript_tex,
                        modulescript_tex,
                    );
                }
            }
        });
    } else {
        let id = egui::Id::new(entity);
        let label_res = ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.push_id(id, |ui| {
                explorerlabel(ui, is_selected, &name_str, icon_tex, is_script_disabled)
            }).inner
        }).inner;

        if label_res.clicked() {
            let ctrl_held = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
            let shift_held = ui.input(|i| i.modifiers.shift);
            if ctrl_held {
                selection.workspace_selected = false;
                selection.players_selected = false;
                selection.lighting_selected = false;
                if selection.entities.contains(&entity) {
                    selection.entities.retain(|&e| e != entity);
                    if selection.entity == Some(entity) {
                        selection.entity = selection.entities.last().copied();
                    }
                } else {
                    selection.entities.push(entity);
                    selection.entity = Some(entity);
                }
            } else if shift_held {
                let pool = get_flat_ordered_entities(explorer_query);
                perform_range_selection(entity, &pool, selection);
            } else {
                selection.entity = Some(entity);
                selection.entities = vec![entity];
                selection.workspace_selected = false;
                selection.players_selected = false;
                selection.lighting_selected = false;
            }
        }

        if label_res.double_clicked() {
            let mut is_script = false;
            if let Ok((_, _, _, _, _, s, l, m)) = explorer_query.get(entity) {
                if s.is_some() || l.is_some() || m.is_some() {
                    is_script = true;
                }
            }
            if is_script {
                if !active_editor.open_entities.contains(&entity) {
                    active_editor.open_entities.push(entity);
                }
                active_editor.entity = Some(entity);
            }
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
                studs_query,
            );
        });

        if label_res.drag_started() {
            dragged_entity.entity = Some(entity);
        }

        if let Some(dragged) = dragged_entity.entity {
            if dragged != entity && !is_descendant(entity, dragged, explorer_query) {
                if label_res.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                if ui.input(|i| i.pointer.any_released()) && label_res.hovered() {
                    if let (Ok((_, _, _, _, _, _, _, parent_global, _, _, _, _)), Ok((_, _, _, _, _, _, _, child_global, _, _, _, _))) = (
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

                        let old_parent = entities_query.get(dragged).ok().and_then(|(_, _, _, child_of_opt, _, _, _, _, _, _, _, _)| child_of_opt.map(|co| co.parent()));
                        let old_transform = entities_query.get(dragged).ok().map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t).unwrap_or(Transform::IDENTITY);

                        let new_transform = Transform {
                            translation: local_translation,
                            rotation: local_rotation,
                            scale: local_scale,
                        };

                        if let Ok(mut d_cmd) = commands.get_entity(dragged) {
                            d_cmd.insert(new_transform);
                        }

                        history.push_command(crate::studio::tools::UndoCommand::ParentChange {
                            entity: dragged,
                            old_parent,
                            new_parent: Some(entity),
                            old_transform,
                            new_transform,
                        });
                    }
                    if commands.get_entity(dragged).is_ok() {
                        if let Ok(mut p_cmd) = commands.get_entity(entity) {
                            p_cmd.add_child(dragged);
                        }
                    }
                    dragged_entity.entity = None;
                }
            }
        }
    }
}

fn draw_player_node(
    ui: &mut egui::Ui,
    entity: Entity,
    selection: &mut ResMut<Selection>,
    explorer_query: &Query<(
        Entity,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&crate::scripting::ecs::ServerScript>,
        Option<&crate::scripting::ecs::LocalScript>,
        Option<&crate::scripting::ecs::ModuleScript>,
    ), Without<Camera3d>>,
    players_tex: egui::TextureId,
) {
    let Ok((_, name, _, _, _, _, _, _)) = explorer_query.get(entity) else { return };
    let name_str = name.as_str().to_string();
    let is_selected = selection.entities.contains(&entity);

    let id = egui::Id::new(entity);
    let label_res = ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.push_id(id, |ui| {
            explorerlabel(ui, is_selected, &name_str, Some(players_tex), false)
        }).inner
    }).inner;

    if label_res.clicked() {
        let ctrl_held = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
        let shift_held = ui.input(|i| i.modifiers.shift);
        if ctrl_held {
            selection.workspace_selected = false;
            selection.players_selected = false;
            selection.lighting_selected = false;
            if selection.entities.contains(&entity) {
                selection.entities.retain(|&e| e != entity);
                if selection.entity == Some(entity) {
                    selection.entity = selection.entities.last().copied();
                }
            } else {
                selection.entities.push(entity);
                selection.entity = Some(entity);
            }
        } else if shift_held {
            let mut sorted_players = Vec::new();
            for (player_entity, name, _, _, _, _, _, _) in explorer_query {
                let name_str = name.as_str();
                if name_str == "Player" || name_str.starts_with("Player_") {
                    sorted_players.push(player_entity);
                }
            }
            sorted_players.sort_by(|&a, &b| {
                let name_a = explorer_query.get(a).map(|(_, n, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
                let name_b = explorer_query.get(b).map(|(_, n, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
                name_a.cmp(name_b)
            });
            perform_range_selection(entity, &sorted_players, selection);
        } else {
            selection.entity = Some(entity);
            selection.entities = vec![entity];
            selection.workspace_selected = false;
            selection.players_selected = true;
            selection.lighting_selected = false;
        }
    }
}

pub fn draw_explorer(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    selection: &mut ResMut<Selection>,
    explorer_query: &Query<(
        Entity,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&crate::scripting::ecs::ServerScript>,
        Option<&crate::scripting::ecs::LocalScript>,
        Option<&crate::scripting::ecs::ModuleScript>,
    ), Without<Camera3d>>,
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
    copiedbuffer: &mut CopiedEntityBuffer,
    dragged_entity: &mut ResMut<HierarchyDraggedEntity>,
    history: &mut ResMut<crate::studio::tools::UndoRedoHistory>,
    active_editor: &mut ResMut<ActiveScriptEditor>,
    workspace_tex: egui::TextureId,
    brick_tex: egui::TextureId,
    players_tex: egui::TextureId,
    lighting_tex: egui::TextureId,
    script_tex: egui::TextureId,
    localscript_tex: egui::TextureId,
    modulescript_tex: egui::TextureId,
    studs_query: &Query<&crate::common::game::bricks::components::BrickStuds>,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Explorer").color(egui::Color32::from_rgb(0, 0, 0)).strong().size(16.0));
    });

    ui.add_space(8.0);
    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(212, 212, 212));
    ui.add_space(8.0);

    let mut roots = Vec::new();
    for (entity, name, parent_opt, _, brick_opt, s_opt, l_opt, m_opt) in explorer_query {
        let is_managed = name.as_str() == "Baseplate" || brick_opt.is_some() || s_opt.is_some() || l_opt.is_some() || m_opt.is_some();
        if is_managed {
            let is_root = if let Some(parent_comp) = parent_opt {
                let parent = parent_comp.parent();
                if let Ok((_, p_name, _, _, p_brick_opt, ps_opt, pl_opt, pm_opt)) = explorer_query.get(parent) {
                    !(p_name.as_str() == "Baseplate" || p_brick_opt.is_some() || ps_opt.is_some() || pl_opt.is_some() || pm_opt.is_some())
                } else {
                    true
                }
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

    let workspace_id = ui.make_persistent_id("workspace_collapsing_header");
    let mut workspace_state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), workspace_id, true);
    if ui.data_mut(|d| d.remove_temp::<bool>(workspace_id.with("should_toggle"))).unwrap_or(false) {
        let open = workspace_state.is_open();
        workspace_state.set_open(!open);
        workspace_state.store(ui.ctx());
    }

    let workspace_res = workspace_state.show_header(ui, |ui| {
        let label_res = explorerlabel(ui, selection.workspace_selected, "Workspace", Some(workspace_tex), false);
        if label_res.clicked() {
            selection.entity = None;
            selection.entities.clear();
            selection.workspace_selected = true;
            selection.players_selected = false;
            selection.lighting_selected = false;
        }
        if label_res.double_clicked() {
            ui.data_mut(|d| d.insert_temp(workspace_id.with("should_toggle"), true));
        }
    });

    let body_res = workspace_res.body(|ui| {
        for (entity, _) in roots {
            draw_entity_node(
                ui,
                entity,
                commands,
                selection,
                explorer_query,
                entities_query,
                copiedbuffer,
                dragged_entity,
                history,
                active_editor,
                studs_query,
                workspace_tex,
                brick_tex,
                script_tex,
                localscript_tex,
                modulescript_tex,
            );
        }
    });

    let header_res = body_res.0;

    if let Some(dragged) = dragged_entity.entity {
        if header_res.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if ui.input(|i| i.pointer.any_released()) && header_res.hovered() {
            if let Ok((_, _, _, child_of_opt, _, _, _, child_global, _, _, _, _)) = entities_query.get(dragged) {
                let old_parent = child_of_opt.map(|co| co.parent());
                let old_transform = entities_query.get(dragged).ok().map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t).unwrap_or(Transform::IDENTITY);

                let new_transform = Transform {
                    translation: child_global.translation(),
                    rotation: child_global.rotation(),
                    scale: child_global.scale(),
                };

                if let Ok(mut d_cmd) = commands.get_entity(dragged) {
                    d_cmd.insert(new_transform);
                    d_cmd.remove::<ChildOf>();
                }

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

    let players_id = ui.make_persistent_id("players_collapsing_header");
    let mut players_state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), players_id, true);
    if ui.data_mut(|d| d.remove_temp::<bool>(players_id.with("should_toggle"))).unwrap_or(false) {
        let open = players_state.is_open();
        players_state.set_open(!open);
        players_state.store(ui.ctx());
    }

    let players_res = players_state.show_header(ui, |ui| {
        let label_res = explorerlabel(ui, selection.players_selected, "Players", Some(players_tex), false);
        if label_res.clicked() {
            selection.entity = None;
            selection.entities.clear();
            selection.workspace_selected = false;
            selection.players_selected = true;
            selection.lighting_selected = false;
        }
        if label_res.double_clicked() {
            ui.data_mut(|d| d.insert_temp(players_id.with("should_toggle"), true));
        }
    });

    players_res.body(|ui| {
        let mut sorted_players = Vec::new();
        for (entity, name, _, _, _, _, _, _) in explorer_query {
            let name_str = name.as_str();
            if name_str == "Player" || name_str.starts_with("Player_") {
                sorted_players.push(entity);
            }
        }

        sorted_players.sort_by(|&a, &b| {
            let name_a = explorer_query.get(a).map(|(_, n, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
            let name_b = explorer_query.get(b).map(|(_, n, _, _, _, _, _, _)| n.as_str()).unwrap_or("");
            name_a.cmp(name_b)
        });

        for child in sorted_players {
            draw_player_node(
                ui,
                child,
                selection,
                explorer_query,
                players_tex,
            );
        }
    });

    let lighting_id = ui.make_persistent_id("lighting_collapsing_header");
    let mut lighting_state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), lighting_id, true);
    if ui.data_mut(|d| d.remove_temp::<bool>(lighting_id.with("should_toggle"))).unwrap_or(false) {
        let open = lighting_state.is_open();
        lighting_state.set_open(!open);
        lighting_state.store(ui.ctx());
    }

    let lighting_res = lighting_state.show_header(ui, |ui| {
        let label_res = explorerlabel(ui, selection.lighting_selected, "Lighting", Some(lighting_tex), false);
        if label_res.clicked() {
            selection.entity = None;
            selection.entities.clear();
            selection.workspace_selected = false;
            selection.players_selected = false;
            selection.lighting_selected = true;
        }
        if label_res.double_clicked() {
            ui.data_mut(|d| d.insert_temp(lighting_id.with("should_toggle"), true));
        }
    });

    lighting_res.body(|_| {});

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
    icon: Option<egui::TextureId>,
    disabled: bool,
) -> egui::Response {
    let size = egui::vec2(ui.available_width(), 20.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

    if !ui.is_rect_visible(rect) {
        return response;
    }

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

    let text_color = if disabled {
        if selected {
            egui::Color32::from_rgb(100, 100, 100)
        } else {
            egui::Color32::from_rgb(150, 150, 150)
        }
    } else if selected {
        egui::Color32::from_rgb(0, 0, 0)
    } else if response.hovered() {
        egui::Color32::from_rgb(20, 20, 20)
    } else {
        egui::Color32::from_rgb(60, 60, 60)
    };

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if let Some(tex_id) = icon {
                let mut img = egui::Image::new((tex_id, egui::vec2(16.0, 16.0)));
                if disabled {
                    img = img.tint(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 128));
                }
                ui.add(img);
                ui.add_space(4.0);
            }
            let mut text_element = egui::RichText::new(label).color(text_color).size(13.5);
            if disabled {
                text_element = text_element.strikethrough();
            }
            ui.add(egui::Label::new(text_element).selectable(false));
        });
    });

    response
}