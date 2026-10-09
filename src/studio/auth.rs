use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender, unbounded};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredentials {
    pub username: String,
    pub uid: i32,
    pub ukey: String,
}

pub fn website_base() -> String {
    crate::common::net::api::api_base()
}

pub fn generate_state() -> String {
    let bytes: [u8; 16] = rand::random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn auth_file_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("vertexia_studio_auth.json"));
        }
    }
    if let Ok(dir) = std::env::current_dir() {
        candidates.push(dir.join("vertexia_studio_auth.json"));
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        candidates.push(PathBuf::from(appdata).join("VERTEXIA").join("studio_auth.json"));
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(PathBuf::from(home.clone()).join(".vertexia").join("studio_auth.json"));
        candidates.push(PathBuf::from(home).join(".config").join("vertexia").join("studio_auth.json"));
    }
    candidates
}

pub fn auth_file_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("vertexia_studio_auth.json");
        }
    }
    auth_file_candidates()
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("vertexia_studio_auth.json"))
}

pub fn load_studio_credentials() -> Option<StoredCredentials> {
    for path in auth_file_candidates() {
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(creds) = serde_json::from_str::<StoredCredentials>(&data) {
                    if !creds.ukey.is_empty() {
                        return Some(creds);
                    }
                }
            }
        }
    }
    None
}

pub fn save_studio_credentials(creds: &StoredCredentials) -> Result<(), String> {
    let path = auth_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(creds).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())?;
    for alt in auth_file_candidates() {
        if alt != path && alt.exists() {
            let _ = std::fs::remove_file(alt);
        }
    }
    Ok(())
}

pub fn clear_studio_credentials() {
    for path in auth_file_candidates() {
        let _ = std::fs::remove_file(path);
    }
}

pub fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        if std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .is_ok()
        {
            return;
        }
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &format!("\"{url}\"")])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = url;
    }
}

pub fn parse_query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let mut iter = pair.splitn(2, '=');
        let k = iter.next()?;
        let v = iter.next().unwrap_or("");
        if k == key {
            return Some(url_decode(v));
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            let hex = format!("{h1}{h2}");
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                out.push(byte as char);
            } else {
                out.push('%');
                out.push(h1);
                out.push(h2);
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

pub fn try_extract_code_from_args() -> Option<(String, String)> {
    for arg in std::env::args().skip(1) {
        let lower = arg.to_lowercase();
        if lower.starts_with("vertexia-studio://") || lower.starts_with("vertexia://") {
            if let Some(q) = arg.split('?').nth(1) {
                let code = parse_query_param(q, "code");
                let state = parse_query_param(q, "state").unwrap_or_default();
                if let Some(c) = code {
                    if !c.is_empty() {
                        return Some((c, state));
                    }
                }
            }
        }
        if arg.contains("code=") && (arg.starts_with("http://") || arg.starts_with("vertexia")) {
            if let Some(q) = arg.split('?').nth(1) {
                if let Some(c) = parse_query_param(q, "code") {
                    let state = parse_query_param(q, "state").unwrap_or_default();
                    if !c.is_empty() {
                        return Some((c, state));
                    }
                }
            }
        }
    }
    None
}

pub fn exchange_code(code: &str) -> Result<StoredCredentials, String> {
    let base = website_base();
    let url = format!("{base}/api/v1/studio/exchange");
    let client = crate::common::net::api::blocking_client(Duration::from_secs(10))?;
    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "code": code }))
        .send()
        .map_err(|e| format!("exchange request failed: {e}"))?;
    let status = resp.status();
    let body = resp.text().map_err(|e| format!("read exchange response: {e}"))?;
    if !status.is_success() {
        return Err(format!("exchange failed {status}: {body}"));
    }
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let ukey = v.get("ukey").and_then(|x| x.as_str()).ok_or("missing ukey")?.to_string();
    let username = v.get("username").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let uid = v.get("uid").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    Ok(StoredCredentials { username, uid, ukey })
}

#[derive(Resource)]
pub struct StudioAuthFlow {
    pub state: Option<String>,
    pub redirect_uri: Option<String>,
    pub receiver: Option<Receiver<Result<StoredCredentials, String>>>,
    pub error: Option<String>,
    pub waiting_since: Option<Instant>,
    pub is_polling: bool,
}

