use bevy::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use bevy::state::app::StatesPlugin;
use crate::app::server::config::ServerAppConfig;
use crate::common::CommonPlugin;
use crate::server::ServerPlugin;
use crate::app::common::log::setup_app_logging;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub static SHUTDOWN_SERVER: AtomicBool = AtomicBool::new(false);

pub struct RaveServerApp {
    config: ServerAppConfig,
}

impl RaveServerApp {
    pub fn new(config: ServerAppConfig) -> Self {
        Self { config }
    }

    pub fn run(self) {
        let mut app = App::new();
        if std::env::var("VERTIGO_APP").unwrap_or_default() == "server" {
            let log_plugin = setup_app_logging("server");
            app.add_plugins(log_plugin);
        }
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))));
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Mesh>();
        app.add_plugins(StatesPlugin);
        app.add_plugins(TransformPlugin);
        app.add_plugins(CommonPlugin);
        app.add_plugins(ServerPlugin {
            map_path: self.config.map_path,
            port: self.config.port,
        });
        app.add_systems(Update, check_thread_shutdown);
        app.run();
    }
}

fn check_thread_shutdown(mut exit_writer: MessageWriter<AppExit>) {
    if SHUTDOWN_SERVER.load(Ordering::Relaxed) {
        exit_writer.write(AppExit::Success);
    }
}