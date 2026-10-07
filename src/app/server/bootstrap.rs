use crate::app::common::log::setup_app_logging;
use crate::app::server::config::ServerAppConfig;
use crate::common::CommonPlugin;
use crate::server::ServerPlugin;
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

pub static SHUTDOWN_SERVER: AtomicBool = AtomicBool::new(false);
pub static PLAYTEST_SERVER_EPOCH: AtomicU64 = AtomicU64::new(0);

#[derive(Resource, Clone, Copy)]
struct PlaytestServerEpoch(u64);

pub fn request_playtest_server_shutdown() {
    SHUTDOWN_SERVER.store(true, Ordering::SeqCst);
    PLAYTEST_SERVER_EPOCH.fetch_add(1, Ordering::SeqCst);
}

pub fn begin_playtest_server_epoch() {
    PLAYTEST_SERVER_EPOCH.fetch_add(1, Ordering::SeqCst);
    SHUTDOWN_SERVER.store(false, Ordering::SeqCst);
}

pub fn pick_free_playtest_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .and_then(|socket| socket.local_addr())
        .map(|addr| addr.port())
        .unwrap_or(5000)
}

pub struct RaveServerApp {
    config: ServerAppConfig,
    epoch: u64,
}

impl RaveServerApp {
    pub fn new(config: ServerAppConfig) -> Self {
        Self {
            config,
            epoch: PLAYTEST_SERVER_EPOCH.load(Ordering::SeqCst),
        }
    }

    pub fn run(self) {
        let mut app = App::new();
        if std::env::var("VERTIGO_APP").unwrap_or_default() == "server" {
            let log_plugin = setup_app_logging("server");
            app.add_plugins(log_plugin);
        }
        app.add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))),
        );
        app.add_plugins(crate::app::common::assets::asset_plugin());
        app.init_asset::<Mesh>();
        app.add_plugins(StatesPlugin);
        app.add_plugins(TransformPlugin);
        app.add_plugins(CommonPlugin);
        app.add_plugins(ServerPlugin {
            map_path: self.config.map_path,
            port: self.config.port,
            bind_addr: self.config.bind_addr,
            netcode_key: self.config.netcode_key,
            protocol_id: self.config.protocol_id,
            allow_unauthenticated: self.config.allow_unauthenticated,
        });
        app.add_systems(Update, check_thread_shutdown);
        app.insert_resource(PlaytestServerEpoch(self.epoch));
        app.run();
    }
}

fn check_thread_shutdown(
    epoch: Res<PlaytestServerEpoch>,
    mut exit_writer: MessageWriter<AppExit>,
) {
    if SHUTDOWN_SERVER.load(Ordering::SeqCst)
        || PLAYTEST_SERVER_EPOCH.load(Ordering::SeqCst) != epoch.0
    {
        exit_writer.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playtest_restart_epoch_never_resurrects_old_server() {
        let first = PLAYTEST_SERVER_EPOCH.load(Ordering::SeqCst);
        begin_playtest_server_epoch();
        let started = PLAYTEST_SERVER_EPOCH.load(Ordering::SeqCst);
        assert_eq!(started, first + 1);
        assert!(!SHUTDOWN_SERVER.load(Ordering::SeqCst));
        request_playtest_server_shutdown();
        assert!(SHUTDOWN_SERVER.load(Ordering::SeqCst));
        begin_playtest_server_epoch();
        let restarted = PLAYTEST_SERVER_EPOCH.load(Ordering::SeqCst);
        assert!(restarted > started);
        assert!(!SHUTDOWN_SERVER.load(Ordering::SeqCst));
        request_playtest_server_shutdown();
    }

    #[test]
    fn playtest_port_picker_returns_usable_port() {
        let port = pick_free_playtest_port();
        assert_ne!(port, 0);
    }
}
