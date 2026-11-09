use crate::state::{AppState, DeviceBatteryState, IpcEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tracing::info;

pub struct BluetoothDeviceStatus {
    pub mac: String,
    pub name: String,
    pub is_connected: bool,
    pub is_paired: bool,
    pub is_apple_device: bool,
    pub battery_percentage: Option<u8>,
}

/// Spawns a background task that periodically checks paired Bluetooth devices,
/// detects connected iPhones for AutoLink presence, and emits IPC events.
pub fn spawn_bluetooth_watcher(state: Arc<AppState>) {
    #[cfg(target_os = "linux")]
    tokio::spawn(async move {
        info!("Bluetooth AutoLink Proximity watcher started");

        // Keep PC discoverable and pairable on launch
        let _ = Command::new("bluetoothctl")
            .args(["discoverable", "on"])
            .output()
            .await;
        let _ = Command::new("bluetoothctl")
            .args(["pairable", "on"])
            .output()
            .await;

        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            if let Ok(devices) = get_paired_bluetooth_devices().await {
                let mut connected_device_found = false;

                for dev in devices {
                    // Strictly require that the device is ACTUALLY connected right now
                    if dev.is_connected && (dev.is_apple_device || dev.name.to_lowercase().contains("iphone") || dev.name.to_lowercase() == "andss") {
                        connected_device_found = true;

                        // Update autolink status if connected
                        let mut autolink = state.autolink_connected.write().await;
                        if !*autolink {
                            *autolink = true;
                            let _ = state.event_tx.send(IpcEvent::AutolinkStatus {
                                connected: true,
                                ip: None,
                                device_name: dev.name.clone(),
                            });
                        }
                        break;
                    }
                }

                // If no target device is actively connected via Bluetooth
                if !connected_device_found {
                    let mut autolink = state.autolink_connected.write().await;
                    if *autolink {
                        *autolink = false;
                        let _ = state.event_tx.send(IpcEvent::AutolinkStatus {
                            connected: false,
                            ip: None,
                            device_name: String::new(),
                        });
                    }
                }
            }
        }
    });
    #[cfg(not(target_os = "linux"))]
    {
        let _ = state;
    }
}

/// Query bluetoothctl for paired devices and their detailed status
pub async fn get_paired_bluetooth_devices() -> Result<Vec<BluetoothDeviceStatus>, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("bluetoothctl")
            .arg("devices")
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();

        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Line format: Device XX:XX:XX:XX:XX:XX Name...
            if parts.len() >= 3 && parts[0] == "Device" {
                let mac = parts[1].to_string();
                let name = parts[2..].join(" ");

                if let Ok(info) = get_device_info(&mac, &name).await {
                    devices.push(info);
                }
            }
        }

        Ok(devices)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Vec::new())
    }
}

/// Query bluetoothctl info <MAC> for detailed connection and battery properties
#[cfg(target_os = "linux")]
async fn get_device_info(mac: &str, fallback_name: &str) -> Result<BluetoothDeviceStatus, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("bluetoothctl")
        .args(["info", mac])
        .output()
        .await?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut name = fallback_name.to_string();
    let mut is_connected = false;
    let mut is_paired = false;
    let mut is_apple_device = false;
    let mut battery_percentage = None;

    let lower_fallback = fallback_name.to_lowercase();
    if lower_fallback.contains("iphone") || lower_fallback.contains("ipad") || lower_fallback.contains("apple") || lower_fallback == "andss" {
        is_apple_device = true;
    }

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Name:") {
            if let Some(n) = trimmed.strip_prefix("Name:") {
                name = n.trim().to_string();
                let lower_name = name.to_lowercase();
                if lower_name.contains("iphone") || lower_name.contains("ipad") || lower_name.contains("apple") || lower_name == "andss" {
                    is_apple_device = true;
                }
            }
        } else if trimmed.starts_with("Connected:") {
            is_connected = trimmed.contains("yes");
        } else if trimmed.starts_with("Paired:") {
            is_paired = trimmed.contains("yes");
        } else if trimmed.starts_with("Icon:") {
            if trimmed.contains("phone") {
                is_apple_device = true;
            }
        } else if trimmed.starts_with("Modalias:") || trimmed.starts_with("ManufacturerData.Key:") {
            if trimmed.contains("004C") || trimmed.contains("0x4c00") {
                is_apple_device = true;
            }
        } else if trimmed.starts_with("Battery Percentage:") {
            if let Some(val_str) = trimmed.strip_prefix("Battery Percentage:") {
                let s = val_str.trim();
                if let Some(start) = s.find('(') {
                    if let Some(end) = s.find(')') {
                        if let Ok(p) = s[start + 1..end].trim().parse::<u8>() {
                            battery_percentage = Some(p);
                        }
                    }
                } else {
                    let cleaned: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                    if let Ok(p) = cleaned.parse::<u8>() {
                        battery_percentage = Some(p);
                    }
                }
            }
        }
    }

    Ok(BluetoothDeviceStatus {
        mac: mac.to_string(),
        name,
        is_connected,
        is_paired,
        is_apple_device,
        battery_percentage,
    })
}
