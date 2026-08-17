#[cfg(target_os = "android")] //Hey android, die!!
fn main() {}

#[cfg(not(target_os = "android"))]
mod desktop_studio {
    use bevy::prelude::*;
    use bevy::log::LogPlugin;
    use RaveEngineLib::common::CommonPlugin;
    use RaveEngineLib::studio::StudioPlugin;

    pub fn main() {
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
            .insert_resource(bevy_egui::EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            })
            .add_plugins(DefaultPlugins.set(LogPlugin {
                filter: "info,wgpu=warn,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off".to_string(),
                ..default()
            }).set(bevy::render::RenderPlugin {
                render_creation: bevy::render::settings::RenderCreation::Automatic(Box::new(
                    bevy::render::settings::WgpuSettings {
                        disabled_features: Some(bevy::render::settings::WgpuFeatures::TEXTURE_BINDING_ARRAY),
                        ..default()
                    }
                )),
                ..default()
            }))
            .add_plugins(lightyear::prelude::client::ClientPlugins {
                tick_duration: core::time::Duration::from_secs_f64(1.0 / 60.0),
            })
            .add_plugins(CommonPlugin)
            .add_plugins(RaveEngineLib::client::ClientPlugin)
            .add_plugins(StudioPlugin)
            .run();
    }
}

#[cfg(not(target_os = "android"))]
fn main() {
    desktop_studio::main();
}