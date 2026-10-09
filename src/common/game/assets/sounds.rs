use std::collections::{HashMap, HashSet};
use std::time::Duration;

use bevy::audio::{
    AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, Decodable, PlaybackSettings, Source,
    SpatialAudioSink, SpatialScale, Volume,
};
use bevy::log::{info, warn};
use bevy::prelude::*;

use super::components::Sound;
use super::fetch;

pub const SOUND_SYNC_INTERVAL_SECS: f32 = 0.5;

#[derive(Resource, Default)]
pub struct SoundAssetCache {
    pub sounds: HashMap<u32, Handle<AudioSource>>,
    pub durations: HashMap<u32, Option<f32>>,
}

#[derive(Resource, Default)]
pub struct PendingSoundFetches(pub HashSet<u32>);

#[derive(bevy::ecs::system::SystemParam)]
pub struct SoundApplyParams<'w> {
    pub pending: ResMut<'w, PendingSoundFetches>,
    pub cache: ResMut<'w, SoundAssetCache>,
    pub audio: Option<ResMut<'w, Assets<AudioSource>>>,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SoundVoice {
    pub asset_id: u32,
    pub looped: bool,
    pub spatial: bool,
}

pub fn is_supported_audio_bytes(bytes: &[u8]) -> bool {
    if bytes.len() < 12 {
        return false;
    }
    if bytes.starts_with(b"ID3") || (bytes[0] == 0xFF && bytes[1] & 0xE0 == 0xE0) {
        return true;
    }
    if bytes.starts_with(b"RIFF") && bytes.len() > 11 && &bytes[8..12] == b"WAVE" {
        return true;
    }
    if bytes.starts_with(b"OggS") {
        return true;
    }
    false
}

fn probe_duration(source: &AudioSource) -> Option<f32> {
    source
        .decoder()
        .total_duration()
        .map(|d| d.as_secs_f32())
        .filter(|d| d.is_finite() && *d > 0.01)
}

pub fn cache_sound_bytes(
    cache: &mut SoundAssetCache,
    audio_assets: &mut Assets<AudioSource>,
    asset_id: u32,
    bytes: &[u8],
) -> bool {
    if !is_supported_audio_bytes(bytes) {
        warn!("Asset {asset_id}: rejected non-audio sound bytes");
        return false;
    }
    let source = AudioSource {
        bytes: bytes.to_vec().into(),
    };
    let duration = probe_duration(&source);
    let handle = audio_assets.add(source);
    info!("Asset {asset_id}: loaded sound (duration: {duration:?})");
    cache.sounds.insert(asset_id, handle);
    cache.durations.insert(asset_id, duration);
    true
}

pub fn synced_start_offset(position_secs: f32, duration_secs: Option<f32>) -> Option<f32> {
    if !position_secs.is_finite() || position_secs <= 0.0 {
        return None;
    }
    match duration_secs {
        Some(d) if d.is_finite() && d > 0.01 => Some(position_secs % d),
        _ => None,
    }
}

pub fn fetch_missing_sounds(
    sounds: Query<&Sound>,
    cache: Res<SoundAssetCache>,
    mut pending: ResMut<PendingSoundFetches>,
    mut pool: ResMut<fetch::AssetFetchPool>,
    auth_store: Option<Res<crate::studio::auth::StudioAuthStore>>,
    client_ukey: Option<Res<crate::client::ClientUkey>>,
    server: Option<Res<crate::server::ServerSettings>>,
) {
    if server.is_some() {
        return;
    }
    let user_ukey = super::current_user_ukey(auth_store, client_ukey);
    for sound in &sounds {
        let asset_id = sound.asset_id;
        if asset_id == 0 || cache.sounds.contains_key(&asset_id) || pending.0.contains(&asset_id) {
            continue;
        }
        pool.ensure_started();
        if pool.submit_kind(asset_id, fetch::AssetKind::Sound, user_ukey.clone()) {
            pending.0.insert(asset_id);
        }
    }
}

