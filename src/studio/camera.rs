use bevy::prelude::*;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::anti_alias::fxaa::Fxaa;
use bevy::camera_controller::free_camera::FreeCamera;
use bevy::camera::Hdr;
use bevy::post_process::bloom::Bloom;
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ContactShadows, ExtendedMaterial};
use crate::studio::bricks::spawn_brick;

#[derive(Component)]
pub struct GizmoCamera;

pub fn setup_studio(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, crate::studio::studs::StudsExtension>>>,
    studs_assets: Res<crate::studio::studs::StudsAssets>,
    mut count: ResMut<crate::studio::bricks::BrickSpawnerCount>,
    mut egui_global_settings: ResMut<bevy_egui::EguiGlobalSettings>,
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
    
    commands.spawn((
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
    )).insert((
        MotionVectorPrepass,
        Fxaa::default(),
        Bloom::default(),
        ScreenSpaceAmbientOcclusion::default(),
        ContactShadows::default(),
        DistanceFog {
            color: Color::srgb(0.70, 0.90, 1.00),
            falloff: FogFalloff::Linear {
                start: 400.0,
                end: 1100.0,
            },
            ..default()
        },
    ));

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

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))),
        MeshMaterial3d(studs_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.28, 0.62, 0.32),
                perceptual_roughness: 0.95,
                reflectance: 0.08,
                metallic: 0.0,
                ..default()
            },
            extension: crate::studio::studs::StudsExtension {
                stud_texture: studs_assets.stud.clone(),
                inlet_texture: studs_assets.inlet.clone(),
            },
        })),
        Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0)),
        crate::common::components::Brick,
        Pickable::default(),
        Name::new("Baseplate"),
    ));

    spawn_brick(&mut commands, &mut meshes, &mut studs_materials, &studs_assets, &mut count, Vec3::new(0.0, 0.14, 0.0));
}

pub fn disable_camera_on_ui_interaction(
    mut camera_query: Query<&mut bevy::camera_controller::free_camera::FreeCameraState>,
    mut contexts: bevy_egui::EguiContexts,
    mut picking_settings: ResMut<bevy::picking::PickingSettings>,
    hover_state: Res<crate::studio::tools::HoverState>,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        let wants_input = ctx.egui_wants_pointer_input() || ctx.egui_wants_keyboard_input() || hover_state.is_hovering_ui;
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