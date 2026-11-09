use crate::state::TransferHistoryItem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub alias: String,
    pub device_type: String,
    pub device_model: String,
    pub port: u16,
    pub protocol: String, // "https" or "http"
    pub download_dir: PathBuf,
    pub auto_accept: bool,
    #[serde(default)]
    pub trusted_devices: Vec<String>,
    pub fingerprint: String,
    pub discovery_enabled: bool,
    #[serde(default = "default_hover_delay_ms")]
    pub hover_delay_ms: u64,
    #[serde(default = "default_auto_hide_delay_ms")]
    pub auto_hide_delay_ms: u64,
    #[serde(default = "default_notch_idle_width")]
    pub notch_idle_width: f64,
    #[serde(default = "default_panel_opacity")]
    pub panel_opacity: f64,
    #[serde(default = "default_autostart")]
    pub autostart: bool,
    #[serde(default = "default_sound_effects")]
    pub sound_effects: bool,
}

fn default_hover_delay_ms() -> u64 { 1000 }
fn default_auto_hide_delay_ms() -> u64 { 420 }
fn default_notch_idle_width() -> f64 { 144.0 }
fn default_panel_opacity() -> f64 { 0.95 }
fn default_autostart() -> bool { false }
fn default_sound_effects() -> bool { true }

impl Default for AppConfig {
    fn default() -> Self {
        let hostname = std::fs::read_to_string("/etc/hostname")
            .map(|s| s.trim().to_string())
            .or_else(|_| std::env::var("HOSTNAME"))
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "MAX-Desktop".to_string());