pub fn advance_sound_positions(
    time: Res<Time>,
    mut accumulator: Local<f32>,
    mut sounds: Query<&mut Sound>,
) {
    let dt = time.delta_secs().min(0.25);
    if dt > 0.0 {
        *accumulator += dt;
    }
    if *accumulator < SOUND_SYNC_INTERVAL_SECS {
        return;
    }
    let step = *accumulator;
    *accumulator = 0.0;
    for mut sound in &mut sounds {
        if !sound.playing || sound.asset_id == 0 {
            continue;
        }
        if !sound.replicate_time {
            if sound.position != 0.0 {
                sound.position = 0.0;
            }
            continue;
        }
        let speed = sound.clamped_speed();
        let next = sound.position + step * speed;
        sound.position = if next.is_finite() { next } else { 0.0 };
    }
}

pub fn reset_sound_positions_on_change(
    mut sounds: Query<(Entity, &mut Sound), Changed<Sound>>,
    mut last_assets: Local<HashMap<Entity, u32>>,
) {
    for (entity, mut sound) in &mut sounds {
        let last = last_assets.get(&entity).copied();
        if last.is_some_and(|id| id != sound.asset_id) && sound.position != 0.0 {
            sound.position = 0.0;
        }
        last_assets.insert(entity, sound.asset_id);
    }
}

fn sound_playback_settings(sound: &Sound, start_offset: Option<f32>) -> PlaybackSettings {
    let base = if sound.looped {
        PlaybackSettings::LOOP
    } else {
        PlaybackSettings::ONCE
    };
    let with_offset = match start_offset {
        Some(offset) => {
            base.with_start_position(Duration::from_secs_f32(offset.max(0.0)))
        }
        None => base,
    };
    with_offset
        .with_volume(Volume::Linear(sound.linear_volume()))
        .with_speed(sound.clamped_speed())
        .with_spatial(sound.spatial)
        .with_spatial_scale(SpatialScale::new(
            crate::common::game::sounds::SPATIAL_SCALE,
        ))
}

pub fn update_sound_playback(
    mut commands: Commands,
    cache: Res<SoundAssetCache>,
    mut sounds: Query<(
        Entity,
        &Sound,
        Option<&AudioPlayer>,
        Option<&SoundVoice>,
    )>,
) {
    for (entity, sound, player_opt, voice_opt) in &mut sounds {
        let handle_opt = if sound.asset_id != 0 {
            cache.sounds.get(&sound.asset_id).cloned()
        } else {
            None
        };
        let should_play = sound.playing && handle_opt.is_some();
        if !should_play {
            if player_opt.is_some() || voice_opt.is_some() {
                commands.entity(entity).remove::<(
                    AudioPlayer,
                    PlaybackSettings,
                    AudioSink,
                    SpatialAudioSink,
                    SoundVoice,
                )>();
            }
            continue;
        }
        let handle = match handle_opt {
            Some(handle) => handle,
            None => Handle::default(),
        };
        let needs_restart = match voice_opt {
            None => true,
            Some(voice) => {
                voice.asset_id != sound.asset_id
                    || voice.looped != sound.looped
                    || voice.spatial != sound.spatial
                    || player_opt.is_none()
            }
        };
        if !needs_restart {
            continue;
        }
        let start_offset = if sound.replicate_time {
            synced_start_offset(sound.position, cache.durations.get(&sound.asset_id).copied().flatten())
        } else {
            None
        };
        let settings = sound_playback_settings(sound, start_offset);
        commands.entity(entity).remove::<(
            AudioPlayer,
            PlaybackSettings,
            AudioSink,
            SpatialAudioSink,
            SoundVoice,
        )>();
        commands
            .entity(entity)
            .insert((AudioPlayer::new(handle), settings));
        commands.entity(entity).insert(SoundVoice {
            asset_id: sound.asset_id,
            looped: sound.looped,
            spatial: sound.spatial,
        });
    }
}

