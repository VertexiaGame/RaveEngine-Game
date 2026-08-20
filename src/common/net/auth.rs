use std::time::Duration;
use serde::{Deserialize, Serialize};
use bevy::log::{warn, debug};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidateResponse {
    pub uid: i32,
    pub username: String,
}
pub fn validate_user_ukey(ukey: &str, allow_unauthenticated: bool) -> Result<ValidateResponse, String> {
    if allow_unauthenticated && (ukey == "studio_play_local_key" || ukey.starts_with("offline_")) {
        //^^ we allow unathenticated responses if the user is running a playtest / defined with ukey "studio_play_local_key"
        //maybe um to be rewritten later
        return Ok(ValidateResponse {
            //set junk data
            uid: 1,
            username: "LocalPlayer".to_string(),
        });
    }

    let base = crate::common::net::api::api_base();
    let api_key = crate::common::net::api::gameserver_api_key()?;

    trace_api(&format!(
        "Starting validation with domain={}, api_key_length={}",
        base,
        api_key.len()
    ));

    let url = format!("{base}/api/v1/auth/validate");

    let client = crate::common::net::api::blocking_client(Duration::from_secs(5))?;

    let resp = client
        .get(&url)
        .query(&[("ukey", ukey)])
        .header("X-Gameserver-Key", api_key)
        .send()
        .map_err(|e| format!("auth request failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().map_err(|e| format!("failed to read auth response: {e}"))?;

    if !status.is_success() {
        warn!("API_LOG: Go backend returned non-200 status {status}: {body}");
        return Err(format!("Server returned error: {status}"));
    }

    let res_data: ValidateResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    debug!("API_LOG: Successfully validated client ukey uid={}, username={}", res_data.uid, res_data.username);
    Ok(res_data)
}

fn trace_api(msg: &str) {
    bevy::log::trace!("API_LOG: {msg}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_keys_rejected_when_unauthenticated_is_disabled() {
        for ukey in ["offline_test_user", "offline_another", "studio_play_local_key"] {
            let result = validate_user_ukey(ukey, false);
            assert!(result.is_err(), "ukey `{ukey}` must be rejected on public servers");
        }
    }

    #[test]
    fn offline_keys_accepted_when_unauthenticated_is_enabled() {
        for ukey in ["offline_test_user", "studio_play_local_key"] {
            let result = validate_user_ukey(ukey, true).expect("offline ukey accepted in studio mode");
            assert_eq!(result.uid, 1);
            assert_eq!(result.username, "LocalPlayer");
        }
    }

    #[test]
    fn unknown_keys_never_short_circuit() {
        let result = validate_user_ukey("some_random_client_ukey", true);
        assert!(
            result.is_err(),
            "a real-looking ukey must not bypass the backend (expects GAMESERVER_API_KEY)"
        );
    }
}