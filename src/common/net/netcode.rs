use bevy::log::warn;

pub fn parse_hex_key(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(key)
}

pub fn format_hex_key(key: &[u8; 32]) -> String {
    key.iter().map(|b| format!("{b:02x}")).collect()
}
//Wow okay
pub fn netcode_private_key(cli: Option<&str>) -> [u8; 32] {
    if let Some(value) = cli {
        if let Some(key) = parse_hex_key(value) {
            return key;
        }
        warn!("invalid --netcode-key");
    }
    if let Ok(value) = std::env::var("NETCODE_PRIVATE_KEY") {
        if let Some(key) = parse_hex_key(&value) {
            return key;
        }
        warn!("Well this isn't very sigma of you, isn't it?");
    }
    let key: [u8; 32] = rand::random();
    warn!(
        "lahcn client with NETCODE_PRIVATE_KEY or ELSE. ",
        format_hex_key(&key)
    );
    key
}

pub fn netcode_protocol_id(cli: Option<u64>) -> u64 {
    if let Some(id) = cli {
        return id;
    }
    if let Ok(value) = std::env::var("NETCODE_PROTOCOL_ID") {
        if let Ok(id) = value.trim().parse::<u64>() {
            return id;
        }
        warn!("Invalid NETCODE_PROTOCOL_ID env var; defaulting to 0");
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_key_accepts_exactly_64_hex_chars() {
        let key = parse_hex_key(&"ab".repeat(32)).expect("valid 64-char hex key");
        assert_eq!(key[0], 0xab);
        assert_eq!(key[31], 0xab);
    }

    #[test]
    fn parse_hex_key_rejects_bad_lengths_and_garbage() {
        assert!(parse_hex_key("").is_none());
        assert!(parse_hex_key(&"ab".repeat(31)).is_none());
        assert!(parse_hex_key(&"zz".repeat(32)).is_none());
        assert!(parse_hex_key(&"AB".repeat(32)).is_some(), "uppercase hex is fine");
    }

    #[test]
    fn format_hex_key_round_trips() {
        let key = parse_hex_key(&"42".repeat(32)).unwrap();
        assert_eq!(format_hex_key(&key), "42".repeat(32));
    }

    #[test]
    fn protocol_id_falls_back_to_zero() {
        assert_eq!(netcode_protocol_id(Some(7)), 7);
        assert_eq!(netcode_protocol_id(None), 0);
    }
}