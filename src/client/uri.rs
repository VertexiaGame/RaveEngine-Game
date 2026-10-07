use std::net::IpAddr;

pub const CLIENT_URI_SCHEME: &str = "vertexia-client";
pub const CLIENT_URI_FALLBACK_SCHEME: &str = "vertexia";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientJoinInfo {
    pub ip: IpAddr,
    pub port: u16,
    pub ukey: String,
}

pub fn build_client_join_uri(host: &str, port: u16, ukey: &str) -> String {
    format!(
        "{}://play?ip={}&port={}&ukey={}",
        CLIENT_URI_SCHEME,
        uri_encode(host),
        port,
        uri_encode(ukey)
    )
}

pub fn parse_client_join_uri(raw: &str) -> Option<ClientJoinInfo> {
    let arg = raw
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .trim();
    if arg.is_empty() {
        return None;
    }
    let lower = arg.to_lowercase();
    let rest = if lower.starts_with(&format!("{CLIENT_URI_SCHEME}://")) {
        &arg[CLIENT_URI_SCHEME.len() + 3..]
    } else if lower.starts_with(&format!("{CLIENT_URI_FALLBACK_SCHEME}://")) {
        &arg[CLIENT_URI_FALLBACK_SCHEME.len() + 3..]
    } else {
        return None;
    };
    let (path, query) = rest.split_once('?')?;
    let action = path.trim().trim_matches('/').to_lowercase();
    if action != "play" && action != "join" {
        return None;
    }
    let host = query_param(query, "ip").or_else(|| query_param(query, "host"))?;
    let port_str = query_param(query, "port")?;
    let ukey = query_param(query, "ukey")?;
    if host.is_empty() || ukey.is_empty() {
        return None;
    }
    let port: u16 = port_str.parse().ok()?;
    let ip: IpAddr = host.parse().or_else(|_| resolve_host(&host, port)).ok()?;
    Some(ClientJoinInfo { ip, port, ukey })
}

pub fn extract_client_join_from_args(args: &[String]) -> Option<ClientJoinInfo> {
    args.iter().skip(1).find_map(|a| parse_client_join_uri(a))
}

#[cfg(target_os = "windows")]
pub fn register_client_uri_scheme() -> Result<(), String> {
    use std::process::Command;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.display().to_string();
    let ps_script = format!(
        "$p='HKCU:\\Software\\Classes\\vertexia-client'; if(!(Test-Path $p)){{New-Item -Path $p -Force | Out-Null}}; Set-ItemProperty -Path $p -Name '(Default)' -Value 'URL:VERTEXIA Client'; Set-ItemProperty -Path $p -Name 'URL Protocol' -Value ''; $c=\"$p\\shell\\open\\command\"; if(!(Test-Path $c)){{New-Item -Path $c -Force | Out-Null}}; Set-ItemProperty -Path $c -Name '(Default)' -Value '\"{exe_str}\" \"%1\"'",
    );
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output();
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn register_client_uri_scheme() -> Result<(), String> {
    Ok(())
}

fn resolve_host(host: &str, port: u16) -> Result<IpAddr, ()> {
    use std::net::ToSocketAddrs;
    format!("{host}:{port}")
        .to_socket_addrs()
        .map_err(|_| ())?
        .find_map(|a| Some(a.ip()))
        .ok_or(())
}

fn query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let mut iter = pair.splitn(2, '=');
        let k = iter.next()?;
        let v = iter.next().unwrap_or("");
        if k.eq_ignore_ascii_case(key) {
            return Some(uri_decode(v));
        }
    }
    None
}

pub fn uri_encode(s: &str) -> String {
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

fn uri_decode(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_client_play_uri() {
        let info = parse_client_join_uri(
            "vertexia-client://play?ip=127.0.0.1&port=5000&ukey=abc123",
        )
        .unwrap();
        assert_eq!(info.ip.to_string(), "127.0.0.1");
        assert_eq!(info.port, 5000);
        assert_eq!(info.ukey, "abc123");
    }

    #[test]
    fn parses_quoted_uri_from_registry() {
        let info = parse_client_join_uri(
            "\"vertexia-client://play?ip=127.0.0.1&port=5000&ukey=abc123\"",
        )
        .unwrap();
        assert_eq!(info.port, 5000);
        assert_eq!(info.ukey, "abc123");
    }

    #[test]
    fn accepts_join_action_and_fallback_scheme() {
        let info = parse_client_join_uri("vertexia://join?ip=10.0.0.2&port=6000&ukey=k1")
            .unwrap();
        assert_eq!(info.ip.to_string(), "10.0.0.2");
        assert_eq!(info.port, 6000);
    }

    #[test]
    fn rejects_studio_auth_uris() {
        assert!(parse_client_join_uri("vertexia-studio://auth?code=abc&state=xyz").is_none());
        assert!(parse_client_join_uri("vertexia://auth?code=abc").is_none());
        assert!(parse_client_join_uri("vertexia-client://auth?code=abc").is_none());
    }

    #[test]
    fn rejects_incomplete_uris() {
        assert!(parse_client_join_uri("vertexia-client://play?ip=127.0.0.1&port=5000").is_none());
        assert!(
            parse_client_join_uri("vertexia-client://play?port=5000&ukey=k").is_none()
        );
        assert!(
            parse_client_join_uri("vertexia-client://play?ip=999.1.1.1&port=5000&ukey=k")
                .is_none()
        );
        assert!(
            parse_client_join_uri("vertexia-client://play?ip=127.0.0.1&port=abc&ukey=k")
                .is_none()
        );
        assert!(parse_client_join_uri("https://example.com/play").is_none());
    }

    #[test]
    fn extracts_join_from_arg_list() {
        let args = vec![
            "RaveEngineClient.exe".to_string(),
            "vertexia-client://play?ip=127.0.0.1&port=5001&ukey=k2".to_string(),
        ];
        let info = extract_client_join_from_args(&args).unwrap();
        assert_eq!(info.port, 5001);
        assert_eq!(info.ukey, "k2");
    }

    #[test]
    fn ignores_arg_list_without_uri() {
        let args = vec![
            "RaveEngineClient.exe".to_string(),
            "--ip".to_string(),
            "127.0.0.1".to_string(),
        ];
        assert!(extract_client_join_from_args(&args).is_none());
    }

    #[test]
    fn build_parse_round_trip() {
        let uri = build_client_join_uri("127.0.0.1", 5000, "abc123");
        let info = parse_client_join_uri(&uri).unwrap();
        assert_eq!(info.ip.to_string(), "127.0.0.1");
        assert_eq!(info.port, 5000);
        assert_eq!(info.ukey, "abc123");
    }
}
