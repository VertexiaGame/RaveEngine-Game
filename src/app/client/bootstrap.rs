use bevy::prelude::*;
use lightyear::prelude::client::ClientPlugins;
use std::time::Duration;
use crate::app::client::config::ClientAppConfig;
use crate::app::client::network_boot::{
    ClientConnectSettings, ClientSpawned, poll_launch_details, initialize_client, trigger_delayed_connect
};
use crate::common::CommonPlugin;
use crate::client::ClientPlugin;
use crate::app::common::log::setup_app_logging;

pub struct RaveClientApp {
    config: ClientAppConfig,
}

impl RaveClientApp {
    pub fn new(config: ClientAppConfig) -> Self {
        Self { config }
    }

    pub fn run(self) {
        let log_plugin = setup_app_logging("client");

        let mut app = App::new();
        app.add_plugins(DefaultPlugins.set(log_plugin).set(bevy::render::RenderPlugin {
            render_creation: bevy::render::settings::RenderCreation::Automatic(Box::new(
                bevy::render::settings::WgpuSettings {
                    disabled_features: Some(bevy::render::settings::WgpuFeatures::TEXTURE_BINDING_ARRAY),
                    ..default()
                }
            )),
            ..default()
        }));
        app.insert_resource(ClientConnectSettings {
            ip: self.config.ip,
            port: self.config.port,
            netcode_key: self.config.netcode_key,
            protocol_id: self.config.protocol_id,
        });
        app.insert_resource(crate::client::ClientUkey(self.config.ukey));
        app.init_resource::<ClientSpawned>();
        app.add_plugins(ClientPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
        });
        app.add_plugins(CommonPlugin);
        app.add_plugins(ClientPlugin);
        app.add_systems(Update, (
            initialize_client,
            trigger_delayed_connect,
            poll_launch_details,
        ));
        app.run();
    }
}