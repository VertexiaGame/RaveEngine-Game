use bevy::prelude::*;
use bevy::log::LogPlugin;
use RaveEngineLib::common::CommonPlugin;
use RaveEngineLib::studio::StudioPlugin;

fn main() {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_default();
    let new_rust_log = if rust_log.is_empty() {
        "info,wgpu=warn,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off".to_string()
    } else if !rust_log.contains("offset_allocator") {
        format!("{rust_log},offset_allocator=off")
    } else {
        rust_log
    };
    unsafe {
        std::env::set_var("VERTIGO_APP", "studio");
        std::env::set_var("RUST_LOG", new_rust_log);
    }
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "info,wgpu=warn,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off".to_string(),
            custom_layer: RaveEngineLib::common::vuis::logging::vuis_custom_layer,
            ..default()
        }))
        .add_plugins(CommonPlugin)
        .add_plugins(StudioPlugin)
        .run();
}