        let download_dir = directories::UserDirs::new()
            .and_then(|u| u.download_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join("Downloads")
            });

        let os_model = if cfg!(target_os = "windows") {
            "Windows Desktop".to_string()
        } else {
            "Linux Desktop".to_string()
        };

        Self {
            alias: format!("{} (MAX)", hostname),
            device_type: "desktop".to_string(),
            device_model: os_model,
            port: 53317,
            protocol: "https".to_string(),
            download_dir,
            auto_accept: false,
            trusted_devices: Vec::new(),
            fingerprint: Uuid::new_v4().to_string(),
            discovery_enabled: true,
            hover_delay_ms: 1000,
            auto_hide_delay_ms: 420,
            notch_idle_width: 144.0,
            panel_opacity: 0.95,
            autostart: false,
            sound_effects: true,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        let base = directories::ProjectDirs::from("org", "max", "max-drop")
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".config/max-drop")
            });
        base.join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mut config) = serde_json::from_str::<Self>(&content) {
                    let original_len = config.trusted_devices.len();
                    config.trusted_devices.retain(|item| {
                        let is_raw_hex = item.len() == 64 && item.chars().all(|c| c.is_ascii_hexdigit());
                        !is_raw_hex
                    });
                    if config.trusted_devices.len() != original_len {
                        let _ = config.save();
                    }
                    return config;
                }
            }
        }
        let default_config = Self::default();
        let _ = default_config.save();
        default_config
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn update_autostart_entry(&self) {
        #[cfg(target_os = "linux")]
        {
            if let Some(config_dir) = directories::BaseDirs::new().map(|d| d.config_dir().to_path_buf()) {
                let autostart_dir = config_dir.join("autostart");
                let desktop_file = autostart_dir.join("max.desktop");
                if self.autostart {
                    let _ = std::fs::create_dir_all(&autostart_dir);
                    let content = "[Desktop Entry]\nType=Application\nName=MAX\nComment=MAX Dynamic Notch\nExec=/home/hyprand/Proyectos/antigravity/MAX/run-max.sh\nIcon=system-run\nTerminal=false\nCategories=Utility;\nX-GNOME-Autostart-enabled=true\n";
                    let _ = std::fs::write(&desktop_file, content);
                } else if desktop_file.exists() {
                    let _ = std::fs::remove_file(&desktop_file);
                }
            }
        }
    }

    pub fn history_path() -> PathBuf {
        let base = Self::config_path();
        base.parent().unwrap_or(&PathBuf::from("/tmp")).join("history.json")
    }

    pub fn load_history() -> Vec<TransferHistoryItem> {
        let path = Self::history_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(items) = serde_json::from_str::<Vec<TransferHistoryItem>>(&content) {
                    return items;
                }
            }
        }
        Vec::new()
    }

    pub fn save_history(items: &[TransferHistoryItem]) -> std::io::Result<()> {
        let path = Self::history_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(items)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn sounds_dir() -> PathBuf {
        let base = Self::config_path();
        base.parent().unwrap_or(&PathBuf::from("/tmp")).join("sounds")
    }

    pub fn ensure_sound_assets() {
        let dir = Self::sounds_dir();
        let _ = std::fs::create_dir_all(&dir);
        let click = dir.join("click.wav");
        let snap = dir.join("snap.wav");
        let complete = dir.join("complete.wav");

        if !click.exists() || !snap.exists() || !complete.exists() {
            let generate_wav = |path: &std::path::Path, duration_s: f64, gen_sample: &dyn Fn(f64) -> f64| {
                let sample_rate = 44100u32;
                let num_samples = (sample_rate as f64 * duration_s) as usize;
                let mut pcm_bytes = Vec::with_capacity(44 + num_samples * 2);

                let byte_rate = sample_rate * 2;
                let data_size = (num_samples * 2) as u32;
                let chunk_size = 36 + data_size;

                // RIFF header
                pcm_bytes.extend_from_slice(b"RIFF");
                pcm_bytes.extend_from_slice(&chunk_size.to_le_bytes());
                pcm_bytes.extend_from_slice(b"WAVE");
                pcm_bytes.extend_from_slice(b"fmt ");
                pcm_bytes.extend_from_slice(&16u32.to_le_bytes());
                pcm_bytes.extend_from_slice(&1u16.to_le_bytes());
                pcm_bytes.extend_from_slice(&1u16.to_le_bytes());
                pcm_bytes.extend_from_slice(&sample_rate.to_le_bytes());
                pcm_bytes.extend_from_slice(&byte_rate.to_le_bytes());
                pcm_bytes.extend_from_slice(&2u16.to_le_bytes());
                pcm_bytes.extend_from_slice(&16u16.to_le_bytes());
                pcm_bytes.extend_from_slice(b"data");
                pcm_bytes.extend_from_slice(&data_size.to_le_bytes());

                for i in 0..num_samples {
                    let t = i as f64 / sample_rate as f64;
                    let sample = gen_sample(t).clamp(-1.0, 1.0);
                    let sample_i16 = (sample * 32767.0) as i16;
                    pcm_bytes.extend_from_slice(&sample_i16.to_le_bytes());
                }

                let _ = std::fs::write(path, pcm_bytes);
            };

            if !click.exists() {
                generate_wav(&click, 0.025, &|t| {
                    (2.0 * std::f64::consts::PI * 1800.0 * t).sin() * (-t / 0.004).exp() * 0.4
                });
            }

            if !snap.exists() {
                generate_wav(&snap, 0.035, &|t| {
                    let freq = 720.0 - (360.0 * (t / 0.035));
                    (2.0 * std::f64::consts::PI * freq * t).sin() * (-t / 0.009).exp() * 0.45
                });
            }

            if !complete.exists() {
                generate_wav(&complete, 0.13, &|t| {
                    if t < 0.04 {
                        (2.0 * std::f64::consts::PI * 1046.5 * t).sin() * (1.0 - t / 0.05) * 0.35
                    } else {
                        let t2 = t - 0.04;
                        (2.0 * std::f64::consts::PI * 1567.98 * t2).sin() * (-t2 / 0.04).exp() * 0.35
                    }
                });
            }
        }
    }
}
