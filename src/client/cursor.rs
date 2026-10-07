use bevy::prelude::*;
use bevy::window::{CursorIcon, CustomCursor, CustomCursorImage};

pub const CLIENT_CURSOR_ASSET_PATH: &str = "content/game/ui/cursor.png";

pub fn apply_client_cursor(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    windows: Query<(Entity, Option<&CursorIcon>), With<Window>>,
    mut cached: Local<Option<Handle<Image>>>,
    mut egui_global: Option<ResMut<bevy_egui::EguiGlobalSettings>>,
    mut egui_contexts: Query<&mut bevy_egui::EguiContextSettings>,
    playtest: Option<Res<super::PlaytestState>>,
) {
    if !super::is_playtesting(playtest) {
        return;
    }
    if let Some(global) = egui_global.as_mut() {
        if global.enable_cursor_icon_updates {
            global.enable_cursor_icon_updates = false;
        }
    }
    for mut settings in &mut egui_contexts {
        if settings.enable_cursor_icon_updates {
            settings.enable_cursor_icon_updates = false;
        }
    }
    let handle = cached
        .get_or_insert_with(|| asset_server.load(CLIENT_CURSOR_ASSET_PATH))
        .clone();
    let custom = CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
        handle,
        hotspot: (0, 0),
        ..default()
    }));
    for (entity, current) in &windows {
        if current != Some(&custom) {
            commands.entity(entity).insert(custom.clone());
        }
    }
}
