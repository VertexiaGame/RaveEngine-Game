use bevy::prelude::*;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::anti_alias::fxaa::Fxaa;
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::camera::Hdr;
use bevy::post_process::bloom::Bloom;
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ContactShadows};

#[derive(Component)]
pub struct GizmoCamera;

pub fn setup_studio(
    mut commands: Commands,
    mut egui_global_settings: ResMut<bevy_egui::EguiGlobalSettings>,
    graphics_settings: Res<crate::studio::ui::GraphicsSettings>,
) {
    egui_global_settings.auto_create_primary_context = false;

    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    
    let mut camera = commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            far: 3000.0,
            fov: 80.0f32.to_radians(),
            ..default()
        }),
        Hdr,
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
        MeshPickingCamera,
        FreeCamera::default(),
        DepthPrepass,
        NormalPrepass,
    ));

    camera.insert((
        MotionVectorPrepass,
        Fxaa::default(),
        DistanceFog {
            color: Color::srgb(0.70, 0.90, 1.00),
            falloff: FogFalloff::Linear {
                start: 400.0,
                end: 1100.0,
            },
            ..default()
        },
    ));

    if graphics_settings.ssao {
        camera.insert(ScreenSpaceAmbientOcclusion::default());
    }
    if graphics_settings.contact_shadows {
        camera.insert(ContactShadows::default());
    }
    if graphics_settings.bloom {
        camera.insert(Bloom::default());
    }

    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Hdr,
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        bevy::camera::visibility::RenderLayers::layer(1),
        bevy_egui::PrimaryEguiContext,
        GizmoCamera,
    ));
}

pub fn disable_camera_on_ui_interaction(
    mut camera_query: Query<&mut bevy::camera_controller::free_camera::FreeCameraState>,
    mut contexts: bevy_egui::EguiContexts,
    mut picking_settings: ResMut<bevy::picking::PickingSettings>,
    hover_state: Res<crate::studio::tools::HoverState>,
    onboarding_state: Res<State<crate::studio::tools::OnboardingState>>,
) {
    let onboarding_active = *onboarding_state.get() != crate::studio::tools::OnboardingState::Inactive;

    if let Ok(ctx) = contexts.ctx_mut() {
        let wants_input = ctx.egui_wants_pointer_input() || ctx.egui_wants_keyboard_input() || hover_state.is_hovering_ui || onboarding_active;
        for mut state in &mut camera_query {
            state.enabled = !wants_input;
        }
        picking_settings.is_enabled = !wants_input;
    }
}

pub fn sync_gizmo_camera(
    camera_query: Query<(&Transform, &Projection), (With<Camera3d>, Without<GizmoCamera>)>,
    mut gizmo_camera: Query<(&mut Transform, &mut Projection), With<GizmoCamera>>,
) {
    if let Some((main_trans, main_proj)) = camera_query.iter().next() {
        if let Some((mut gizmo_trans, mut gizmo_proj)) = gizmo_camera.iter_mut().next() {
            *gizmo_trans = *main_trans;
            *gizmo_proj = main_proj.clone();
        }
    }
}