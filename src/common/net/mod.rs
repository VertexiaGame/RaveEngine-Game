pub mod components;
pub mod messages;
pub mod auth;

use bevy::prelude::*;
use lightyear::prelude::*;
use std::time::Duration;

pub mod replicon {
    pub use bevy_replicon::prelude::*;
}

fn lerp_network_transform(
    start: components::NetworkTransform,
    end: components::NetworkTransform,
    t: f32,
) -> components::NetworkTransform {
    components::NetworkTransform {
        translation: start.translation.lerp(end.translation, t),
        rotation: start.rotation.slerp(end.rotation, t),
        scale: start.scale.lerp(end.scale, t),
    }
}

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, _app: &mut App) {}
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        register_protocol(app);
    }
}

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
    app.component::<components::NetworkTransform>()
        .replicate()
        .add_interpolation_with(lerp_network_transform);
    app.component::<components::PlayersServiceContainer>().replicate();
    app.component::<components::LightingServiceContainer>().replicate();
    app.component::<crate::common::game::bricks::components::Brick>().replicate();
    app.component::<crate::common::game::bricks::components::BrickShapeComponent>().replicate();
    app.component::<crate::common::game::bricks::components::BrickPhysics>().replicate();
    app.component::<crate::common::game::bricks::components::BrickColor>().replicate();
    app.component::<crate::scripting::ecs::LocalScript>().replicate();
    app.component::<crate::scripting::ecs::ModuleScript>().replicate();

    app.register_message::<messages::PlayerInputMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.register_message::<messages::HelloMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.register_message::<messages::KickMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);

    app.register_message::<messages::AuthSuccessMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ServerToClient);
}