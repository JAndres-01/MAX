use crate::state::AppState;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tracing::info;

/// Spawns an autonomous mDNS publisher that guarantees `hyprand.local` and `synapsedrop.local`
/// always dynamically resolve to the PC's active IP across ANY Wi-Fi, Ethernet, or Hotspot network.
pub fn spawn_mdns_publisher(state: Arc<AppState>) {
    #[cfg(target_os = "linux")]
    tokio::spawn(async move {
        info!("Synapse mDNS Publisher started (hyprand.local, synapsedrop.local & _localsend._tcp)");
        let mut last_published_ip = String::new();
        let mut last_published_alias = String::new();
        let mut last_published_port: u16 = 0;
        let mut active_child1: Option<tokio::process::Child> = None;
        let mut active_child2: Option<tokio::process::Child> = None;
        let mut active_child3: Option<tokio::process::Child> = None;
        let mut active_child4: Option<tokio::process::Child> = None;

        loop {
            // Find current primary non-loopback IPv4
            let mut current_ip = String::new();
            if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
                for (name, ip) in interfaces {
                    if let std::net::IpAddr::V4(v4) = ip {
                        if !v4.is_loopback() && !v4.is_link_local() {
                            let s = v4.to_string();
                            // Prefer wlan0 or active Wi-Fi / Hotspot / Ethernet
                            if name.starts_with("wlan") || name.starts_with("wlp") || name.starts_with("eth") || name.starts_with("enp") {
                                current_ip = s;
                                break;
                            } else if current_ip.is_empty() {
                                current_ip = s;
                            }
                        }
                    }
                }
            }

            let (alias, port) = {
                let cfg = state.config.read().await;
                (cfg.alias.clone(), cfg.port)
            };

            let ip_changed = !current_ip.is_empty() && current_ip != last_published_ip;
            let config_changed = !alias.is_empty() && (alias != last_published_alias || port != last_published_port);
            
            // Check if published processes exited unexpectedly
            let child1_dead = match &mut active_child1 {
                Some(c) => c.try_wait().ok().flatten().is_some(),
                None => false,
            };
            let child2_dead = match &mut active_child2 {
                Some(c) => c.try_wait().ok().flatten().is_some(),
                None => false,
            };
            let child3_dead = match &mut active_child3 {
                Some(c) => c.try_wait().ok().flatten().is_some(),
                None => false,
            };
            let child4_dead = match &mut active_child4 {
                Some(c) => c.try_wait().ok().flatten().is_some(),
                None => false,
            };

            if !current_ip.is_empty() && (ip_changed || config_changed || child1_dead || child2_dead || child3_dead || child4_dead) {
                if ip_changed || config_changed {
                    info!("Publishing mDNS: hyprand.local, synapsedrop.local & _localsend._tcp ({}:{}) -> {}", alias, port, current_ip);
                }

                // Terminate previous publisher processes cleanly
                if let Some(mut child) = active_child1.take() {
                    let _ = child.kill().await;
                }
                if let Some(mut child2) = active_child2.take() {
                    let _ = child2.kill().await;
                }
                if let Some(mut child3) = active_child3.take() {
                    let _ = child3.kill().await;
                }
                if let Some(mut child4) = active_child4.take() {
                    let _ = child4.kill().await;
                }

                // Clean up any orphaned avahi-publish processes from prior runs
                let _ = Command::new("killall").arg("avahi-publish").status().await;

                // Brief grace period for Avahi DBus to unregister previous entries
                tokio::time::sleep(Duration::from_millis(250)).await;

                // 1. Publish hyprand.local with explicit -a (address) and -R (no reverse collision)
                let child1 = Command::new("avahi-publish")
                    .args(["-a", "-f", "-R", "hyprand.local", &current_ip])
                    .kill_on_drop(true)
                    .spawn();

                if let Ok(c) = child1 {
                    active_child1 = Some(c);
                }

                // 2. Publish synapsedrop.local
                let child2 = Command::new("avahi-publish")
                    .args(["-a", "-f", "-R", "synapsedrop.local", &current_ip])
                    .kill_on_drop(true)
                    .spawn();

                if let Ok(c) = child2 {
                    active_child2 = Some(c);
                }

                // 3. Publish _localsend._tcp service on port 53317 (Standard LocalSend / AirDrop discovery)
                let child3 = Command::new("avahi-publish")
                    .args(["-s", "-f", &alias, "_localsend._tcp", &port.to_string()])
                    .kill_on_drop(true)
                    .spawn();

                if let Ok(c) = child3 {
                    active_child3 = Some(c);
                }

                // 4. Publish _synapsedrop._tcp service on port 53318 (HTTP companion server)
                let companion_port = port + 1;
                let child4 = Command::new("avahi-publish")
                    .args(["-s", "-f", &alias, "_synapsedrop._tcp", &companion_port.to_string()])
                    .kill_on_drop(true)
                    .spawn();

                if let Ok(c) = child4 {
                    active_child4 = Some(c);
                }

                last_published_ip = current_ip;
                last_published_alias = alias;
                last_published_port = port;
            }

            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });
    #[cfg(not(target_os = "linux"))]
    {
        // On Windows and other OSes, discovery is handled natively via UDP multicast (224.0.0.167:53317).
        let _ = state;
    }
}

/// Spawns an autonomous mDNS browser on Linux that listens to Avahi in real-time
/// for any `_localsend._tcp` devices appearing or announcing on the network.
pub fn spawn_mdns_browser(state: Arc<AppState>) {
    #[cfg(target_os = "linux")]
    tokio::spawn(async move {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};

        loop {
            let mut cmd = Command::new("avahi-browse");
            cmd.args(["-r", "-p", "_localsend._tcp"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true);

            if let Ok(mut child) = cmd.spawn() {
                if let Some(stdout) = child.stdout.take() {
                    let mut reader = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        // Avahi parseable format:
                        // =;interface;protocol;name;type;domain;hostname;ip;port;txt
                        if line.starts_with('=') {
                            let parts: Vec<&str> = line.split(';').collect();
                            if parts.len() >= 9 {
                                let proto_family = parts[2]; // "IPv4"
                                if proto_family == "IPv4" {
                                    let ip = parts[7];
                                    let port = parts[8].parse::<u16>().unwrap_or(53317);
                                    if !crate::protocol::discovery::is_own_ip(ip) && ip != "127.0.0.1" {
                                        let state_clone = Arc::clone(&state);
                                        let ip_str = ip.to_string();
                                        tokio::spawn(async move {
                                            crate::protocol::discovery::probe_device_at_address(
                                                state_clone,
                                                &ip_str,
                                                port,
                                            ).await;
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                let _ = child.wait().await;
            }

            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });
    #[cfg(not(target_os = "linux"))]
    {
        let _ = state;
    }
}
