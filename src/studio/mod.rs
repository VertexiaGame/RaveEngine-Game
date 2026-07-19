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
            .init_resource::<ui::resources::ActiveScriptEditor>()
            .init_resource::<ui::resources::PlayInClientProcesses>()
            .init_resource::<ui::resources::PlaytestBackup>()
            .init_resource::<ui::resources::FileDialogState>()
            .init_resource::<tools::SnapConfig>()
            .init_resource::<tools::UndoRedoHistory>()
            .init_resource::<tools::PlayersService>()
            .init_resource::<crate::common::game::environment::lighting::LightingService>()
            .init_resource::<ui::panels::onboarding::OnboardingData>()
            .add_message::<tools::UndoRedoAction>()
            .insert_resource(bevy::picking::mesh_picking::MeshPickingSettings {
                require_markers: false,
                ..default()
            })
            .add_plugins(MeshPickingPlugin)
            .add_plugins(FreeCameraPlugin)
            .add_systems(Startup, (
                crate::studio::camera::setup_studio.after(crate::common::game::bricks::studs::setup_studs),
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
                ).run_if(in_state(tools::OnboardingState::Inactive)),
            )
            .add_systems(
                Update,
                (
                    ui::updatecameraspeedindicator,
                    ui::update_camera_fov
                        .before(bevy::camera_controller::free_camera::run_freecamera_controller),
                    crate::studio::camera::disable_camera_on_ui_interaction
                        .before(bevy::camera_controller::free_camera::run_freecamera_controller),
                ),
            )
            .add_systems(
                Update,
                (
                    crate::studio::camera::sync_gizmo_camera,
                    crate::studio::camera::toggle_editor_camera_active,
                    crate::studio::camera::disable_cameras_on_minimization,
                    ui::resources::handle_file_dialog_results,
                ),
            )
            .add_systems(Update, ui::resources::cleanup_play_processes_on_exit)
            .add_systems(
                PostUpdate,
                tools::correct_child_transforms.after(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(EguiPrimaryContextPass, ui::studio_ui);
    }
}

#[cfg(feature = "bench")]
fn spawn_studio_benchmark(mut commands: Commands) {
    let target = commands.spawn((
        Name::new("BenchBrick"),
        Transform::default(),
        GlobalTransform::default(),
        crate::common::game::bricks::components::Brick,
    )).id();
    commands.insert_resource(tools::Selection {
        entity: Some(target),
        entities: vec![target],
        ..default()
    });
}

#[cfg(feature = "bench")]
fn update_studio_benchmark(
    state: Res<State<tools::ToolState>>,
    mut next_state: ResMut<NextState<tools::ToolState>>,
    mut frame: Local<usize>,
    mut line_cache: Local<Option<(usize, String)>>,
) {
    *frame += 1;
    let next = match state.get() {
        tools::ToolState::Move => tools::ToolState::Size,
        tools::ToolState::Size => tools::ToolState::Rotate,
        _ => tools::ToolState::Move,
    };
    next_state.set(next);
    ui::line_numbers(&mut line_cache, 100 + (*frame % 2));
}

#[cfg(feature = "bench")]
fn record_studio_assets(
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    mut stats: ResMut<crate::common::core::bench::BenchStats>,
) {
    stats.set_asset_counts(meshes.len(), materials.len());
}

#[cfg(feature = "bench")]
pub fn add_studio_benchmark(app: &mut App) {
    app.init_state::<tools::ToolState>()
        .init_resource::<tools::Selection>()
        .init_asset::<StandardMaterial>()
        .add_systems(Startup, spawn_studio_benchmark)
        .add_systems(Update, (update_studio_benchmark, gizmos::update_gizmos).chain())
        .add_systems(Last, record_studio_assets.before(crate::common::core::bench::bench_finish_frame));
}
