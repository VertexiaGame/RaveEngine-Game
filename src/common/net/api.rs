use std::time::Duration;

pub fn api_base() -> String {
    let domain = std::env::var("VERTIGO_API_DOMAIN").unwrap_or_else(|_| "localhost:3000".to_string());
    if domain.starts_with("http://") || domain.starts_with("https://") {
        domain
    } else {
        format!("http://{domain}")
    }
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
