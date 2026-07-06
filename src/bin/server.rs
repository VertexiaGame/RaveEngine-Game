use bevy::prelude::*;
use bevy::log::LogPlugin;
use bevy::state::app::StatesPlugin;
use RaveEngineLib::server::ServerPlugin;
use RaveEngineLib::common::CommonPlugin;

fn main() {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_default();
    let new_rust_log = if rust_log.is_empty() {
        "debug,wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=debug,lightyear_udp=trace,lightyear_netcode=trace,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off".to_string()
    } else if !rust_log.contains("offset_allocator") {
        format!("{rust_log},offset_allocator=off")
    } else {
        rust_log
    };
    unsafe {
        std::env::set_var("VERTIGO_APP", "server");
        std::env::set_var("RUST_LOG", new_rust_log);
    }

    let mut port = 5000;
    let mut map_path = "assets/maps/default.vrtx".to_string();

    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(p) = args[i + 1].parse::<u16>() {
                port = p;
            }
        }
        if args[i] == "--map" && i + 1 < args.len() {
            map_path = args[i + 1].clone();
        }
    }

    let mut app = App::new();
    app.add_plugins(LogPlugin {
        level: bevy::log::Level::DEBUG,
        filter: "wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=debug,lightyear_udp=trace,lightyear_netcode=trace,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off".to_string(),
        custom_layer: RaveEngineLib::common::vuis::logging::vuis_custom_layer,
        ..default()
    });
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Mesh>();
    app.add_plugins(StatesPlugin);
    app.add_plugins(TransformPlugin);
    app.add_plugins(CommonPlugin);
    app.add_plugins(ServerPlugin {
        map_path,
        port,
    });
    app.run();
}