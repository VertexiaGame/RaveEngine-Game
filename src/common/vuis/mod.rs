pub mod types;
pub mod api;
pub mod logging;

use bevy::prelude::*;
use bevy::ui::{BackgroundGradient, LinearGradient, ColorStop, InterpolationColorSpace};
use types::{VuisNode, VuisAnimationState, PlaceholderTextComponent, VuisRootContainer};

pub struct VuisPlugin;

impl Plugin for VuisPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<api::VuisEngine>()
            .add_systems(Update, (
                scale_vuis_root_system,
                grid_layout_update_system,
                sync_vuis_node_changes,
                placeholder_update_system,
                text_styling_update_system,
                animation_system,
            ));
    }
}

pub fn scale_vuis_root_system(
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut query_root: Query<(&mut UiTransform, &mut Node, &VuisRootContainer)>,
) {
    let Ok(window) = window_query.single() else { return; };
    for (mut transform, mut node, root_container) in query_root.iter_mut() {
        let scale_x = window.width() / root_container.design_width;
        let scale_y = window.height() / root_container.design_height;
        let scale = scale_x.min(scale_y).max(0.1);

        node.width = Val::Px(root_container.design_width);
        node.height = Val::Px(root_container.design_height);
        node.position_type = PositionType::Absolute;
        node.left = Val::Percent(50.0);
        node.top = Val::Percent(50.0);
        node.margin = UiRect {
            left: Val::Px(-root_container.design_width / 2.0),
            top: Val::Px(-root_container.design_height / 2.0),
            ..default()
        };
        transform.scale = Vec2::new(scale, scale);
    }
}

