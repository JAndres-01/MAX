use crate::protocol::client::create_scan_client;
use crate::protocol::{DeviceDto, DeviceType, PeerDevice};
use crate::state::{AppState, IpcEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info};

/// Autonomous Continuity Autolink Watcher with Intelligent Fallback Hotspot.
///
/// 1. Home / Known Wi-Fi Mode (Default Priority):
///    If the PC finds a known Wi-Fi (Home, Work, etc.), it connects normally.
///    The hotspot stays OFF. Both PC and iPhone communicate over LAN with 0 disruptions.
///
/// 2. Offline / University Mode (Fallback):
///    If the PC is disconnected from any Wi-Fi for 15+ seconds (meaning no known router exists in range),
///    it automatically starts the local high-speed "Hyprand" hotspot (10.42.0.1)
///    so the iPhone connects automatically and can transfer photos & clipboard at full speed.
pub fn spawn_autolink_watcher(state: Arc<AppState>) {
    tokio::spawn(async move {
        info!("Synapse Auto-Link background service started");
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let https_client = create_scan_client("https");
        let http_client = create_scan_client("http");
        let mut disconnected_ticks: u32 = 0;

        loop {
            interval.tick().await;

            // 1. Check Wi-Fi state for intelligent fallback hotspot
            #[cfg(target_os = "linux")]
            if let Ok(output) = Command::new("nmcli")
                .args(["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device"])
                .output()
                .await
            {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 3 && parts[0] == "wlan0" {
                        let state_str = parts[2];
                        let conn_name = parts.get(3).copied().unwrap_or("");

                        if state_str == "connected" || state_str == "conectado" {
                            if conn_name != "Hyprand" {
                                // Connected to home/work Wi-Fi! Reset counter
                                disconnected_ticks = 0;
                            }
                        } else if state_str == "disconnected" || state_str == "desconectado" {
                            disconnected_ticks += 1;
                            // If disconnected for ~15 seconds (3 ticks), activate Hyprand hotspot fallback
                            if disconnected_ticks == 3 {
                                info!("No known Wi-Fi network detected in range. Silently activating 'Hyprand' hotspot fallback...");
                                let _ = Command::new("nmcli")
                                    .args(["connection", "up", "Hyprand"])
                                    .output()
                                    .await;
                            }
                        }
                    }
                }
            }

            #[cfg(not(target_os = "linux"))]
            let _ = disconnected_ticks;

            // 2. Discover and probe active subnets
            let mut probe_ips: Vec<String> = Vec::new();

            if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
                for (_name, ip) in interfaces {
                    if let std::net::IpAddr::V4(v4) = ip {
                        let octets = v4.octets();
                        // A. Local PC Hotspot "Hyprand" (10.42.0.1/24) -> probe connected iPhone clients (2..60)
                        if octets[0] == 10 && octets[1] == 42 && octets[2] == 0 {
                            for host in 2..=60 {
                                probe_ips.push(format!("10.42.0.{}", host));
                            }
                        }
                        // B. iOS Personal Hotspot (172.20.10.1)
                        if octets[0] == 172 && octets[1] == 20 && octets[2] == 10 {
                            probe_ips.push("172.20.10.1".to_string());
                        }
                    }
                }
            }

            for target_ip in probe_ips {
                let target_ip_clone = target_ip.clone();
                let state_clone = Arc::clone(&state);
                let https_c = https_client.clone();
                let http_c = http_client.clone();

                tokio::spawn(async move {
                    for (client, proto) in [(&https_c, "https"), (&http_c, "http")] {
                        let url = format!("{}://{}:53317/api/localsend/v2/info", proto, target_ip_clone);
                        if let Ok(resp) = client.get(&url).send().await {
                            if let Ok(dto) = resp.json::<DeviceDto>().await {
                                let peer = PeerDevice {
                                    ip: target_ip_clone.clone(),
                                    port: dto.port,
                                    alias: dto.alias.clone(),
                                    device_model: dto.device_model.clone(),
                                    device_type: dto.device_type.clone(),
                                    fingerprint: dto.fingerprint.clone(),
                                    protocol: dto.protocol.clone(),
                                    download: dto.download,
                                    last_seen: 0,
                                };
                                state_clone.add_or_update_peer(peer).await;

                                // Register ourselves to the target
                                let reg_dto = {
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
                                let reg_url = format!("{}://{}:53317/api/localsend/v2/register", proto, target_ip_clone);
                                let _ = client.post(&reg_url).json(&reg_dto).send().await;
                                break;
                            }
                        }
                    }
                });
            }

            // 3. Update autolink connection status
            let peers = state.peers.read().await;
            let has_iphone = peers.values().any(|p| {
                let name = p.alias.to_lowercase();
                name.contains("iphone")
                    || name.contains("ipad")
                    || name.contains("apple")
                    || p.device_model.as_deref().unwrap_or("").to_lowercase().contains("iphone")
            });
            let iphone_peer = peers.values().find(|p| {
                let name = p.alias.to_lowercase();
                name.contains("iphone")
                    || name.contains("ipad")
                    || name.contains("apple")
                    || p.device_model.as_deref().unwrap_or("").to_lowercase().contains("iphone")
            }).cloned();
            drop(peers);

            let mut autolink = state.autolink_connected.write().await;
            if *autolink != has_iphone {
                *autolink = has_iphone;
                let _ = state.event_tx.send(IpcEvent::AutolinkStatus {
                    connected: has_iphone,
                    ip: iphone_peer.map(|p| p.ip),
                    device_name: "iPhone".to_string(),
                });
            }
        }
    });
}
