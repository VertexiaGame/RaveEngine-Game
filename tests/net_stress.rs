use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use lightyear::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::server::*;
use RaveEngineLib::common::net::messages::{GameChannel, HelloMessage};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

//a lil explanation
//this will stress test with a bunch of clients connecting to the server
//if it reaches the PING_THRESHOLD_MS, it will stop and report the number of clients connected at that time (the "maximum", meaning it reached a high ping)


const PING_THRESHOLD_MS: f32 = 100.0;
const MAX_CLIENTS: usize = 100;
const WAVE_SIZE: usize = 5;
const WAVE_INTERVAL: usize = 60;
const MAX_STEPS: usize = 20_000;

#[derive(Resource, Default)]
struct PingSnapshot {
    worst_rtt_ms: f32,
    connected_clients: usize,
    samples_ready: bool,
}

fn sample_pings(
    clients: Query<&PingManager, With<ClientOf>>,
    mut snapshot: ResMut<PingSnapshot>,
    mut counter: Local<u32>,
) {
    *counter += 1;
    if *counter % 30 != 0 {
        return;
    }
    snapshot.connected_clients = clients.iter().len();
    let mut worst = 0.0f32;
    let mut ready = false;
    for ping in &clients {
        if ping.latency_samples_recv() > 0 {
            ready = true;
        }
        worst = worst.max(ping.rtt().as_secs_f32() * 1000.0);
    }
    snapshot.worst_rtt_ms = worst;
    snapshot.samples_ready = ready;
}

fn free_port() -> u16 {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("failed to bind probe socket");
    let port = socket.local_addr().unwrap().port();
    drop(socket);
    port
}

fn build_server_app(port: u16) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))));
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<bevy::render::mesh::Mesh>();
    app.add_plugins(StatesPlugin);
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaveEngineLib::common::CommonPlugin);
    app.add_plugins(RaveEngineLib::server::ServerPlugin {
        map_path: "assets/maps/does_not_exist.vrtx".to_string(),
        port,
        bind_addr: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        netcode_key: [0u8; 32],
        protocol_id: 0,
        allow_unauthenticated: true,
    });
    app.init_resource::<PingSnapshot>();
    app.add_systems(Update, sample_pings);
    app.finish();
    app
}

#[derive(Component)]
struct StressHelloSent;

fn trigger_connect(
    mut commands: Commands,
    mut frames: Local<u32>,
    client_query: Query<Entity, With<Client>>,
    mut connected: Local<bool>,
) {
    if *connected {
        return;
    }
    *frames += 1;
    if *frames >= 3 {
        for entity in &client_query {
            commands.trigger(Connect { entity });
        }
        *connected = true;
    }
}

#[derive(Resource)]
struct StressUkey(String);

fn send_hello(
    mut commands: Commands,
    mut client_query: Query<
        (Entity, &mut MessageSender<HelloMessage>),
        (With<Connected>, Without<StressHelloSent>),
    >,
    ukey: Option<Res<StressUkey>>,
) {
    let Some(ukey) = ukey else {
        return;
    };
    for (entity, mut sender) in &mut client_query {
        let hello = HelloMessage {
            ukey: ukey.0.clone(),
        };
        let _ = sender.send::<GameChannel>(hello);
        commands.entity(entity).insert(StressHelloSent);
    }
}

fn build_client_app(port: u16, client_id: u64) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))));
    app.add_plugins(StatesPlugin);
    app.add_plugins(TransformPlugin);
    app.add_plugins(ClientPlugins {
        tick_duration: Duration::from_secs_f64(1.0 / 60.0),
    });
    app.add_plugins(RaveEngineLib::common::net::ProtocolPlugin);
    app.insert_resource(StressUkey(format!("offline_stress_{client_id}")));

    let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: [0u8; 32],
        protocol_id: 0,
    };
    let netcode_config = client::NetcodeConfig {
        client_timeout_secs: 10,
        ..default()
    };
    app.world_mut().spawn((
        Client::default(),
        UdpIo::default(),
        NetcodeClient::new(auth, netcode_config).unwrap(),
        LocalAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)),
        PeerAddr(server_addr),
        Transport::new(PriorityConfig::default()).with_compression(CompressionConfig::LZ4),
    ));
    app.add_systems(Update, (trigger_connect, send_hello));
    app.finish();
    app
}

#[test]
fn server_ping_degrades_under_client_load() {
    let port = free_port();
    let mut server = build_server_app(port);
    let mut clients: Vec<App> = Vec::new();
    let mut worst_ever_ms = 0.0f32;
    let mut exceeded_at = None;

    for step in 0..MAX_STEPS {
        if step % WAVE_INTERVAL == 0 && clients.len() < MAX_CLIENTS {
            let wave = WAVE_SIZE.min(MAX_CLIENTS - clients.len());
            for _ in 0..wave {
                let client_id = 1000 + clients.len() as u64;
                let mut app = build_client_app(port, client_id);
                for _ in 0..3 {
                    app.update();
                }
                clients.push(app);
            }
        }

        server.update();
        for client in clients.iter_mut() {
            client.update();
        }

        if step % 30 == 0 {
            let snapshot = server.world().resource::<PingSnapshot>();
            worst_ever_ms = worst_ever_ms.max(snapshot.worst_rtt_ms);
            if snapshot.samples_ready && snapshot.worst_rtt_ms > PING_THRESHOLD_MS {
                exceeded_at = Some((clients.len(), snapshot.worst_rtt_ms, snapshot.connected_clients));
                break;
            }
        }
    }

    let snapshot = server.world().resource::<PingSnapshot>();
    println!(
        "STRESS RESULT: launched {} clients, {} connected, worst ping ever {:.1}ms, final ping {:.1}ms{}",
        clients.len(),
        snapshot.connected_clients,
        worst_ever_ms,
        snapshot.worst_rtt_ms,
        match exceeded_at {
            Some((n, ms, c)) => format!(
                " -> ping exceeded {}ms at {} clients ({} connected, {:.1}ms)",
                PING_THRESHOLD_MS, n, c, ms
            ),
            None => format!(
                " -> ping never exceeded {}ms within cap of {} clients",
                PING_THRESHOLD_MS, MAX_CLIENTS
            ),
        }
    );

    assert!(
        snapshot.connected_clients >= 2,
        "expected at least 2 clients to connect, got {}",
        snapshot.connected_clients
    );
}
