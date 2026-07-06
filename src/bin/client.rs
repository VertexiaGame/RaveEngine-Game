use bevy::prelude::*;
use bevy::log::LogPlugin;
use lightyear::prelude::*;
use lightyear::prelude::client::*;
use RaveEngineLib::client::ClientPlugin;
use RaveEngineLib::common::CommonPlugin;

#[derive(Resource)]
struct ClientConnectSettings {
    ip: std::net::IpAddr,
    port: u16,
}

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
        std::env::set_var("VERTIGO_APP", "client");
        std::env::set_var("RUST_LOG", new_rust_log);
    }

    let mut ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
    let mut port = 5000;

    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(p) = args[i + 1].parse::<u16>() {
                port = p;
            }
        }
        if args[i] == "--ip" && i + 1 < args.len() {
            if let Ok(ip_addr) = args[i + 1].parse::<std::net::IpAddr>() {
                ip = ip_addr;
            }
        }
    }

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(LogPlugin {
        level: bevy::log::Level::DEBUG,
        filter: "wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=debug,lightyear_udp=trace,lightyear_netcode=trace,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off".to_string(),
        custom_layer: RaveEngineLib::common::vuis::logging::vuis_custom_layer,
        ..default()
    }));
    app.insert_resource(ClientConnectSettings { ip, port });
    app.add_plugins(client::ClientPlugins {
        tick_duration: core::time::Duration::from_secs_f64(1.0 / 60.0),
    });
    app.add_plugins(CommonPlugin);
    app.add_plugins(ClientPlugin);
    app.add_systems(Startup, setup_client.after(RaveEngineLib::client::setup_player_assets));
    app.add_systems(Update, trigger_delayed_connect);
    app.run();
}

fn setup_client(mut commands: Commands, settings: Res<ClientConnectSettings>) {
    let server_addr = std::net::SocketAddr::new(settings.ip, settings.port);
    let client_id = rand::random::<u64>();

    commands.insert_resource(RaveEngineLib::client::LocalClientId(client_id));

    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: [0u8; 32],
        protocol_id: 0,
    };

    let netcode_config = NetcodeConfig {
        client_timeout_secs: 15,
        ..default()
    };

    commands.spawn((
        Client::default(),
        UdpIo::default(),
        NetcodeClient::new(auth, netcode_config).unwrap(),
        LocalAddr(std::net::SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            0,
        )),
        PeerAddr(server_addr),
    ));
}

fn trigger_delayed_connect(
    mut commands: Commands,
    mut frame_count: Local<u32>,
    client_query: Query<Entity, With<Client>>,
    mut connected: Local<bool>,
) {
    if *connected {
        return;
    }
    *frame_count += 1;
    if *frame_count >= 30 {
        for entity in &client_query {
            commands.trigger(Connect { entity });
            info!("Handshake connection triggered after rendering warmup");
        }
        *connected = true;
    }
}