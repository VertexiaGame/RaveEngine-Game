pub mod camera;
pub mod gizmos;
pub mod tools;
pub mod ui;
pub mod studs;
pub mod bricks;

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;
use bevy::camera_controller::free_camera::FreeCameraPlugin;
use bevy::pbr::{ExtendedMaterial, MaterialPlugin};

pub struct StudioPlugin;

impl Plugin for StudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<tools::ToolState>()
            .init_resource::<tools::Selection>()
            .init_resource::<tools::DragState>()
            .init_resource::<tools::PartDragState>()
            .init_resource::<tools::HoverState>()
            .init_resource::<tools::CanvasContextMenu>()
            .init_resource::<bricks::BrickSpawnerCount>()
            .init_resource::<ui::StudioUiTextureIds>()
            .init_resource::<ui::CameraSpeedIndicator>()
            .init_resource::<ui::FovIndicator>()
            .init_resource::<ui::CopiedEntityBuffer>()
            .init_resource::<ui::HierarchyDraggedEntity>()
            .init_resource::<ui::SettingsWindow>()
            .init_resource::<tools::SnapConfig>()
            .init_resource::<tools::UndoRedoHistory>()
            .add_message::<tools::UndoRedoAction>()
            .add_plugins(MeshPickingPlugin)
            .add_plugins(FreeCameraPlugin)
            .add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>::default())
            .add_systems(Startup, (
                studs::setup_studs,
                camera::setup_studio.after(studs::setup_studs),
                ui::setup_ui_assets,
                ui::configure_visuals,
            ))
            .add_systems(
                Update,
                (
                    studs::configure_studs_samplers,
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