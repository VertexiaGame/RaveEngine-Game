pub mod bricks;
pub mod movement;
pub mod physics;
pub mod assets;

use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bricks::BricksPlugin)
           .add_plugins(physics::PhysicsSimulationPlugin)
           .add_plugins(assets::AssetsPlugin);
    }
}