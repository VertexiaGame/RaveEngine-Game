pub mod api;
pub mod auth;
pub mod components;
pub mod messages;
pub mod netcode;

use bevy::prelude::*;
use lightyear::prelude::*;
use std::time::Duration;

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        register_protocol(app);
    }
}

pub fn register_protocol(app: &mut App) {
    app.register_type::<components::Player>();
    app.register_type::<components::NetworkTransform>();
    app.register_type::<components::PlayersServiceContainer>();
    app.register_type::<components::LightingServiceContainer>();
    app.register_type::<components::AssetServiceContainer>();

    app.add_channel::<messages::GameChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        ..default()
    })
    .add_direction(lightyear::prelude::NetworkDirection::ClientToServer)
    .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);

    app.add_channel::<messages::InputChannel>(ChannelSettings {
        mode: ChannelMode::SequencedUnreliable,
        send_frequency: Duration::from_secs_f64(1.0 / 60.0),
        ..default()
    })
    .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.component::<components::Player>().replicate();
    app.component::<components::NetworkTransform>().replicate();
    app.component::<components::PlayersServiceContainer>()
        .replicate();
    app.component::<components::LightingServiceContainer>()
        .replicate();
    app.component::<components::AssetServiceContainer>()
        .replicate();
    app.component::<crate::common::game::assets::components::Image>()
        .replicate();
    app.register_type::<crate::common::game::assets::components::ImageFace>();
    app.component::<crate::common::game::assets::components::Mesh>()
        .replicate();
    app.component::<crate::common::game::assets::components::Texture>()
        .replicate();
    app.component::<crate::common::game::assets::components::Sound>()
        .replicate();
    app.component::<crate::common::game::bricks::components::Brick>()
        .replicate();
    app.component::<crate::common::game::bricks::components::BrickShapeComponent>()
        .replicate();
    app.component::<crate::common::game::bricks::components::BrickPhysics>()
        .replicate();
    app.component::<crate::common::game::bricks::components::BrickColor>()
        .replicate();
    app.component::<crate::common::game::bricks::components::BrickStuds>()
        .replicate();
    app.component::<crate::scripting::ecs::LocalScript>()
        .replicate();
    app.component::<crate::scripting::ecs::ModuleScript>()
        .replicate();

    app.register_message::<messages::PlayerMoveMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.register_message::<messages::HelloMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.register_message::<messages::KickMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);

    app.register_message::<messages::AuthSuccessMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);

    app.register_message::<messages::ChatSendMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.register_message::<messages::ChatBroadcastMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);

    app.register_message::<messages::PlayerSoundRequest>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.register_message::<messages::PlayerSoundBroadcast>()
        .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);
}