pub fn apply_live_sound_settings(
    sounds: Query<(Entity, &Sound, Option<&SoundVoice>)>,
    mut sinks: Query<&mut AudioSink>,
    mut spatial_sinks: Query<&mut SpatialAudioSink>,
) {
    for (entity, sound, voice_opt) in &sounds {
        if voice_opt.is_none() || !sound.playing {
            continue;
        }
        let volume = Volume::Linear(sound.linear_volume());
        let speed = sound.clamped_speed();
        if let Ok(mut sink) = sinks.get_mut(entity) {
            sink.set_volume(volume);
            sink.set_speed(speed);
        }
        if let Ok(mut sink) = spatial_sinks.get_mut(entity) {
            sink.set_volume(volume);
            sink.set_speed(speed);
        }
    }
}

pub struct SoundInstancesPlugin;

impl Plugin for SoundInstancesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundAssetCache>()
            .init_resource::<PendingSoundFetches>()
            .add_systems(
                Update,
                (update_sound_playback, apply_live_sound_settings)
                    .run_if(crate::client::is_playtesting),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::super::components::Sound;
    use super::*;

    #[test]
    fn volume_clamps_to_0_100() {
        assert_eq!(Sound::clamp_volume(50.0), 50.0);
        assert_eq!(Sound::clamp_volume(250.0), 100.0);
        assert_eq!(Sound::clamp_volume(-5.0), 0.0);
        assert_eq!(Sound::clamp_volume(f32::NAN), 50.0);
        assert!((Sound::default().linear_volume() - 0.5).abs() < 1e-6);
        assert!((Sound { volume: 100.0, ..Sound::default() }.linear_volume() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn speed_clamps_to_playable_range() {
        assert_eq!(Sound::clamp_speed(1.0), 1.0);
        assert_eq!(Sound::clamp_speed(0.0), Sound::MIN_SPEED);
        assert_eq!(Sound::clamp_speed(500.0), Sound::MAX_SPEED);
        assert_eq!(Sound::clamp_speed(f32::NAN), 1.0);
    }

    #[test]
    fn sound_content_id_round_trips() {
        assert_eq!(Sound::default().as_content_id(), "sound/0");
        assert_eq!(
            Sound { asset_id: 42, ..Sound::default() }.as_content_id(),
            "sound/42"
        );
        assert_eq!(Sound::parse_content_id("sound/42"), Some(42));
        assert_eq!(Sound::parse_content_id("42"), Some(42));
        assert_eq!(Sound::parse_content_id("mesh/42"), None);
        assert_eq!(Sound::parse_content_id("nope"), None);
    }

    #[test]
    fn sync_offset_uses_clip_duration() {
        assert_eq!(synced_start_offset(0.0, Some(120.0)), None);
        assert_eq!(synced_start_offset(65.0, Some(60.0)), Some(5.0));
        assert_eq!(synced_start_offset(65.0, None), None);
        assert_eq!(synced_start_offset(f32::NAN, Some(60.0)), None);
    }

    #[test]
    fn audio_magic_validation_rejects_image_fallbacks() {
        let png = [0x89u8, b'P', b'N', b'G', 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(!is_supported_audio_bytes(&png));
        assert!(!is_supported_audio_bytes(&[]));
        let wav = [b'R', b'I', b'F', b'F', 0, 0, 0, 0, b'W', b'A', b'V', b'E'];
        assert!(is_supported_audio_bytes(&wav));
        let mp3 = [b'I', b'D', b'3', 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(is_supported_audio_bytes(&mp3));
        let ogg = [b'O', b'g', b'g', b'S', 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(is_supported_audio_bytes(&ogg));
    }

    #[test]
    fn sound_defaults_match_spec() {
        let s = Sound::default();
        assert_eq!(s.asset_id, 0);
        assert!(!s.looped);
        assert!(!s.replicate_time);
        assert!(!s.playing);
        assert!(!s.spatial);
        assert_eq!(s.position, 0.0);
    }
}
