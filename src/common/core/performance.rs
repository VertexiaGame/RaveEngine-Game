use bevy::prelude::*;
use bevy::winit::{WinitSettings, UpdateMode};
use bevy::window::{PresentMode, PrimaryWindow, WindowMode};
use std::time::Duration;

#[derive(Component)]
pub struct PreviousTransform(pub Transform);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MsaaQuality {
    Off,
    Sample2,
    Sample4,
    Sample8,
}

impl MsaaQuality {
    pub fn samples(self) -> u32 {
        match self {
            MsaaQuality::Off => 1,
            MsaaQuality::Sample2 => 2,
            MsaaQuality::Sample4 => 4,
            MsaaQuality::Sample8 => 8,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShadowQuality {
    Off,
    Low,
    Medium,
    High,
}

impl ShadowQuality {
    pub fn shadow_map_size(self) -> usize {
        match self {
            ShadowQuality::Off | ShadowQuality::Low => 512,
            ShadowQuality::Medium => 1024,
            ShadowQuality::High => 2048,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VsyncMode {
    Off,
    On,
    Adaptive,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShadowCascades {
    Two,
    Three,
    Four,
}

impl ShadowCascades {
    pub fn count(self) -> usize {
        match self {
            ShadowCascades::Two => 2,
            ShadowCascades::Three => 3,
            ShadowCascades::Four => 4,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShadowFilter {
    Fast,
    HighQuality,
}

impl ShadowFilter {
    pub fn method(self) -> bevy::light::ShadowFilteringMethod {
        match self {
            ShadowFilter::Fast => bevy::light::ShadowFilteringMethod::Hardware2x2,
            ShadowFilter::HighQuality => bevy::light::ShadowFilteringMethod::Gaussian,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CloudQuality {
    Off,
    Low,
    Medium,
    High,
}

impl CloudQuality {
    pub fn preset(self) -> (bool, f32, u32, u32) {
        match self {
            CloudQuality::Off => (false, 0.5, 6, 3),
            CloudQuality::Low => (true, 0.5, 6, 3),
            CloudQuality::Medium => (true, 0.75, 9, 4),
            CloudQuality::High => (true, 1.0, 12, 6),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewDistance {
    Low,
    Medium,
    High,
}

impl ViewDistance {
    pub fn brick_lod_distances(self) -> (f32, f32, f32) {
        match self {
            ViewDistance::Low => (12.0, 48.0, 96.0),
            ViewDistance::Medium => (20.0, 64.0, 128.0),
            ViewDistance::High => (28.0, 80.0, 160.0),
        }
    }
}

#[derive(Resource)]
pub struct GraphicsSettings {
    pub ssao: bool,
    pub contact_shadows: bool,
    pub bloom: bool,
    pub msaa: MsaaQuality,
    pub shadow_quality: ShadowQuality,
    pub shadow_filter: ShadowFilter,
    pub vsync: VsyncMode,
    pub cloud_quality: CloudQuality,
    pub view_distance: ViewDistance,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            ssao: false,
            contact_shadows: false,
            bloom: true,
            msaa: MsaaQuality::Sample4,
            shadow_quality: ShadowQuality::Medium,
            shadow_filter: ShadowFilter::HighQuality,
            vsync: VsyncMode::On,
            cloud_quality: CloudQuality::High,
            view_distance: ViewDistance::High,
        }
    }
}

pub struct PerformancePlugin;

impl Plugin for PerformancePlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.register_required_components::<Camera3d, bevy::light::ShadowFilteringMethod>()
                .insert_resource(WinitSettings::desktop_app())
                .init_resource::<GraphicsSettings>()
                .add_systems(Update, manage_winit_performance)
                .add_systems(Last, apply_graphics_settings);
        }
    }
}

pub fn apply_graphics_settings(
    settings: Res<GraphicsSettings>,
    mut cameras: Query<&mut Msaa>,
    mut shadow_filter_methods: Query<&mut bevy::light::ShadowFilteringMethod, With<Camera3d>>,
    mut directional_light_shadow_map: ResMut<bevy::light::DirectionalLightShadowMap>,
    mut directional_lights: Query<&mut bevy::light::DirectionalLight>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if settings.is_changed() {
        let msaa = Msaa::from_samples(settings.msaa.samples());
        for mut camera_msaa in &mut cameras {
            *camera_msaa = msaa;
        }
        let shadow_filtering = settings.shadow_filter.method();
        for mut filtering in &mut shadow_filter_methods {
            *filtering = shadow_filtering;
        }
        directional_light_shadow_map.size = settings.shadow_quality.shadow_map_size();

        let present_mode = match settings.vsync {
            VsyncMode::Off => PresentMode::Immediate,
            VsyncMode::On => PresentMode::Fifo,
            VsyncMode::Adaptive => PresentMode::AutoVsync,
        };
        for mut window in &mut windows {
            window.present_mode = present_mode;
        }
    }

    if settings.shadow_quality == ShadowQuality::Off {
        for mut light in &mut directional_lights {
            light.shadow_maps_enabled = false;
        }
    }
}

pub fn manage_winit_performance(
    mut winit_settings: ResMut<WinitSettings>,
    drag_state: Option<Res<crate::studio::tools::DragState>>,
    part_drag_state: Option<Res<crate::studio::tools::PartDragState>>,
    physics_state: Option<Res<crate::common::game::physics::PhysicsSimulationState>>,
    camera_query: Query<(Entity, &Transform), With<Camera3d>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut prev_transforms: Query<&mut PreviousTransform>,
    mut commands: Commands,
    mut last_mouse_position: Local<Option<Vec2>>,
    mut last_mouse_movement_time: Local<f32>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
) {
    let current_time = time.elapsed_secs();
    
    let mut is_hovered = false;
    let mut is_fullscreen = false;
    
    if let Ok(window) = windows.single() {
        if !matches!(window.mode, WindowMode::Windowed) {
            is_fullscreen = true;
        }
        if let Some(cursor_pos) = window.cursor_position() {
            is_hovered = true;
            if let Some(last_pos) = *last_mouse_position {
                if cursor_pos.distance_squared(last_pos) > 0.0001 {
                    *last_mouse_position = Some(cursor_pos);
                    *last_mouse_movement_time = current_time;
                }
            } else {
                *last_mouse_position = Some(cursor_pos);
                *last_mouse_movement_time = current_time;
            }
        } else {
            *last_mouse_position = None;
        }
    }

    let time_since_last_move = current_time - *last_mouse_movement_time;
    let is_mouse_active = is_hovered && (time_since_last_move < 3.0);

    let buttons_pressed = mouse_buttons.is_some_and(|b| b.any_pressed([
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
        MouseButton::Back,
        MouseButton::Forward,
    ]));
    let keys_pressed = keys.is_some_and(|k| k.any_pressed([
        KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD,
        KeyCode::KeyQ, KeyCode::KeyE, KeyCode::ArrowUp, KeyCode::ArrowDown,
        KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::Space,
        KeyCode::ShiftLeft, KeyCode::ShiftRight,
        KeyCode::ControlLeft, KeyCode::ControlRight,
    ]));

    let mut is_active = is_fullscreen || is_mouse_active || buttons_pressed || keys_pressed;

    if let Some(ds) = drag_state {
        if ds.active {
            is_active = true;
        }
    }
    if let Some(pds) = part_drag_state {
        if pds.active {
            is_active = true;
        }
    }

    let mut physics_running = false;
    if let Some(ps) = physics_state {
        if *ps == crate::common::game::physics::PhysicsSimulationState::Running {
            physics_running = true;
            is_active = true;
        }
    }

    for (entity, transform) in &camera_query {
        if let Ok(mut prev) = prev_transforms.get_mut(entity) {
            let dist_sq = transform.translation.distance_squared(prev.0.translation);
            let rot_diff = transform.rotation.dot(prev.0.rotation).abs();
            if dist_sq > 0.00001 || rot_diff < 0.99999 {
                is_active = true;
            }
            prev.0 = *transform;
        } else {
            commands.entity(entity).insert(PreviousTransform(*transform));
            is_active = true;
        }
    }

    if is_active {
        winit_settings.focused_mode = UpdateMode::Continuous;
    } else {
        winit_settings.focused_mode = UpdateMode::reactive(Duration::from_secs(60));
    }

    if physics_running {
        winit_settings.unfocused_mode = UpdateMode::Continuous;
    } else {
        winit_settings.unfocused_mode = UpdateMode::reactive_low_power(Duration::from_secs(60));
    }
}