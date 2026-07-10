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
        if !app.is_plugin_added::<bevy_egui::EguiPlugin>() {
            app.add_plugins(bevy_egui::EguiPlugin::default());
        }
        if !app.is_plugin_added::<bevy::diagnostic::FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default());
        }
        if !app.is_plugin_added::<bevy::render::occlusion_culling::OcclusionCullingPlugin>() {
            app.add_plugins(bevy::render::occlusion_culling::OcclusionCullingPlugin);
        }
        app.init_state::<tools::ToolState>()
            .init_state::<tools::OnboardingState>()
            .init_resource::<tools::Selection>()
            .init_resource::<tools::DragState>()
            .init_resource::<tools::PartDragState>()
            .init_resource::<tools::HoverState>()
            .init_resource::<tools::CanvasContextMenu>()
            .init_resource::<tools::MarqueeState>()
            .init_resource::<ui::StudioUiTextureIds>()
            .init_resource::<ui::CameraSpeedIndicator>()
            .init_resource::<ui::FovIndicator>()
            .init_resource::<ui::CopiedEntityBuffer>()
            .init_resource::<ui::HierarchyDraggedEntity>()
            .init_resource::<ui::SettingsWindow>()
            .init_resource::<tools::SnapConfig>()
            .init_resource::<tools::UndoRedoHistory>()
            .init_resource::<tools::PlayersService>()
            .init_resource::<ui::panels::onboarding::OnboardingData>()
            .add_message::<tools::UndoRedoAction>()
            .add_plugins(MeshPickingPlugin)
            .add_plugins(FreeCameraPlugin)
            .add_systems(Startup, (
                camera::setup_studio.after(crate::common::game::bricks::studs::setup_studs),
                ui::setup_ui_assets,
                ui::configure_visuals,
            ))
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
                    tools::handle_part_drag_start,
                    tools::handle_part_drag,
                    tools::handle_part_drag_end,
                    tools::handle_hover,
                    tools::update_cursor,
                    tools::handle_keyboard_shortcuts,
                    tools::handle_undo_redo_action,
                    tools::handle_marquee_selection,
                    ui::updatecameraspeedindicator,
                    ui::update_camera_fov
                        .before(bevy::camera_controller::free_camera::run_freecamera_controller),
                    camera::disable_camera_on_ui_interaction
                        .before(bevy::camera_controller::free_camera::run_freecamera_controller),
                    camera::sync_gizmo_camera,
                ),
            )
            .add_systems(
                PostUpdate,
                tools::correct_child_transforms.after(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(EguiPrimaryContextPass, ui::studio_ui);
    }
}