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
    pub bind_addr: std::net::IpAddr,
    pub netcode_key: [u8; 32],
    pub protocol_id: u64,
    pub allow_unauthenticated: bool,
}

#[derive(Resource, Default)]
pub struct ClientPlayerMap(pub std::collections::HashMap<u64, Entity>);

#[derive(Resource, Default)]
pub struct PendingAuths(pub std::collections::HashMap<u64, Entity>);

const AUTH_WORKERS: usize = 16;
const AUTH_QUEUE_CAP: usize = 64;

struct AuthWorkers {
    tx: crossbeam_channel::Sender<(u64, String)>,
    rx: std::sync::Mutex<crossbeam_channel::Receiver<(u64, Result<crate::common::net::auth::ValidateResponse, String>)>>,
}

impl AuthWorkers { //pool of threads that validate ukey -- website
    fn start(allow_unauthenticated: bool) -> Self {
        let (job_tx, job_rx) = crossbeam_channel::bounded::<(u64, String)>(AUTH_QUEUE_CAP);
        let (res_tx, res_rx) = crossbeam_channel::bounded::<(u64, Result<crate::common::net::auth::ValidateResponse, String>)>(AUTH_QUEUE_CAP);
        for _ in 0..AUTH_WORKERS {
            let job_rx = job_rx.clone();
            let res_tx = res_tx.clone();
            std::thread::spawn(move || {
                while let Ok((client_id, ukey)) = job_rx.recv() {
                    let result = crate::common::net::auth::validate_user_ukey(&ukey, allow_unauthenticated);
                    let _ = res_tx.send((client_id, result));
                }
            });
        }
        Self {
            tx: job_tx,
            rx: std::sync::Mutex::new(res_rx),
        }
    }
}

#[derive(Resource, Default)]

//NOW we cap AuthWorker to (by default) 64 pending auth requests (woah there thats a big polytoria typa connections)
//and 16 workers. hopefully enough to handle a few hundred clients connecting??? most servers are going to be 0 - 16 players, so this should be 
//wayy more than enough
pub struct AuthWorkerPool(Option<std::sync::Arc<AuthWorkers>>);

impl AuthWorkerPool {
    pub fn ensure_started(&mut self, allow_unauthenticated: bool) {
        if self.0.is_none() {
            self.0 = Some(std::sync::Arc::new(AuthWorkers::start(allow_unauthenticated)));
        }
    }

    pub fn submit(&self, client_id: u64, ukey: String) -> bool {
        match &self.0 {
            Some(workers) => workers.tx.try_send((client_id, ukey)).is_ok(),
            None => false,
        }
    }

    pub fn drain_results(
        &self,
    ) -> Vec<(u64, Result<crate::common::net::auth::ValidateResponse, String>)> {
        match &self.0 {
            Some(workers) => {
                let rx = workers.rx.lock().unwrap();
                rx.try_iter().collect()
            }
            None => Vec::new(),
        }
    }
}

pub struct ServerPlugin {
    pub map_path: String,
    pub port: u16,
    pub bind_addr: std::net::IpAddr,
    pub netcode_key: [u8; 32],
    pub protocol_id: u64,
    pub allow_unauthenticated: bool,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerSettings {
            map_path: self.map_path.clone(),
            port: self.port,
            bind_addr: self.bind_addr,
            netcode_key: self.netcode_key,
            protocol_id: self.protocol_id,
            allow_unauthenticated: self.allow_unauthenticated,
        })
        .insert_resource(Gravity(Vec3::new(0.0, -186.9 * 0.28, 0.0)))
        .insert_resource(ReplicationMetadata::new(
            Duration::from_secs_f64(1.0 / 30.0),
        ))
        .init_resource::<ClientPlayerMap>()
        .init_resource::<PendingAuths>()
        .init_resource::<AuthWorkerPool>()
        .add_plugins(server::ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
        })
        .add_plugins(crate::common::net::ProtocolPlugin)
        .add_systems(Startup, (setup_server, map::load_map))
        .add_systems(Update, (
            player::handle_player_moves,
            player::handle_hello_messages,
            player::handle_auth_results,
            player::sync_players_service_properties,
        ).chain())
        .add_systems(
            FixedPostUpdate,
            player::apply_player_movement
                .in_set(PhysicsSystems::Prepare)
                .after(avian3d::physics_transform::PhysicsTransformSystems::TransformToPosition),
        )
        .add_systems(PostUpdate, player::sync_transforms_to_network)
        .add_observer(player::handle_new_client)
        .add_observer(player::handle_client_disconnect);
    }
}

fn setup_server(
    mut commands: Commands,
    settings: Res<ServerSettings>,
) {
    info!("Starting setup_server system on {}:{}", settings.bind_addr, settings.port);
    let bind_addr = std::net::SocketAddr::new(settings.bind_addr, settings.port);

    if settings.netcode_key == [0u8; 32] {
        warn!("netcode_key false");
    }
    if settings.allow_unauthenticated {
        warn!("studio keys allowed -- playtets/stress test so um if this is unintended fuck!");
    }

    let netcode_config = NetcodeConfig {
        protocol_id: settings.protocol_id,
        private_key: settings.netcode_key,
        client_timeout_secs: 15,
        ..Default::default()
    };

    let server_entity = commands.spawn((
        Server::default(),
        ServerUdpIo::default(),
        LocalAddr(bind_addr),
        NetcodeServer::new(netcode_config),
        Transport::new(PriorityConfig::default())
            .with_compression(CompressionConfig::LZ4),
    )).id();

    commands.trigger(Start { entity: server_entity });
    info!("Server entity spawned and Start trigger dispatched");

    commands.insert_resource(crate::scripting::vm::server_vm::ServerScriptVM::new());


    //spawn the services to replicate to client
    commands.spawn((
        Name::new("Workspace"),
    ));

    commands.spawn((
        Name::new("Players"),
        crate::common::net::components::PlayersServiceContainer, //Players, contains defs for speed, jump power and some other player specific attributes
        Replicate::default(),
    ));

    commands.spawn((
        Name::new("Lighting"),
        crate::common::net::components::LightingServiceContainer, //lighting, contains time of day, ambient light etc
        Replicate::default(),
    ));

    commands.spawn((
        Name::new("AssetService"),
        crate::common::net::components::AssetServiceContainer, //for assets, images and in the future meshes and sounds
        Replicate::default(),
    ));
}