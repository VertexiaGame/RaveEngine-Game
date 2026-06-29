pub mod assets;
pub mod indicator;
pub mod panels;
pub mod visuals;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle};
use crate::studio::tools::{ToolState, Selection};
use crate::common::components::Brick;

pub use assets::{StudioUiAssets, StudioUiTextureIds, setup_ui_assets};
pub use indicator::{CameraSpeedIndicator, updatecameraspeedindicator};
pub use visuals::configure_visuals;

#[derive(Resource, Default)]
pub struct CopiedEntityBuffer {
    pub transform: Option<Transform>,
    pub mesh: Option<Mesh3d>,
    pub material: Option<MeshMaterial3d<StandardMaterial>>,
    pub name: Option<String>,
    pub is_brick: bool,
}

#[allow(deprecated)]
pub fn studio_ui(
    mut contexts: EguiContexts,
    mut next_tool: ResMut<NextState<ToolState>>,
    current_tool: Res<State<ToolState>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut count: ResMut<crate::studio::camera::BrickSpawnerCount>,
    ui_assets: Option<Res<StudioUiAssets>>,
    mut texture_ids: ResMut<StudioUiTextureIds>,
    mut cameraindicator: ResMut<CameraSpeedIndicator>,
    mut cameraquery: Query<(
        &bevy::camera_controller::free_camera::FreeCamera,
        &mut bevy::camera_controller::free_camera::FreeCameraState,
    )>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut selection: ResMut<Selection>,
    entitiesquery: Query<(Entity, &Name, Option<&Brick>)>,
    mut copiedbuffer: ResMut<CopiedEntityBuffer>,
    fullentityquery: Query<(
        &Transform,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Name,
        Option<&Brick>,
    )>,
) {
    let Some(assets) = ui_assets else { return; };

    let move_tex = *texture_ids.move_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.move_icon.clone()))
    });
    let rotate_tex = *texture_ids.rotate_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.rotate_icon.clone()))
    });
    let scale_tex = *texture_ids.scale_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.scale_icon.clone()))
    });
    let add_tex = *texture_ids.add_tex.get_or_insert_with(|| {
        contexts.add_image(EguiTextureHandle::Strong(assets.add_icon.clone()))
    });

    let Ok(ctx) = contexts.ctx_mut() else { return; };
    ctx.set_visuals(egui::Visuals::light());

    let frame = egui::Frame::NONE
        .fill(egui::Color32::from_rgb(245, 246, 247))
        .inner_margin(egui::Margin::same(0));

    egui::Panel::top("topbar")
        .frame(frame)
        .show(ctx, |ui| {
            panels::draw_top_bar(
                ui,
                &mut next_tool,
                &current_tool,
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut count,
                move_tex,
                rotate_tex,
                scale_tex,
                add_tex,
                &diagnostics,
            );
        });

    egui::SidePanel::left("explorer")
        .frame(egui::Frame::none()
            .fill(egui::Color32::from_rgb(245, 246, 247))
            .inner_margin(egui::Margin::symmetric(12, 12))
        )
        .default_width(220.0)
        .show(ctx, |ui| {
            panels::draw_explorer(
                ui,
                &mut commands,
                &mut selection,
                &entitiesquery,
                &mut copiedbuffer,
                &fullentityquery,
            );
        });

    indicator::draw_indicator(ctx, &mut cameraindicator, &mut cameraquery);
}