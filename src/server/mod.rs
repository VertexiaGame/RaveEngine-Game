use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use std::time::Duration;
use avian3d::prelude::*;

pub mod player;
pub mod map;

#[derive(Resource)]
pub struct ServerSettings {
    pub map_path: String,
    pub port: u16,
}

pub struct ServerPlugin {
    pub map_path: String,
    pub port: u16,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerSettings {
            map_path: self.map_path.clone(),
            port: self.port,
        })
        .insert_resource(Gravity(Vec3::new(0.0, -186.9 * 0.28, 0.0)))
        .add_plugins(server::ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
        })
        .add_plugins(crate::common::net::ProtocolPlugin)
        .add_systems(Startup, (setup_server, map::load_map))
        .add_systems(Update, (
            player::handle_player_inputs,
            player::handle_hello_messages,
            player::sync_players_service_properties,
        ))
        .add_systems(PostUpdate, player::sync_transforms_to_network)
        .add_observer(player::handle_new_client)
        .add_observer(player::handle_client_disconnect);
    }
}

fn setup_server(
    mut commands: Commands,
    settings: Res<ServerSettings>,
) {
    info!("Starting setup_server system on port: {}", settings.port);
    let bind_addr = std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)), settings.port);

    let netcode_config = NetcodeConfig {
        protocol_id: 0,
        private_key: [0u8; 32],
        client_timeout_secs: 15,
        ..Default::default()
    };

    let server_entity = commands.spawn((
        Server::default(),
        ServerUdpIo::default(),
        LocalAddr(bind_addr),
        NetcodeServer::new(netcode_config),
    )).id();

    commands.trigger(Start { entity: server_entity });
    info!("Server entity spawned and Start trigger dispatched");

    commands.spawn((
        Name::new("Players"),
        crate::common::net::components::PlayersServiceContainer,
        Replicate::default(),
    ));
}