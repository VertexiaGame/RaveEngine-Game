use std::time::Duration;

pub const API_BASE_URL: &str = "http://localhost:3000";

pub fn api_base() -> String {
    if let Ok(value) = std::env::var("VERTIGO_API_DOMAIN") {
        let trimmed = value.trim().trim_end_matches('/').to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    API_BASE_URL.to_string()
}

pub fn gameserver_api_key() -> Result<String, String> {
    let api_key = std::env::var("GAMESERVER_API_KEY")
        .map_err(|_| "GAMESERVER_API_KEY environment variable is not configured".to_string())?;
    Ok(api_key.trim().trim_matches('"').to_string())
}

pub fn blocking_client(timeout: Duration) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))
}

static SHARED_AUTH_CLIENT: std::sync::OnceLock<reqwest::blocking::Client> =
    std::sync::OnceLock::new();

pub fn shared_auth_client() -> Result<&'static reqwest::blocking::Client, String> {
    if let Some(client) = SHARED_AUTH_CLIENT.get() {
        return Ok(client);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;
    Ok(SHARED_AUTH_CLIENT.get_or_init(|| client))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_base_prefers_env_domain() {
        let prev = std::env::var("VERTIGO_API_DOMAIN").ok();
        unsafe {
            std::env::set_var("VERTIGO_API_DOMAIN", "http://127.0.0.1:3000/");
        }
        assert_eq!(api_base(), "http://127.0.0.1:3000");
        unsafe {
            std::env::remove_var("VERTIGO_API_DOMAIN");
        }
        assert_eq!(api_base(), API_BASE_URL);
        if let Some(value) = prev {
            unsafe {
                std::env::set_var("VERTIGO_API_DOMAIN", value);
            }
        }
    }
}