impl Default for StudioAuthFlow {
    fn default() -> Self {
        Self {
            state: None,
            redirect_uri: None,
            receiver: None,
            error: None,
            waiting_since: None,
            is_polling: false,
        }
    }
}

#[derive(Resource, Default, Debug, Clone)]
pub struct StudioAuthStore {
    pub credentials: Option<StoredCredentials>,
}

impl StudioAuthStore {
    pub fn is_logged_in(&self) -> bool {
        self.credentials.as_ref().is_some_and(|c| !c.ukey.is_empty())
    }
}


pub const LOGIN_REQUIRED_TOOLTIP: &str =
    "This element cannot be inserted or updated because you are not logged in into VERTEXIA."; //String!

pub fn init_studio_auth(mut store: ResMut<StudioAuthStore>, mut flow: ResMut<StudioAuthFlow>) {
    let creds = load_studio_credentials();
    store.credentials = creds.clone();
    *flow = StudioAuthFlow::default();
    if let Some((code, _state)) = try_extract_code_from_args() {
        match exchange_code(&code) {
            Ok(new_creds) => {
                let _ = save_studio_credentials(&new_creds);
                store.credentials = Some(new_creds);
            }
            Err(e) => {
                bevy::log::warn!("Failed to exchange URI code: {e}");
                flow.error = Some(e);
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let _ = try_register_uri_scheme();
    }
}

#[cfg(target_os = "windows")]
fn try_register_uri_scheme() -> Result<(), String> {
    use std::process::Command;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.display().to_string();
    let ps_script = format!(
        "$p='HKCU:\\Software\\Classes\\vertexia-studio'; if(!(Test-Path $p)){{New-Item -Path $p -Force | Out-Null}}; Set-ItemProperty -Path $p -Name '(Default)' -Value 'URL:VERTEXIA Studio'; Set-ItemProperty -Path $p -Name 'URL Protocol' -Value ''; $c=\"$p\\shell\\open\\command\"; if(!(Test-Path $c)){{New-Item -Path $c -Force | Out-Null}}; Set-ItemProperty -Path $c -Name '(Default)' -Value '\"{exe_str}\" \"%1\"'; $p2='HKCU:\\Software\\Classes\\vertexia'; if(!(Test-Path $p2)){{New-Item -Path $p2 -Force | Out-Null}}; Set-ItemProperty -Path $p2 -Name '(Default)' -Value 'URL:VERTEXIA'; Set-ItemProperty -Path $p2 -Name 'URL Protocol' -Value ''; $c2=\"$p2\\shell\\open\\command\"; if(!(Test-Path $c2)){{New-Item -Path $c2 -Force | Out-Null}}; Set-ItemProperty -Path $c2 -Name '(Default)' -Value '\"{exe_str}\" \"%1\"'",
    );
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output();
    Ok(())
}

pub fn start_studio_auth_flow(flow: &mut StudioAuthFlow) -> Result<(), String> {
    let state = generate_state();
    let (tx, rx): (Sender<Result<StoredCredentials, String>>, Receiver<Result<StoredCredentials, String>>) = unbounded();
    let redirect_uri;
    let expected_state = state.clone();
    let base = website_base();
    match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => {
            let port = listener.local_addr().map_err(|e| e.to_string())?.port();
            redirect_uri = format!("http://127.0.0.1:{port}/callback");
            let base_clone = base.clone();
            let state_clone = expected_state.clone();
            std::thread::spawn(move || {
                run_local_callback_server(listener, state_clone, tx, base_clone);
            });
        }
        Err(e) => {
            bevy::log::warn!("Local callback bind failed ({e}), falling back to URI scheme");
            redirect_uri = "vertexia-studio://auth".to_string();
            let tx2 = tx.clone();
            let state2 = expected_state.clone();
            let base2 = base.clone();
            std::thread::spawn(move || {
                run_polling_only(state2, tx2, base2);
            });
        }
    }
    let website = website_base();
    let auth_url = format!(
        "{website}/studio/auth?state={}&redirect_uri={}",
        urlencoding(&state),
        urlencoding(&redirect_uri)
    );
    open_browser(&auth_url);

    flow.state = Some(state.clone());
    flow.redirect_uri = Some(redirect_uri);
    flow.receiver = Some(rx);
    flow.error = None;
    flow.waiting_since = Some(Instant::now());
    flow.is_polling = true;
    Ok(())
}

fn run_polling_only(expected_state: String, tx: Sender<Result<StoredCredentials, String>>, website_base: String) {
    let start = Instant::now();
    let timeout = Duration::from_secs(300);
    let mut last_poll = Instant::now() - Duration::from_secs(2);
    loop {
        if start.elapsed() > timeout {
            let _ = tx.send(Err("Authentication timed out (5 minutes)".to_string()));
            break;
        }
        if last_poll.elapsed() >= Duration::from_secs(2) {
            last_poll = Instant::now();
            if let Some(code) = poll_for_code(&expected_state, &website_base) {
                match exchange_code_with_base(&code, &website_base) {
                    Ok(creds) => {
                        let _ = tx.send(Ok(creds));
                        break;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn poll_for_code(state: &str, base: &str) -> Option<String> {
    let url = format!("{base}/api/v1/studio/poll?state={}", urlencoding(state));
    let client = crate::common::net::api::blocking_client(Duration::from_secs(3)).ok()?;
    let resp = client.get(&url).send().ok()?;
    let body = resp.text().ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    if v.get("status").and_then(|x| x.as_str()) == Some("authorized") {
        v.get("code").and_then(|x| x.as_str()).map(|s| s.to_string())
    } else {
        None
    }
}

fn exchange_code_with_base(code: &str, base: &str) -> Result<StoredCredentials, String> {
    let url = format!("{base}/api/v1/studio/exchange");
    let client = crate::common::net::api::blocking_client(Duration::from_secs(10))?;
    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "code": code }))
        .send()
        .map_err(|e| format!("exchange request failed: {e}"))?;
    let status = resp.status();
    let body = resp.text().map_err(|e| format!("read exchange response: {e}"))?;
    if !status.is_success() {
        return Err(format!("exchange failed {status}: {body}"));
    }
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let ukey = v.get("ukey").and_then(|x| x.as_str()).ok_or("missing ukey")?.to_string();
    let username = v.get("username").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let uid = v.get("uid").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    Ok(StoredCredentials { username, uid, ukey })
}

pub fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn run_local_callback_server(listener: TcpListener, expected_state: String, tx: Sender<Result<StoredCredentials, String>>, website_base: String) {
    let _ = listener.set_nonblocking(true);
    let start = Instant::now();
    let timeout = Duration::from_secs(300);
    let mut last_poll = Instant::now();
    let mut polled_creds: Option<StoredCredentials> = None;
    let mut poll_success_time: Option<Instant> = None;
    loop {
        if start.elapsed() > timeout {
            if let Some(creds) = polled_creds.take() {
                let _ = tx.send(Ok(creds));
            } else {
                let _ = tx.send(Err("Authentication timed out (5 minutes)".to_string()));
            }
            break;
        }
        if let Some(t) = poll_success_time {
            if t.elapsed() > Duration::from_secs(10) {
                break;
            }
        }
        if polled_creds.is_none() && last_poll.elapsed() >= Duration::from_secs(2) {
            last_poll = Instant::now();
            if let Some(code) = poll_for_code(&expected_state, &website_base) {
                match exchange_code_with_base(&code, &website_base) {
                    Ok(creds) => {
                        let _ = tx.send(Ok(creds.clone()));
                        polled_creds = Some(creds);
                        poll_success_time = Some(Instant::now());
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                let mut buf = [0u8; 8192];
                let n = match stream.read(&mut buf) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                if n == 0 {
                    continue;
                }
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let first_line = req.lines().next().unwrap_or("");
                let parts: Vec<&str> = first_line.split_whitespace().collect();
                if parts.len() < 2 {
                    continue;
                }
                let method = parts[0];
                let path = parts[1];
                if method == "OPTIONS" {
                    let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                    continue;
                }
                if path.starts_with("/favicon.ico") {
                    let html = "<html><body>not found</body></html>";
                    let resp = format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nConnection: close\r\n\r\n{}",
                        html.as_bytes().len(),
                        html
                    );
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                    continue;
                }
                let query = path.split('?').nth(1).unwrap_or("");
                let code_opt = parse_query_param(query, "code");
                let state_opt = parse_query_param(query, "state");
                let error_opt = parse_query_param(query, "error");

                if let Some(err) = error_opt {
                    let html = format!("<html><body style=\"font-family:sans-serif;text-align:center;padding:40px;\"><h2>Authorization denied</h2><p>{}</p><p>You may close this window.</p></body></html>", err);
                    let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nConnection: close\r\n\r\n{}", html.as_bytes().len(), html);
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                    let _ = tx.send(Err(format!("Authorization denied: {err}")));
                    break;
                }

                if let Some(code) = code_opt {
                    let state = state_opt.unwrap_or_default();
                    if state != expected_state {
                        let html = "<html><body style=\"font-family:sans-serif;text-align:center;padding:40px;\"><h2>State mismatch</h2><p>Please try again from Studio.</p></body></html>";
                        let resp = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nConnection: close\r\n\r\n{}", html.as_bytes().len(), html);
                        let _ = stream.write_all(resp.as_bytes());
                        let _ = stream.flush();
                        let _ = tx.send(Err("State mismatch - please try again".to_string()));
                        break;
                    }
                    if let Some(creds) = polled_creds.clone() {
                        let html = format!("<html><head><meta charset=\"utf-8\"><title>VERTEXIA Studio</title></head><body style=\"font-family:sans-serif;text-align:center;padding:40px;background:#f5f5f8;\"><div style=\"max-width:480px;margin:0 auto;background:#fff;padding:32px;border-radius:12px;box-shadow:0 4px 24px rgba(0,0,0,0.08);\"><h2 style=\"color:#7423CB;margin:0 0 12px;\">Successfully authorized!</h2><p style=\"color:#444;\">You are now logged in as <b>{}</b>. You may close this window and return to VERTEXIA Studio.</p><p style=\"margin-top:20px;\"><a href=\"#\" onclick=\"window.close();return false;\" style=\"display:inline-block;background:#7423CB;color:#fff;padding:10px 22px;border-radius:6px;text-decoration:none;font-weight:700;\">Close window</a></p><script>setTimeout(function(){{try{{window.close()}}catch(e){{}}}},1200);</script></div></body></html>", creds.username);
                        let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nConnection: close\r\n\r\n{}", html.as_bytes().len(), html);
                        let _ = stream.write_all(resp.as_bytes());
                        let _ = stream.flush();
                        break;
                    }
                    let exchange_url = format!("{website_base}/api/v1/studio/exchange");
                    let result = (|| -> Result<StoredCredentials, String> {
                        let client = crate::common::net::api::blocking_client(Duration::from_secs(10))?;
                        let resp = client
                            .post(&exchange_url)
                            .json(&serde_json::json!({ "code": code }))
                            .send()
                            .map_err(|e| format!("exchange failed: {e}"))?;
                        let status = resp.status();
                        let body = resp.text().map_err(|e| e.to_string())?;
                        if !status.is_success() {
                            return Err(format!("exchange {status}: {body}"));
                        }
                        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
                        let ukey = v.get("ukey").and_then(|x| x.as_str()).ok_or("missing ukey")?.to_string();
                        let username = v.get("username").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let uid = v.get("uid").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
                        Ok(StoredCredentials { username, uid, ukey })
                    })();

                    match result {
                        Ok(creds) => {
                            let html = format!("<html><head><meta charset=\"utf-8\"><title>VERTEXIA Studio</title></head><body style=\"font-family:sans-serif;text-align:center;padding:40px;background:#f5f5f8;\"><div style=\"max-width:480px;margin:0 auto;background:#fff;padding:32px;border-radius:12px;box-shadow:0 4px 24px rgba(0,0,0,0.08);\"><h2 style=\"color:#7423CB;margin:0 0 12px;\">Successfully authorized!</h2><p style=\"color:#444;\">You are now logged in as <b>{}</b>. You may close this window and return to VERTEXIA Studio.</p><p style=\"margin-top:20px;\"><a href=\"#\" onclick=\"window.close();return false;\" style=\"display:inline-block;background:#7423CB;color:#fff;padding:10px 22px;border-radius:6px;text-decoration:none;font-weight:700;\">Close window</a></p><script>setTimeout(function(){{try{{window.close()}}catch(e){{}}}},1200);</script></div></body></html>", creds.username);
                            let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nConnection: close\r\n\r\n{}", html.as_bytes().len(), html);
                            let _ = stream.write_all(resp.as_bytes());
                            let _ = stream.flush();
                            let _ = tx.send(Ok(creds));
                            break;
                        }
                        Err(e) => {
                            let html = format!("<html><body style=\"font-family:sans-serif;text-align:center;padding:40px;\"><h2>Login failed</h2><p>{}</p></body></html>", e);
                            let resp = format!("HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/html\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nConnection: close\r\n\r\n{}", html.as_bytes().len(), html);
                            let _ = stream.write_all(resp.as_bytes());
                            let _ = stream.flush();
                            let _ = tx.send(Err(e));
                            break;
                        }
                    }
                } else {
                    let html = "<html><body style=\"font-family:sans-serif;text-align:center;padding:40px;\"><h2>Invalid request</h2><p>Missing code parameter.</p></body></html>";
                    let resp = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nConnection: close\r\n\r\n{}", html.as_bytes().len(), html);
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(80));
                continue;
            }
            Err(e) => {
                let _ = tx.send(Err(format!("listener error: {e}")));
                break;
            }
        }
    }
}

pub fn poll_studio_auth_flow(
    mut flow: ResMut<StudioAuthFlow>,
    mut store: ResMut<StudioAuthStore>,
    mut next_onboarding: ResMut<NextState<crate::studio::tools::OnboardingState>>,
    onboarding: Res<State<crate::studio::tools::OnboardingState>>,
) {
    if let Some(receiver) = flow.receiver.as_ref() {
        if let Ok(result) = receiver.try_recv() {
            match result {
                Ok(creds) => {
                    let _ = save_studio_credentials(&creds);
                    store.credentials = Some(creds);
                    flow.error = None;
                    flow.state = None;
                    flow.receiver = None;
                    flow.polling_state();
                    if *onboarding.get() == crate::studio::tools::OnboardingState::Login {
                        next_onboarding.set(crate::studio::tools::OnboardingState::Inactive);
                    }
                }
                Err(e) => {
                    flow.error = Some(e);
                    flow.receiver = None;
                    flow.state = None;
                    flow.is_polling = false;
                    flow.waiting_since = None;
                }
            }
            return;
        }
    }

    if flow.is_polling {
        if let Some(start) = flow.waiting_since {
            if start.elapsed() > Duration::from_secs(300) {
                flow.error = Some("Authentication timed out".to_string());
                flow.state = None;
                flow.receiver = None;
                flow.is_polling = false;
                flow.waiting_since = None;
                return;
            }
        }
    }

    if flow.state.is_none() && flow.receiver.is_none() {
        if let Some(creds) = load_studio_credentials() {
            if store.credentials.is_none() {
                store.credentials = Some(creds);
            }
        }
        if store.credentials.is_some() && *onboarding.get() == crate::studio::tools::OnboardingState::Login && flow.error.is_none() {
            //auto-skip login if already authenticated and not in error state
        }
    }
}

trait FlowExt {
    fn polling_state(&mut self);
}
impl FlowExt for StudioAuthFlow {
    fn polling_state(&mut self) {
        self.waiting_since = None;
        self.is_polling = false;
    }
}

pub fn check_uri_auth_on_update(
    mut store: ResMut<StudioAuthStore>,
    mut flow: ResMut<StudioAuthFlow>,
    mut next: ResMut<NextState<crate::studio::tools::OnboardingState>>,
    state: Res<State<crate::studio::tools::OnboardingState>>,
) {
    if let Some((code, _)) = try_extract_code_from_args() {
        if store.credentials.is_none() {
            if let Ok(creds) = exchange_code(&code) {
                let _ = save_studio_credentials(&creds);
                store.credentials = Some(creds);
                flow.error = None;
                if *state.get() == crate::studio::tools::OnboardingState::Login {
                    next.set(crate::studio::tools::OnboardingState::Inactive);
                }
            }
        }
    }
}
