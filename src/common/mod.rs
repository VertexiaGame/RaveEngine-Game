pub mod bricks;
pub mod environment;
pub mod physics;
pub mod performance;
pub mod vrtx;
pub mod components;
pub mod network;
pub mod vuis;

use bevy::prelude::*;

pub struct CommonPlugin;

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(environment::EnvironmentPlugin)
           .add_plugins(physics::PhysicsSimulationPlugin)
           .add_plugins(bricks::BricksPlugin)
           .add_plugins(performance::PerformancePlugin)
           .add_plugins(vuis::VuisPlugin);
    }
}