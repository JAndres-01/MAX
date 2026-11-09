use crate::config::AppConfig;
use crate::protocol::{DeviceDto, FileDto, PeerDevice, TransferProgress};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum IpcEvent {
    PeersUpdated(Vec<PeerDevice>),
    IncomingRequest(IncomingTransferRequest),
    TransferProgress(TransferProgress),
    TransferCompleted {
        session_id: String,
        success: bool,
        message: String,
        file_path: Option<String>,
        folder_path: Option<String>,
        is_incoming: bool,
        file_count: usize,
        #[serde(default)]
        total_file_count: usize,
        #[serde(default)]
        is_partial: bool,
    },
    HistoryUpdated(Vec<TransferHistoryItem>),
    StatusChanged { is_active: bool, alias: String, port: u16 },
    ClipboardUpdated(ClipboardItem),
    BatteryUpdated(DeviceBatteryState),
    NotificationReceived(DeviceNotification),
    AutolinkStatus { connected: bool, ip: Option<String>, device_name: String },
    ConfigUpdated(AppConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferHistoryItem {
    pub id: String,
    pub file_name: String,
    pub file_path: Option<String>,
    pub file_size: u64,
    pub is_incoming: bool,
    pub device_alias: String,
    pub timestamp: u64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceBatteryState {
    pub percentage: u8,
    pub is_charging: bool,
    pub device_name: String,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceNotification {
    pub app: String,
    pub title: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: String,
    pub text: String,
    pub item_type: String, // "text" or "url"
    pub timestamp: u64,
    pub source: String,    // "pc" or "iphone"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingTransferRequest {
    pub session_id: String,
    pub sender: DeviceDto,
    pub files: Vec<FileDto>,
    pub total_size: u64,
}

pub struct ActiveSession {
    pub session_id: String,
    pub sender_info: DeviceDto,
    pub files: HashMap<String, FileDto>,
    pub file_tokens: HashMap<String, String>, // file_id -> token
    pub accepted_files: HashMap<String, String>, // file_id -> destination file path
    pub response_tx: Option<tokio::sync::oneshot::Sender<bool>>,
    pub cancel_tx: Option<mpsc::Sender<()>>,
    pub completed_file_ids: std::collections::HashSet<String>,
    pub completed_bytes: u64,
    pub total_session_bytes: u64,
    pub start_time: std::time::Instant,
}

pub struct AppState {
    pub config: RwLock<AppConfig>,
    pub peers: RwLock<HashMap<String, PeerDevice>>, // IP/fingerprint -> PeerDevice
    pub active_sessions: RwLock<HashMap<String, ActiveSession>>,
    pub active_send_abort: RwLock<Option<tokio::task::AbortHandle>>,
    pub transfer_history: RwLock<Vec<TransferHistoryItem>>,
    pub clipboard_history: RwLock<Vec<ClipboardItem>>,
    pub battery_state: RwLock<Option<DeviceBatteryState>>,
    pub autolink_connected: RwLock<bool>,
    pub clipboard_ws_tx: broadcast::Sender<ClipboardItem>,
    pub event_tx: broadcast::Sender<IpcEvent>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        let (clipboard_ws_tx, _) = broadcast::channel(100);
        let history = AppConfig::load_history();
        Self {
            config: RwLock::new(config),
            peers: RwLock::new(HashMap::new()),
            active_sessions: RwLock::new(HashMap::new()),
            active_send_abort: RwLock::new(None),
            transfer_history: RwLock::new(history),
            clipboard_history: RwLock::new(Vec::new()),
            battery_state: RwLock::new(None),
            autolink_connected: RwLock::new(false),
            clipboard_ws_tx,
            event_tx,
        }
    }

    pub async fn record_history_item(&self, item: TransferHistoryItem) {
        let mut history = self.transfer_history.write().await;
        history.insert(0, item);
        if history.len() > 60 {
            history.truncate(60);
        }
        let list: Vec<TransferHistoryItem> = history.clone();
        drop(history);
        let _ = AppConfig::save_history(&list);
        let _ = self.event_tx.send(IpcEvent::HistoryUpdated(list));
    }

    pub async fn clear_history(&self) {
        let mut history = self.transfer_history.write().await;
        history.clear();
        drop(history);
        let _ = AppConfig::save_history(&[]);
        let _ = self.event_tx.send(IpcEvent::HistoryUpdated(Vec::new()));
    }

    pub async fn add_or_update_peer(&self, mut peer: PeerDevice) {
        let (my_fingerprint, my_alias) = {
            let cfg = self.config.read().await;
            (cfg.fingerprint.clone(), cfg.alias.clone())
        };

        // Do not add ourselves
        if peer.fingerprint == my_fingerprint || peer.alias == my_alias {
            return;
        }

        // Do not add loopback or local host IPs
        if peer.ip == "127.0.0.1" || peer.ip == "::1" || peer.ip.starts_with("127.") {
            return;
        }

        if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
            for (_name, local_ip) in interfaces {
                if local_ip.to_string() == peer.ip {
                    return;
                }
            }
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        peer.last_seen = now;

        let mut peers = self.peers.write().await;
        // Key by fingerprint so each unique physical device is recorded only once!
        peers.insert(peer.fingerprint.clone(), peer);

        let peers_list: Vec<PeerDevice> = peers.values().cloned().collect();
        drop(peers);
        let _ = self.event_tx.send(IpcEvent::PeersUpdated(peers_list));
    }

    pub async fn clean_stale_peers(&self, ttl_seconds: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut peers = self.peers.write().await;
        let original_len = peers.len();
        peers.retain(|_, p| now.saturating_sub(p.last_seen) < ttl_seconds);

        if peers.len() != original_len {
            let peers_list: Vec<PeerDevice> = peers.values().cloned().collect();
            drop(peers);
            let _ = self.event_tx.send(IpcEvent::PeersUpdated(peers_list));
        }
    }
}
