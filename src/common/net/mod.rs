pub mod components;
pub mod messages;
pub mod auth;

use bevy::prelude::*;
use lightyear::prelude::*;

pub mod replicon {
    pub use bevy_replicon::prelude::*;
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

    app.add_channel::<messages::GameChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        ..default()
    })
    .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);

    app.component::<components::Player>().replicate();
    app.component::<components::NetworkTransform>().replicate();
    app.component::<components::PlayersServiceContainer>().replicate();
    app.component::<crate::common::game::bricks::components::Brick>().replicate();
    app.component::<crate::common::game::bricks::components::BrickShapeComponent>().replicate();
    app.component::<crate::common::game::bricks::components::BrickPhysics>().replicate();
    app.component::<crate::common::game::bricks::components::BrickColor>().replicate();

    app.register_message::<messages::PlayerInputMessage>()
        .add_direction(lightyear::prelude::NetworkDirection::ClientToServer);
}