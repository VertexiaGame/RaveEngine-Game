use bevy::prelude::*;
use bevy::pbr::DefaultOpaqueRendererMethod;
use bevy::light::{DirectionalLightShadowMap, PointLightShadowMap};
use bevy_egui::EguiPlugin;
use RaveEngineLib::common;
use RaveEngineLib::studio;

fn main() {
    App::new()
        .insert_resource(DefaultOpaqueRendererMethod::forward())
        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .insert_resource(PointLightShadowMap { size: 1024 })
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                unapproved_path_mode: bevy::asset::UnapprovedPathMode::Allow,
                ..default()
            })
        )
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(common::CommonPlugin)
        .add_plugins(studio::StudioPlugin)
        .run();
}