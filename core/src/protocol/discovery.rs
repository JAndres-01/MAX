use crate::protocol::client::create_scan_client;
use crate::protocol::{DeviceDto, DeviceType, PeerDevice};
use crate::state::AppState;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tracing::{info, warn};

pub const MULTICAST_IPV4: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 167);
pub const MULTICAST_PORT: u16 = 53317;

pub fn is_own_ip(ip: &str) -> bool {
    if ip == "127.0.0.1" || ip == "::1" || ip == "0.0.0.0" {
        return true;
    }
    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        for (_name, addr) in interfaces {
            if addr.to_string() == ip {
                return true;
            }
        }
    }
    false
}

fn get_active_arp_neighbors() -> Vec<String> {
    let mut ips = HashSet::new();
    if let Ok(content) = std::fs::read_to_string("/proc/net/arp") {
        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let ip = parts[0];
                let flags = parts[2];
                let mac = parts[3];
                if flags != "0x0" && mac != "00:00:00:00:00:00" && !is_own_ip(ip) {
                    ips.insert(ip.to_string());
                }
            }
        }
    }
    ips.into_iter().collect()
}

pub async fn start_discovery(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let multicast_addr = SocketAddrV4::new(MULTICAST_IPV4, MULTICAST_PORT);

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_broadcast(true)?;
    let _ = socket.set_multicast_loop_v4(true);
    let _ = socket.set_multicast_ttl_v4(16);
    socket.set_nonblocking(true)?;

    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, MULTICAST_PORT);
    socket.bind(&bind_addr.into())?;
    let _ = socket.join_multicast_v4(&MULTICAST_IPV4, &Ipv4Addr::UNSPECIFIED);

    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        for (_name, ip) in interfaces {
            if let std::net::IpAddr::V4(ipv4) = ip {
                if !ipv4.is_loopback() && !ipv4.is_link_local() {
                    let _ = socket.join_multicast_v4(&MULTICAST_IPV4, &ipv4);
                }
            }
        }
    }

    let std_socket: std::net::UdpSocket = socket.into();
    let udp_socket = Arc::new(UdpSocket::from_std(std_socket)?);

    info!(
        "LocalSend Discovery running on {}:{} (Multicast, Unicast ARP & Subnet Scan)",
        MULTICAST_IPV4, MULTICAST_PORT
    );

    let listener_socket = Arc::clone(&udp_socket);
    let listener_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut buf = [0u8; 65535];
        loop {
            match listener_socket.recv_from(&mut buf).await {
                Ok((len, remote_addr)) => {
                    let remote_ip = match remote_addr {
                        SocketAddr::V4(v4) => v4.ip().to_string(),
                        SocketAddr::V6(v6) => v6.ip().to_string(),
                    };

                    if is_own_ip(&remote_ip) {
                        continue;
                    }

                    if let Ok(text) = std::str::from_utf8(&buf[..len]) {
                        if let Ok(dto) = serde_json::from_str::<DeviceDto>(text) {
                            let my_fingerprint = {
                                let cfg = listener_state.config.read().await;
                                cfg.fingerprint.clone()
                            };

                            if dto.fingerprint != my_fingerprint {
                                let peer = PeerDevice {
                                    ip: remote_ip.clone(),
                                    port: dto.port,
                                    alias: dto.alias.clone(),
                                    device_model: dto.device_model.clone(),
                                    device_type: dto.device_type.clone(),
                                    fingerprint: dto.fingerprint.clone(),
                                    protocol: dto.protocol.clone(),
                                    download: dto.download,
                                    last_seen: 0,
                                };
                                listener_state.add_or_update_peer(peer).await;

                                if dto.announcement == Some(true) || dto.announce == Some(true) {
                                    let resp_dto = {
                                        let cfg = listener_state.config.read().await;
                                        DeviceDto {
                                            alias: cfg.alias.clone(),
                                            version: "2.1".to_string(),
                                            device_model: Some(cfg.device_model.clone()),
                                            device_type: DeviceType::Desktop,
                                            fingerprint: cfg.fingerprint.clone(),
                                            port: cfg.port,
                                            protocol: cfg.protocol.clone(),
                                            download: true,
                                            announcement: Some(false),
                                            announce: Some(false),
                                        }
                                    };
                                    if let Ok(resp_json) = serde_json::to_vec(&resp_dto) {
                                        let target = SocketAddr::new(remote_addr.ip(), dto.port);
                                        let _ = listener_socket.send_to(&resp_json, target).await;
                                    }

                                    let sender_ip = remote_ip.clone();
                                    let sender_port = dto.port;
                                    let sender_proto = dto.protocol.clone();
                                    let reg_client = create_scan_client(&sender_proto);
                                    let reg_dto = resp_dto.clone();
                                    tokio::spawn(async move {
                                        let url = format!(
                                            "{}://{}:{}/api/localsend/v2/register",
                                            sender_proto, sender_ip, sender_port
                                        );
                                        let _ = reg_client.post(&url).json(&reg_dto).send().await;
                                    });
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("UDP discovery error: {}", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    let beacon_socket = Arc::clone(&udp_socket);
    let beacon_state = Arc::clone(&state);
    let https_client = create_scan_client("https");
    let http_client = create_scan_client("http");

    tokio::spawn(async move {
        let dest_multicast = SocketAddr::V4(multicast_addr);
        let dest_broadcast = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::BROADCAST), MULTICAST_PORT);

        let mut loop_counter: u64 = 0;
        loop {
            let active_neighbors = get_active_arp_neighbors();

            {
                let cfg = beacon_state.config.read().await;
                if cfg.discovery_enabled {
                    let dto = DeviceDto {
                        alias: cfg.alias.clone(),
                        version: "2.1".to_string(),
                        device_model: Some(cfg.device_model.clone()),
                        device_type: DeviceType::Desktop,
                        fingerprint: cfg.fingerprint.clone(),
                        port: cfg.port,
                        protocol: cfg.protocol.clone(),
                        download: true,
                        announcement: Some(true),
                        announce: Some(true),
                    };

                    if let Ok(bytes) = serde_json::to_vec(&dto) {
                        let _ = beacon_socket.send_to(&bytes, dest_multicast).await;
                        let _ = beacon_socket.send_to(&bytes, dest_broadcast).await;

                        if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
                            for (_name, ip) in interfaces {
                                if let std::net::IpAddr::V4(ipv4) = ip {
                                    if !ipv4.is_loopback() && !ipv4.is_link_local() {
                                        let octets = ipv4.octets();
                                        let subnet_broadcast = Ipv4Addr::new(octets[0], octets[1], octets[2], 255);
                                        let target = SocketAddr::new(std::net::IpAddr::V4(subnet_broadcast), MULTICAST_PORT);
                                        let _ = beacon_socket.send_to(&bytes, target).await;
                                    }
                                }
                            }
                        }

                        for neighbor_ip in &active_neighbors {
                            if let Ok(addr) = format!("{}:{}", neighbor_ip, MULTICAST_PORT).parse::<SocketAddr>() {
                                let _ = beacon_socket.send_to(&bytes, addr).await;
                            }
                            // Directly probe active neighbors over HTTP/HTTPS for instantaneous discovery
                            let state_probe = Arc::clone(&beacon_state);
                            let ip_probe = neighbor_ip.clone();
                            tokio::spawn(async move {
                                probe_device_at_address(state_probe, &ip_probe, MULTICAST_PORT).await;
                            });
                        }
                    }
                }
            }

            loop_counter += 1;
            // Run subnet scan gently in the background on startup (cycle 1) and every 30 seconds (15 cycles)
            if loop_counter == 1 || loop_counter % 15 == 0 {
                let state_scan = Arc::clone(&beacon_state);
                let https_scan = https_client.clone();
                let http_scan = http_client.clone();
                tokio::spawn(async move {
                    scan_subnet_for_peers(state_scan, https_scan, http_scan).await;
                });
            }

            let active_peers: Vec<PeerDevice> = {
                let peers = beacon_state.peers.read().await;
                peers.values().cloned().collect()
            };
            for peer in active_peers {
                let state_clone = Arc::clone(&beacon_state);
                let client = create_scan_client(&peer.protocol);
                let my_dto_clone = {
                    let cfg = state_clone.config.read().await;
                    DeviceDto {
                        alias: cfg.alias.clone(),
                        version: "2.1".to_string(),
                        device_model: Some(cfg.device_model.clone()),
                        device_type: DeviceType::Desktop,
                        fingerprint: cfg.fingerprint.clone(),
                        port: cfg.port,
                        protocol: cfg.protocol.clone(),
                        download: true,
                        announcement: Some(false),
                        announce: Some(false),
                    }
                };
                tokio::spawn(async move {
                    let url = format!("{}://{}:{}/api/localsend/v2/register", peer.protocol, peer.ip, peer.port);
                    let mut probe_ok = false;
                    if let Ok(resp) = client.post(&url).json(&my_dto_clone).send().await {
                        if resp.status().is_success() {
                            if let Ok(peer_dto) = resp.json::<DeviceDto>().await {
                                let updated_peer = PeerDevice {
                                    ip: peer.ip,
                                    port: peer_dto.port,
                                    alias: peer_dto.alias,
                                    device_model: peer_dto.device_model,
                                    device_type: peer_dto.device_type,
                                    fingerprint: peer_dto.fingerprint,
                                    protocol: peer.protocol,
                                    download: peer_dto.download,
                                    last_seen: 0,
                                };
                                state_clone.add_or_update_peer(updated_peer).await;
                                probe_ok = true;
                            }
                        }
                    }
                    if !probe_ok {
                        state_clone.record_peer_probe_failure(&peer.fingerprint).await;
                    }
                });
            }

            beacon_state.clean_stale_peers(6).await;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    Ok(())
}

pub async fn probe_device_at_address(
    state: Arc<AppState>,
    ip: &str,
    port: u16,
) {
    if is_own_ip(ip) || ip == "127.0.0.1" || ip == "::1" {
        return;
    }

    let my_dto = {
        let cfg = state.config.read().await;
        DeviceDto {
            alias: cfg.alias.clone(),
            version: "2.1".to_string(),
            device_model: Some(cfg.device_model.clone()),
            device_type: DeviceType::Desktop,
            fingerprint: cfg.fingerprint.clone(),
            port: cfg.port,
            protocol: cfg.protocol.clone(),
            download: true,
            announcement: Some(false),
            announce: Some(false),
        }
    };

    let https_client = create_scan_client("https");
    let https_url = format!("https://{}:{}/api/localsend/v2/register", ip, port);
    if let Ok(resp) = https_client.post(&https_url).json(&my_dto).send().await {
        if resp.status().is_success() {
            if let Ok(peer_dto) = resp.json::<DeviceDto>().await {
                if peer_dto.fingerprint != my_dto.fingerprint {
                    let peer = PeerDevice {
                        ip: ip.to_string(),
                        port: peer_dto.port,
                        alias: peer_dto.alias.clone(),
                        device_model: peer_dto.device_model.clone(),
                        device_type: peer_dto.device_type.clone(),
                        fingerprint: peer_dto.fingerprint.clone(),
                        protocol: "https".to_string(),
                        download: peer_dto.download,
                        last_seen: 0,
                    };
                    state.add_or_update_peer(peer).await;
                    return;
                }
            }
        }
    }

    let http_client = create_scan_client("http");
    let http_url = format!("http://{}:{}/api/localsend/v2/register", ip, port);
    if let Ok(resp) = http_client.post(&http_url).json(&my_dto).send().await {
        if resp.status().is_success() {
            if let Ok(peer_dto) = resp.json::<DeviceDto>().await {
                if peer_dto.fingerprint != my_dto.fingerprint {
                    let peer = PeerDevice {
                        ip: ip.to_string(),
                        port: peer_dto.port,
                        alias: peer_dto.alias.clone(),
                        device_model: peer_dto.device_model.clone(),
                        device_type: peer_dto.device_type.clone(),
                        fingerprint: peer_dto.fingerprint.clone(),
                        protocol: "http".to_string(),
                        download: peer_dto.download,
                        last_seen: 0,
                    };
                    state.add_or_update_peer(peer).await;
                }
            }
        }
    }
}

async fn scan_subnet_for_peers(
    state: Arc<AppState>,
    https_client: reqwest::Client,
    http_client: reqwest::Client,
) {
    let interfaces = match local_ip_address::list_afinet_netifas() {
        Ok(ifas) => ifas,
        Err(_) => return,
    };

    let mut target_ips = Vec::new();

    for (_name, ip) in interfaces {
        if let std::net::IpAddr::V4(ipv4) = ip {
            if !ipv4.is_loopback() && !ipv4.is_link_local() {
                let octets = ipv4.octets();
                for last_octet in 1..=254 {
                    if last_octet != octets[3] {
                        target_ips.push(format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], last_octet));
                    }
                }
            }
        }
    }

    if target_ips.is_empty() {
        return;
    }

    let my_dto = {
        let cfg = state.config.read().await;
        DeviceDto {
            alias: cfg.alias.clone(),
            version: "2.1".to_string(),
            device_model: Some(cfg.device_model.clone()),
            device_type: DeviceType::Desktop,
            fingerprint: cfg.fingerprint.clone(),
            port: cfg.port,
            protocol: cfg.protocol.clone(),
            download: true,
            announcement: Some(true),
            announce: Some(true),
        }
    };

    // Scan gently in chunks of 16 to avoid socket exhaustion or triggering router SYN protection
    for chunk in target_ips.chunks(16) {
        let mut tasks = Vec::new();
        for ip in chunk {
            let https_c = https_client.clone();
            let http_c = http_client.clone();
            let state_clone = Arc::clone(&state);
            let dto_clone = my_dto.clone();
            let ip_clone = ip.clone();

            tasks.push(tokio::spawn(async move {
                let https_url = format!("https://{}:53317/api/localsend/v2/register", ip_clone);
                if let Ok(resp) = https_c.post(&https_url).json(&dto_clone).send().await {
                    if resp.status().is_success() {
                        if let Ok(peer_dto) = resp.json::<DeviceDto>().await {
                            if peer_dto.fingerprint != dto_clone.fingerprint {
                                let peer = PeerDevice {
                                    ip: ip_clone.clone(),
                                    port: peer_dto.port,
                                    alias: peer_dto.alias.clone(),
                                    device_model: peer_dto.device_model.clone(),
                                    device_type: peer_dto.device_type.clone(),
                                    fingerprint: peer_dto.fingerprint.clone(),
                                    protocol: "https".to_string(),
                                    download: peer_dto.download,
                                    last_seen: 0,
                                };
                                state_clone.add_or_update_peer(peer).await;
                                return;
                            }
                        }
                    }
                }

                let http_url = format!("http://{}:53317/api/localsend/v2/register", ip_clone);
                if let Ok(resp) = http_c.post(&http_url).json(&dto_clone).send().await {
                    if resp.status().is_success() {
                        if let Ok(peer_dto) = resp.json::<DeviceDto>().await {
                            if peer_dto.fingerprint != dto_clone.fingerprint {
                                let peer = PeerDevice {
                                    ip: ip_clone,
                                    port: peer_dto.port,
                                    alias: peer_dto.alias.clone(),
                                    device_model: peer_dto.device_model.clone(),
                                    device_type: peer_dto.device_type.clone(),
                                    fingerprint: peer_dto.fingerprint.clone(),
                                    protocol: "http".to_string(),
                                    download: peer_dto.download,
                                    last_seen: 0,
                                };
                                state_clone.add_or_update_peer(peer).await;
                            }
                        }
                    }
                }
            }));
        }
        let _ = futures_util::future::join_all(tasks).await;
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}
