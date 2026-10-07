pub mod assets_path;
pub mod core;
pub mod game;
pub mod net;

use bevy::prelude::*;

pub struct CommonPlugin;

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(game::GamePlugin)
            .add_plugins(core::CorePlugin)
            .add_plugins(crate::scripting::plugin::ScriptingPlugin);
    }
}
