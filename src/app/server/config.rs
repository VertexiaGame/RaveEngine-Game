use std::net::{IpAddr, Ipv4Addr};

pub struct ServerAppConfig {
    pub port: u16,
    pub map_path: String,
    pub bind_addr: IpAddr,
    pub netcode_key: [u8; 32],
    pub protocol_id: u64,
    pub allow_unauthenticated: bool,
}

impl ServerAppConfig {
    pub fn from_env_and_args() -> Self {
        let mut port = 5000; //default
        let mut map_path = "assets/maps/default.vrtx".to_string(); //Defalt
        let mut bind_addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let mut netcode_key_cli: Option<String> = None;
        let mut protocol_id_cli: Option<u64> = None;
        let mut allow_unauthenticated = false;

        let args: Vec<String> = std::env::args().collect();
        for i in 0..args.len() {
            if args[i] == "--port" && i + 1 < args.len() {
                if let Ok(p) = args[i + 1].parse::<u16>() {
                    port = p;
                }
            }
            if args[i] == "--map" && i + 1 < args.len() {
                map_path = args[i + 1].clone();
            }
            if args[i] == "--bind" && i + 1 < args.len() {
                if let Ok(ip) = args[i + 1].parse::<IpAddr>() {
                    bind_addr = ip;
                }
            }
            if args[i] == "--netcode-key" && i + 1 < args.len() {
                netcode_key_cli = Some(args[i + 1].clone());
            }
            if args[i] == "--protocol-id" && i + 1 < args.len() {
                if let Ok(id) = args[i + 1].parse::<u64>() {
                    protocol_id_cli = Some(id);
                }
            }
            if args[i] == "--allow-unauthenticated" {
                allow_unauthenticated = true;
            }
        }

        Self {
            port,
            map_path,
            bind_addr,
            netcode_key: crate::common::net::netcode::netcode_private_key(netcode_key_cli.as_deref()),
            protocol_id: crate::common::net::netcode::netcode_protocol_id(protocol_id_cli),
            allow_unauthenticated,
        }
    }
}