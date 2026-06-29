pub mod camera;
pub mod gizmos;
pub mod tools;
pub mod ui;

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;
use bevy::camera_controller::free_camera::FreeCameraPlugin;

pub struct StudioPlugin;

impl Plugin for StudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<tools::ToolState>()
            .init_resource::<tools::Selection>()
            .init_resource::<tools::DragState>()
            .init_resource::<tools::HoverState>()
            .init_resource::<camera::BrickSpawnerCount>()
            .init_resource::<ui::StudioUiTextureIds>()
            .init_resource::<ui::CameraSpeedIndicator>()
            .init_resource::<ui::CopiedEntityBuffer>()
            .add_plugins(MeshPickingPlugin)
            .add_plugins(FreeCameraPlugin)
            .add_systems(Startup, (camera::setup_studio, ui::setup_ui_assets, ui::configure_visuals))
            .add_systems(
                Update,
                (
                    gizmos::update_gizmos,
                    gizmos::sync_gizmos,
                    gizmos::draw_selection_outline,
                    tools::select_brick,
                    tools::handle_drag_start,
                    tools::handle_drag,
                    tools::handle_drag_end,
                    tools::handle_hover,
                    tools::update_cursor,
                    ui::updatecameraspeedindicator,
                ),
            )
            .add_systems(EguiPrimaryContextPass, ui::studio_ui);
    }
}