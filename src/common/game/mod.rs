pub mod bricks;
pub mod physics;

use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bricks::BricksPlugin)
           .add_plugins(physics::PhysicsSimulationPlugin);
    }
}