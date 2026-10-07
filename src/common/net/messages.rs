use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct GameChannel;

pub struct InputChannel;

#[derive(Message, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PlayerMoveMessage {
    pub position: Vec3,
    pub velocity: Vec3,
    pub grounded: bool,
    pub yaw: f32,
    pub in_first_person: bool,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct HelloMessage {
    pub ukey: String,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct KickMessage {
    pub reason: String,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct AuthSuccessMessage {
    pub uid: i32,
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSoundKind {
    StepStart,
    StepStop,
    Jump,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct PlayerSoundRequest {
    pub kind: PlayerSoundKind,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct PlayerSoundBroadcast {
    pub kind: PlayerSoundKind,
    pub position: Vec3,
    pub source_client_id: u64,
}

pub const CHAT_MAX_CHARS: usize = 100;
pub const CHAT_COOLDOWN_SECS: f64 = 5.0;
pub const CHAT_MAX_MESSAGES: usize = 100;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct ChatSendMessage {
    pub text: String,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct ChatBroadcastMessage {
    pub username: String,
    pub text: String,
}

pub fn truncate_chat_text(text: &str) -> String {
    if text.chars().count() <= CHAT_MAX_CHARS {
        text.to_string()
    } else {
        text.chars().take(CHAT_MAX_CHARS).collect()
    }
}

pub fn sanitize_chat_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(truncate_chat_text(trimmed))
}

pub fn chat_cooldown_remaining(last_sent_at: Option<f64>, now: f64) -> f64 {
    match last_sent_at {
        Some(last) => (last + CHAT_COOLDOWN_SECS - now).max(0.0),
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_to_100_chars() {
        let long = "a".repeat(150);
        assert_eq!(truncate_chat_text(&long).chars().count(), CHAT_MAX_CHARS);
        assert_eq!(truncate_chat_text("hi").chars().count(), 2);
    }

    #[test]
    fn rejects_blank_messages() {
        assert!(sanitize_chat_text("").is_none());
        assert!(sanitize_chat_text("   ").is_none());
        assert!(sanitize_chat_text("\n\t ").is_none());
    }

    #[test]
    fn trims_and_truncates() {
        let with_space = format!("  {}  ", "b".repeat(150));
        let cleaned = sanitize_chat_text(&with_space).unwrap();
        assert_eq!(cleaned.chars().count(), CHAT_MAX_CHARS);
        assert!(!cleaned.starts_with(' '));
    }

    #[test]
    fn cooldown_blocks_early_resend() {
        assert!(chat_cooldown_remaining(Some(10.0), 12.0) > 0.0);
        assert_eq!(chat_cooldown_remaining(Some(10.0), 15.0), 0.0);
        assert_eq!(chat_cooldown_remaining(Some(10.0), 20.0), 0.0);
        assert_eq!(chat_cooldown_remaining(None, 10.0), 0.0);
    }
}
