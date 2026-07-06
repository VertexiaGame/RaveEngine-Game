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
    ambient: Option<ResMut<GlobalAmbientLight>>,
) {
    egui_global_settings.auto_create_primary_context = false;

    if let Some(mut amb) = ambient {
        amb.color = Color::srgb(0.85, 0.88, 1.0);
        amb.brightness = 1000.0;
    }

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.96, 0.85),
            illuminance: 12000.0,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            shadow_depth_bias: 0.1,
            shadow_normal_bias: 1.2,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.32, 0.95, 0.0)),
    ));

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
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.70, 0.90, 1.00)),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            far: 3000.0,
            fov: 80.0f32.to_radians(),
            ..default()
        }),
        Hdr,
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
        Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
        MeshPickingCamera,
        FreeCamera::default(),
        DepthPrepass,
        NormalPrepass,
        bevy::render::occlusion_culling::OcclusionCulling,
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

    let ssao_val = if graphics_settings.ssao { Some(ScreenSpaceAmbientOcclusion::default()) } else { None };
    let contact_val = if graphics_settings.contact_shadows { Some(ContactShadows::default()) } else { None };
    let bloom_val = if graphics_settings.bloom { Some(Bloom::default()) } else { None };

    if let Some(ssao) = ssao_val.clone() {
        camera.insert(ssao);
    }
    if let Some(contact) = contact_val.clone() {
        camera.insert(contact);
    }
    if let Some(bloom) = bloom_val.clone() {
        camera.insert(bloom);
    }

    let mut gizmo_camera = commands.spawn((
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Hdr,
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
        bevy::camera::visibility::RenderLayers::layer(1),
        bevy_egui::PrimaryEguiContext,
        GizmoCamera,
        DepthPrepass,
        NormalPrepass,
        MotionVectorPrepass,
        DistanceFog {
            color: Color::srgb(0.70, 0.90, 1.00),
            falloff: FogFalloff::Linear {
                start: 400.0,
                end: 1100.0,
            },
            ..default()
        },
        bevy::render::occlusion_culling::OcclusionCulling,
    ));

    if let Some(ssao) = ssao_val {
        gizmo_camera.insert(ssao);
    }
    if let Some(contact) = contact_val {
        gizmo_camera.insert(contact);
    }
    if let Some(bloom) = bloom_val {
        gizmo_camera.insert(bloom);
    }
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