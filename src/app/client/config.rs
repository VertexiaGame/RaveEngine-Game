use std::net::IpAddr;

pub struct ClientAppConfig {
    pub ip: IpAddr,
    pub port: u16,
    pub ukey: String,
    pub netcode_key: Option<[u8; 32]>,
    pub protocol_id: Option<u64>,
}

impl ClientAppConfig {
    pub fn from_env_and_args() -> Self {
        let mut ip = IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
        let mut port = 5000;
        let mut ukey = "".to_string();
        let mut netcode_key_cli: Option<String> = None;
        let mut protocol_id_cli: Option<u64> = None;

        if let Ok(env_ukey) = std::env::var("VERTIGO_CLIENT_UKEY") {
            ukey = env_ukey;
        }
        if let Ok(ip_str) = std::env::var("VERTIGO_SERVER_IP") {
            if let Ok(parsed_ip) = ip_str.parse::<IpAddr>() {
                ip = parsed_ip;
            }
        }
        if let Ok(port_str) = std::env::var("VERTIGO_SERVER_PORT") {
            if let Ok(parsed_port) = port_str.parse::<u16>() {
                port = parsed_port;
            }
        }

        let args: Vec<String> = std::env::args().collect();
        for i in 0..args.len() {
            if args[i] == "--port" && i + 1 < args.len() {
                if let Ok(p) = args[i + 1].parse::<u16>() {
                    port = p;
                }
            }
            if args[i] == "--ip" && i + 1 < args.len() {
                if let Ok(ip_addr) = args[i + 1].parse::<IpAddr>() {
                    ip = ip_addr;
                }
            }
            if args[i] == "--ukey" && i + 1 < args.len() {
                ukey = args[i + 1].clone();
            }
            if args[i] == "--netcode-key" && i + 1 < args.len() {
                netcode_key_cli = Some(args[i + 1].clone());
            }
            if args[i] == "--protocol-id" && i + 1 < args.len() {
                if let Ok(id) = args[i + 1].parse::<u64>() {
                    protocol_id_cli = Some(id);
                }
            }
        }

        let netcode_key = netcode_key_cli
            .as_deref()
            .and_then(crate::common::net::netcode::parse_hex_key)
            .or_else(|| {
                std::env::var("NETCODE_PRIVATE_KEY")
                    .ok()
                    .as_deref()
                    .and_then(crate::common::net::netcode::parse_hex_key)
            });
        let protocol_id = protocol_id_cli.or_else(|| {
            std::env::var("NETCODE_PROTOCOL_ID")
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
        });

        Self {
            ip,
            port,
            ukey,
            netcode_key,
            protocol_id,
        }
    }
}