pub fn grid_layout_update_system(
    query_nodes: Query<(Entity, &VuisNode, Option<&Children>)>,
    mut query_node_styles: Query<(Option<&VuisNode>, &mut Node)>,
) {
    for (_, parent_vnode, children_opt) in query_nodes.iter() {
        let is_flow = parent_vnode.LayoutFlow != "None";
        if let Some(children) = children_opt {
            for child_ent in children.iter() {
                if let Ok((child_vnode_opt, mut child_node)) = query_node_styles.get_mut(child_ent) {
                    if is_flow {
                        if child_node.position_type != PositionType::Relative {
                            child_node.position_type = PositionType::Relative;
                            child_node.left = Val::Auto;
                            child_node.top = Val::Auto;
                        }
                    } else {
                        if child_node.position_type != PositionType::Absolute {
                            child_node.position_type = PositionType::Absolute;
                            if let Some(child_vnode) = child_vnode_opt {
                                child_node.left = Val::Px(child_vnode.PositionX);
                                child_node.top = Val::Px(child_vnode.PositionY);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn sync_vuis_node_changes(
    mut commands: Commands,
    query_changed: Query<(Entity, &VuisNode), Changed<VuisNode>>,
    mut query_bevy_ui: Query<(&mut Node, &mut BackgroundColor, &mut UiTransform)>,
    query_children: Query<&Children>,
    query_text: Query<&Text>,
) {
    for (entity, node) in query_changed.iter() {
        info!(
            "VUIS: Node component changed for element (ID: {}), syncing Bevy UI... Pos: ({}, {}), Size: ({}x{})",
            node.Id, node.PositionX, node.PositionY, node.WidthPx, node.HeightPx
        );
        if let Ok((mut bevy_node, mut bg_color, mut transform)) = query_bevy_ui.get_mut(entity) {
            bevy_node.left = Val::Px(node.PositionX);
            bevy_node.top = Val::Px(node.PositionY);
            bevy_node.width = if node.WidthPx <= 0.0 { Val::Auto } else { Val::Px(node.WidthPx) };
            bevy_node.height = if node.HeightPx <= 0.0 { Val::Auto } else { Val::Px(node.HeightPx) };
            bevy_node.border_radius = BorderRadius::all(Val::Px(node.BorderRadiusPx));
            bevy_node.overflow = if node.IsScrollable { Overflow::scroll_y() } else { Overflow::visible() };
            bevy_node.scrollbar_width = if node.IsScrollable { node.ScrollbarWidth } else { 0.0 };

            bevy_node.display = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { Display::Grid } else { Display::Flex };
            bevy_node.grid_template_columns = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { vec![RepeatedGridTrack::flex(node.GridColumns as u16, 1.0)] } else { Vec::new() };
            bevy_node.grid_template_rows = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { vec![RepeatedGridTrack::flex(node.GridRows as u16, 1.0)] } else { Vec::new() };
            bevy_node.column_gap = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { Val::Px(node.GridColumnGap) } else { Val::Auto };
            bevy_node.row_gap = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { Val::Px(node.GridRowGap) } else { Val::Auto };

            bg_color.0 = node.BackgroundColor;
            transform.rotation = Rot2::radians(-node.Rotation);
        }

        if node.IsHidden {
            commands.entity(entity).insert(Visibility::Hidden);
        } else {
            commands.entity(entity).insert(Visibility::Inherited);
        }

        if node.HasShadow {
            commands.entity(entity).insert(BoxShadow::new(
                node.ShadowColor,
                Val::Px(node.ShadowOffsetX),
                Val::Px(node.ShadowOffsetY),
                Val::Px(node.ShadowSpread),
                Val::Px(node.ShadowBlur),
            ));
        } else {
            commands.entity(entity).remove::<BoxShadow>();
        }

        if node.HasText {
            if let Ok(children) = query_children.get(entity) {
                for child in children.iter() {
                    if query_text.get(child).is_ok() {
                        if node.HasShadow {
                            commands.entity(child).insert(TextShadow {
                                offset: Vec2::new(node.ShadowOffsetX, node.ShadowOffsetY),
                                color: node.ShadowColor,
                            });
                        } else {
                            commands.entity(child).remove::<TextShadow>();
                        }
                    }
                }
            }
        }

        if node.IsGradient {
            commands.entity(entity).insert(BackgroundGradient::from(LinearGradient {
                color_space: InterpolationColorSpace::Oklaba,
                angle: 0.0,
                stops: vec![
                    ColorStop::percent(node.GradientColor1, 0.0),
                    ColorStop::percent(node.GradientColor2, 100.0),
                ],
            }));
        } else {
            commands.entity(entity).remove::<BackgroundGradient>();
        }

        if node.BorderWidthPx > 0.0 {
            commands.entity(entity).insert((
                Node {
                    border: UiRect::all(Val::Px(node.BorderWidthPx)),
                    ..default()
                },
                BorderColor::all(node.BorderColor),
            ));
        } else {
            commands.entity(entity).remove::<BorderColor>();
        }
    }
}

pub fn placeholder_update_system(
    mut commands: Commands,
    query_nodes: Query<(Entity, &VuisNode, Option<&Children>)>,
    query_main_text: Query<&Text, Without<PlaceholderTextComponent>>,
    query_placeholder: Query<&PlaceholderTextComponent>,
    mut query_placeholder_mut: Query<(&mut Text, &mut Visibility, &PlaceholderTextComponent)>,
) {
    for (node_entity, vnode, children_opt) in query_nodes.iter() {
        if !vnode.IsInput { continue; }
        
        let mut has_placeholder = false;
        
        if let Some(children) = children_opt {
            for child in children.iter() {
                if query_placeholder.get(child).is_ok() {
                    has_placeholder = true;
                }
            }
        }
        
        if !has_placeholder {
            let p_ent = commands.spawn((
                Text::new(vnode.Placeholder.clone()),
                TextFont { font_size: FontSize::Px(vnode.FontSizePx), ..default() },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                PlaceholderTextComponent(node_entity),
            )).id();
            commands.entity(node_entity).add_child(p_ent);
        }
    }

    for (mut p_text, mut p_vis, p_comp) in query_placeholder_mut.iter_mut() {
        if let Ok((_, vnode, children_opt)) = query_nodes.get(p_comp.0) {
            p_text.0 = vnode.Placeholder.clone();
            let mut has_main_text = false;
            if let Some(children) = children_opt {
                for child in children.iter() {
                    if query_main_text.get(child).is_ok() {
                        if let Ok(text) = query_main_text.get(child) {
                            if !text.0.is_empty() {
                                has_main_text = true;
                            }
                        }
                    }
                }
            }

            if has_main_text {
                *p_vis = Visibility::Hidden;
            } else {
                *p_vis = Visibility::Inherited;
            }
        }
    }
}

pub fn text_styling_update_system(
    query_nodes: Query<(&VuisNode, Option<&Children>)>,
    mut query_text_fonts: Query<(&Text, &mut TextFont)>,
) {
    for (node, children_opt) in query_nodes.iter() {
        if let Some(children) = children_opt {
            for child in children.iter() {
                if let Ok((_, mut text_font)) = query_text_fonts.get_mut(child) {
                    text_font.font_size = FontSize::Px(node.FontSizePx);
                    
                    text_font.weight = if node.IsBold {
                        bevy::text::FontWeight::BOLD
                    } else {
                        bevy::text::FontWeight::default()
                    };

                    text_font.style = if node.IsItalic {
                        bevy::text::FontStyle::Italic
                    } else {
                        bevy::text::FontStyle::default()
                    };
                }
            }
        }
    }
}

pub fn animation_system(
    time: Res<Time>,
    mut query_nodes: Query<(&VuisNode, &mut Node, &mut UiTransform, &mut VuisAnimationState)>,
) {
    for (node, mut ui_node, mut trans, mut state) in query_nodes.iter_mut() {
        if state.IsPlaying && node.AnimDuration > 0.0 {
            state.Timer += time.delta_secs();
            if state.Timer >= node.AnimDuration {
                state.Timer = 0.0;
                state.Forward = !state.Forward;
            }
            let progress = state.Timer / node.AnimDuration;
            let eased = if state.Forward { progress } else { 1.0 - progress };
            
            let current_width = node.WidthPx + (node.AnimTargetWidth - node.WidthPx) * eased;
            let current_height = node.HeightPx + (node.AnimTargetHeight - node.HeightPx) * eased;
            let current_x = node.PositionX + (node.AnimTargetX - node.PositionX) * eased;
            let current_y = node.PositionY + (node.AnimTargetY - node.PositionY) * eased;
            let current_rot = node.Rotation + (node.AnimTargetRotation - node.Rotation) * eased;

            ui_node.width = if current_width <= 0.0 { Val::Auto } else { Val::Px(current_width) };
            ui_node.height = if current_height <= 0.0 { Val::Auto } else { Val::Px(current_height) };
            ui_node.left = Val::Px(current_x);
            ui_node.top = Val::Px(current_y);
            trans.rotation = Rot2::radians(-current_rot);
        } else if !state.IsPlaying {
            ui_node.width = if node.WidthPx <= 0.0 { Val::Auto } else { Val::Px(node.WidthPx) };
            ui_node.height = if node.HeightPx <= 0.0 { Val::Auto } else { Val::Px(node.HeightPx) };
            ui_node.left = Val::Px(node.PositionX);
            ui_node.top = Val::Px(node.PositionY);
            trans.rotation = Rot2::radians(-node.Rotation);
        }
    }
}