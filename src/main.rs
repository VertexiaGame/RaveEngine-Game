mod common;
mod studio;

use bevy::prelude::*;
use bevy::pbr::DefaultOpaqueRendererMethod;
use bevy::light::DirectionalLightShadowMap;
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .insert_resource(DefaultOpaqueRendererMethod::forward())
        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(common::CommonPlugin)
        .add_plugins(studio::StudioPlugin)
        .run();
}