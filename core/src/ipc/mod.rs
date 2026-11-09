use crate::protocol::client::{send_files, send_text};
use crate::protocol::PeerDevice;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/synapse-drop.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum IpcCommand {
    GetStatus,
    GetPeers,
    GetBattery,
    GetAutolinkStatus,
    ExploreDir {
        path: Option<String>,
    },
    SendFiles {
        target_ip: String,
        target_port: Option<u16>,
        protocol: Option<String>,
        files: Vec<String>,
    },
    SendText {
        target_ip: String,
        target_port: Option<u16>,
        protocol: Option<String>,
        text: String,
    },
    AcceptTransfer {
        session_id: String,
    },
    RejectTransfer {
        session_id: String,
    },
    SetAutoAccept {
        enabled: bool,
    },
    SetAlias {
        alias: String,
    },
    SubscribeEvents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatus {
    pub is_active: bool,
    pub alias: String,
    pub port: u16,
    pub protocol: String,
    pub auto_accept: bool,
    pub peers_count: usize,
    pub download_dir: String,
    pub battery: Option<crate::state::DeviceBatteryState>,
    pub autolink_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirShortcut {
    pub name: String,
    pub path: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub size_formatted: String,
    pub icon: String,
    pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResponse {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub home_path: String,
    pub shortcuts: Vec<DirShortcut>,
    pub entries: Vec<FileEntry>,
}

pub async fn start_ipc_server(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let socket_path = Path::new(DEFAULT_SOCKET_PATH);
    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }

    let listener = UnixListener::bind(socket_path)?;
    info!("Unix Domain Socket IPC listening on {}", DEFAULT_SOCKET_PATH);

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let state_clone = Arc::clone(&state);
                    tokio::spawn(handle_ipc_connection(stream, state_clone));
                }
                Err(e) => {
                    warn!("IPC accept error: {}", e);
                }
            }
        }
    });

    Ok(())
}

fn get_system_boot_time() -> u64 {
    if let Ok(content) = std::fs::read_to_string("/proc/stat") {
        for line in content.lines() {
            if line.starts_with("btime ") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    if let Ok(ts) = val.parse::<u64>() {
                        return ts;
                    }
                }
            }
        }
    }
    if let Ok(content) = std::fs::read_to_string("/proc/uptime") {
        if let Some(uptime_str) = content.split_whitespace().next() {
            if let Ok(uptime_secs) = uptime_str.parse::<f64>() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                return now.saturating_sub(uptime_secs as u64);
            }
        }
    }
    0
}

fn compute_dir_size(path: &Path, max_depth: usize) -> u64 {
    if max_depth == 0 {
        return 0;
    }
    let mut total = 0;
    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            let p = entry.path();
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                } else if meta.is_dir() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if !file_name.starts_with('.') && !["node_modules", "target", "build", "dist", ".git", ".cache"].contains(&file_name.as_str()) {
                        total += compute_dir_size(&p, max_depth - 1);
                    }
                }
            }
        }
    }
    total
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn file_entry_from_path(path: &Path) -> Option<(FileEntry, u64)> {
    let metadata = std::fs::metadata(path).ok()?;
    let file_name = path.file_name()?.to_string_lossy().to_string();
    if file_name.starts_with('.') {
        return None;
    }

    let is_dir = metadata.is_dir();
    let bytes = if is_dir {
        compute_dir_size(path, 8)
    } else {
        metadata.len()
    };
    let formatted = format_bytes(bytes);

    let (icon, file_type) = if is_dir {
        ("󰉋".to_string(), "folder".to_string())
    } else {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "heic" | "avif" => {
                ("󰈟".to_string(), "image".to_string())
            }
            "mp4" | "mkv" | "webm" | "mov" | "avi" | "flv" | "wmv" => {
                ("󰕧".to_string(), "video".to_string())
            }
            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" => {
                ("󰎆".to_string(), "audio".to_string())
            }
            "pdf" | "docx" | "doc" | "xlsx" | "pptx" | "txt" | "md" | "odt" | "csv" => {
                ("󰈙".to_string(), "document".to_string())
            }
            "zip" | "tar" | "gz" | "xz" | "7z" | "rar" | "zst" => {
                ("󰛫".to_string(), "archive".to_string())
            }
            "rs" | "qml" | "py" | "js" | "ts" | "json" | "sh" | "html" | "css" | "cpp" | "c" | "toml" | "yaml" => {
                ("󰡨".to_string(), "code".to_string())
            }
            _ => ("󰈔".to_string(), "other".to_string()),
        }
    };

    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let created = metadata
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let best_time = mtime.max(created);

    Some((
        FileEntry {
            name: file_name,
            path: path.to_string_lossy().to_string(),
            is_dir,
            size: bytes,
            size_formatted: formatted,
            icon,
            file_type,
        },
        best_time,
    ))
}

fn scan_recent_dir(
    dir: &Path,
    boot_time: u64,
    max_depth: usize,
    current_depth: usize,
    out: &mut Vec<(FileEntry, u64)>,
) {
    if current_depth > max_depth || !dir.exists() {
        return;
    }
    if let Ok(read_dir) = std::fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                if !["node_modules", "target", "build", "dist", ".git", ".cache"].contains(&file_name.as_str()) {
                    if let Some((entry_dto, ts)) = file_entry_from_path(&path) {
                        if ts >= boot_time {
                            out.push((entry_dto, ts));
                        }
                    }
                    scan_recent_dir(&path, boot_time, max_depth, current_depth + 1, out);
                }
            } else if path.is_file() {
                if let Some((entry, ts)) = file_entry_from_path(&path) {
                    if ts >= boot_time {
                        out.push((entry, ts));
                    }
                }
            }
        }
    }
}

pub fn explore_path(target_path: Option<String>) -> ExploreResponse {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let home_path = PathBuf::from(&home);

    let is_recent_req = match &target_path {
        Some(p) => p.trim() == "recent://" || p.trim() == "recent",
        None => false,
    };

    // Build shortcuts list with "Recientes" at the top
    let mut shortcuts = Vec::new();
    shortcuts.push(DirShortcut {
        name: "Recientes".to_string(),
        path: "recent://".to_string(),
        icon: "󰋚".to_string(),
    });

    shortcuts.push(DirShortcut {
        name: "Home".to_string(),
        path: home_path.to_string_lossy().to_string(),
        icon: "󰋜".to_string(),
    });

    let check_and_add = |shortcuts: &mut Vec<DirShortcut>, name: &str, folder: &str, icon: &str| {
        let p = home_path.join(folder);
        if p.exists() {
            shortcuts.push(DirShortcut {
                name: name.to_string(),
                path: p.to_string_lossy().to_string(),
                icon: icon.to_string(),
            });
        }
    };

    let descargas = home_path.join("Descargas");
    if descargas.exists() {
        check_and_add(&mut shortcuts, "Descargas", "Descargas", "󰉍");
    } else {
        check_and_add(&mut shortcuts, "Downloads", "Downloads", "󰉍");
    }

    let imagenes = home_path.join("Imágenes");
    if imagenes.exists() {
        check_and_add(&mut shortcuts, "Imágenes", "Imágenes", "󰈟");
    } else {
        check_and_add(&mut shortcuts, "Pictures", "Pictures", "󰈟");
    }

    let docs = home_path.join("Documentos");
    if docs.exists() {
        check_and_add(&mut shortcuts, "Documentos", "Documentos", "󰈙");
    } else {
        check_and_add(&mut shortcuts, "Documents", "Documents", "󰈙");
    }

    let videos = home_path.join("Vídeos");
    if videos.exists() {
        check_and_add(&mut shortcuts, "Vídeos", "Vídeos", "󰕧");
    } else {
        check_and_add(&mut shortcuts, "Videos", "Videos", "󰕧");
    }

    if is_recent_req {
        let boot_time = get_system_boot_time();
        let mut raw_recent = Vec::new();

        // Scan primary user folders
        scan_recent_dir(&home_path.join("Descargas"), boot_time, 2, 0, &mut raw_recent);
        scan_recent_dir(&home_path.join("Downloads"), boot_time, 2, 0, &mut raw_recent);
        scan_recent_dir(&home_path.join("Documentos"), boot_time, 2, 0, &mut raw_recent);
        scan_recent_dir(&home_path.join("Documents"), boot_time, 2, 0, &mut raw_recent);
        scan_recent_dir(&home_path.join("Imágenes"), boot_time, 2, 0, &mut raw_recent);
        scan_recent_dir(&home_path.join("Pictures"), boot_time, 2, 0, &mut raw_recent);
        scan_recent_dir(&home_path.join("Vídeos"), boot_time, 2, 0, &mut raw_recent);
        scan_recent_dir(&home_path.join("Videos"), boot_time, 2, 0, &mut raw_recent);
        scan_recent_dir(&home_path.join("Escritorio"), boot_time, 2, 0, &mut raw_recent);
        scan_recent_dir(&home_path.join("Desktop"), boot_time, 2, 0, &mut raw_recent);
        // Direct files in home
        scan_recent_dir(&home_path, boot_time, 0, 0, &mut raw_recent);

        // Deduplicate by path
        let mut seen = std::collections::HashSet::new();
        let mut deduped: Vec<(FileEntry, u64)> = Vec::new();
        for item in raw_recent {
            if seen.insert(item.0.path.clone()) {
                deduped.push(item);
            }
        }

        // Sort descending by timestamp (newest first)
        deduped.sort_by(|a, b| b.1.cmp(&a.1));

        let entries: Vec<FileEntry> = deduped.into_iter().map(|(entry, _)| entry).collect();

        return ExploreResponse {
            current_path: "recent://".to_string(),
            parent_path: None,
            home_path: home_path.to_string_lossy().to_string(),
            shortcuts,
            entries,
        };
    }

    let current = match target_path {
        Some(p) if !p.trim().is_empty() => {
            let p_buf = PathBuf::from(&p);
            if p_buf.exists() && p_buf.is_dir() {
                p_buf
            } else if p_buf.exists() && p_buf.parent().is_some() {
                p_buf.parent().unwrap().to_path_buf()
            } else {
                home_path.clone()
            }
        }
        _ => {
            let download = home_path.join("Downloads");
            let descargas = home_path.join("Descargas");
            if descargas.exists() {
                descargas
            } else if download.exists() {
                download
            } else {
                home_path.clone()
            }
        }
    };

    let parent_path = current.parent().map(|p| p.to_string_lossy().to_string());

    let mut entries = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(&current) {
        for entry_res in read_dir.flatten() {
            let entry_path = entry_res.path();
            if let Some((entry, _)) = file_entry_from_path(&entry_path) {
                entries.push(entry);
            }
        }
    }

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else if a.is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    ExploreResponse {
        current_path: current.to_string_lossy().to_string(),
        parent_path,
        home_path: home_path.to_string_lossy().to_string(),
        shortcuts,
        entries,
    }
}

async fn handle_ipc_connection(stream: UnixStream, state: Arc<AppState>) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<IpcCommand>(&line) {
            Ok(IpcCommand::GetStatus) => {
                let status = {
                    let cfg = state.config.read().await;
                    let peers = state.peers.read().await;
                    let battery = state.battery_state.read().await.clone();
                    let autolink_connected = *state.autolink_connected.read().await;
                    DaemonStatus {
                        is_active: true,
                        alias: cfg.alias.clone(),
                        port: cfg.port,
                        protocol: cfg.protocol.clone(),
                        auto_accept: cfg.auto_accept,
                        peers_count: peers.len(),
                        download_dir: cfg.download_dir.to_string_lossy().to_string(),
                        battery,
                        autolink_connected,
                    }
                };
                if let Ok(json) = serde_json::to_string(&status) {
                    let _ = writer.write_all(format!("{}\n", json).as_bytes()).await;
                }
            }
            Ok(IpcCommand::GetBattery) => {
                let battery = state.battery_state.read().await.clone();
                if let Ok(json) = serde_json::to_string(&battery) {
                    let _ = writer.write_all(format!("{}\n", json).as_bytes()).await;
                }
            }
            Ok(IpcCommand::GetAutolinkStatus) => {
                let autolink = *state.autolink_connected.read().await;
                let battery = state.battery_state.read().await.clone();
                let res = serde_json::json!({
                    "connected": autolink,
                    "battery": battery,
                });
                let _ = writer.write_all(format!("{}\n", res).as_bytes()).await;
            }
            Ok(IpcCommand::GetPeers) => {
                let peers: Vec<PeerDevice> = {
                    let p = state.peers.read().await;
                    p.values().cloned().collect()
                };
                if let Ok(json) = serde_json::to_string(&peers) {
                    let _ = writer.write_all(format!("{}\n", json).as_bytes()).await;
                }
            }
            Ok(IpcCommand::ExploreDir { path }) => {
                let res = explore_path(path);
                if let Ok(json) = serde_json::to_string(&res) {
                    let _ = writer.write_all(format!("{}\n", json).as_bytes()).await;
                }
            }
            Ok(IpcCommand::SendFiles {
                target_ip,
                target_port,
                protocol,
                files,
            }) => {
                let state_sender = Arc::clone(&state);
                let port = target_port.unwrap_or(53317);
                let proto = protocol.unwrap_or_else(|| "https".to_string());
                let paths: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
                let target_ip_clone = target_ip.clone();

                tokio::spawn(async move {
                    if let Err(e) = send_files(Arc::clone(&state_sender), target_ip.clone(), port, proto, paths).await {
                        error!("Failed to send files: {}", e);
                        // Clean unreachable peer from peers list immediately
                        let mut peers = state_sender.peers.write().await;
                        peers.retain(|_, p| p.ip != target_ip);
                        let peers_list: Vec<PeerDevice> = peers.values().cloned().collect();
                        drop(peers);
                        let _ = state_sender.event_tx.send(crate::state::IpcEvent::PeersUpdated(peers_list));

                        let _ = tokio::process::Command::new("notify-send")
                            .args([
                                "-a", "DropToSend",
                                "-i", "dialog-warning",
                                "⚠️ Error de Envío",
                                &format!("El dispositivo ({}) no responde o se ha desconectado.", target_ip_clone)
                            ])
                            .output()
                            .await;
                    }
                });

                let _ = writer.write_all(b"{\"status\":\"sending_started\"}\n").await;
            }
            Ok(IpcCommand::SendText {
                target_ip,
                target_port,
                protocol,
                text,
            }) => {
                let state_sender = Arc::clone(&state);
                let port = target_port.unwrap_or(53317);
                let proto = protocol.unwrap_or_else(|| "https".to_string());
                let target_ip_clone = target_ip.clone();

                tokio::spawn(async move {
                    if let Err(e) = send_text(Arc::clone(&state_sender), target_ip.clone(), port, proto, text).await {
                        error!("Failed to send text: {}", e);
                        let mut peers = state_sender.peers.write().await;
                        peers.retain(|_, p| p.ip != target_ip);
                        let peers_list: Vec<PeerDevice> = peers.values().cloned().collect();
                        drop(peers);
                        let _ = state_sender.event_tx.send(crate::state::IpcEvent::PeersUpdated(peers_list));

                        let _ = tokio::process::Command::new("notify-send")
                            .args([
                                "-a", "DropToSend",
                                "-i", "dialog-warning",
                                "⚠️ Error de Envío",
                                &format!("El dispositivo ({}) no responde o se ha desconectado.", target_ip_clone)
                            ])
                            .output()
                            .await;
                    }
                });

                let _ = writer.write_all(b"{\"status\":\"sending_started\"}\n").await;
            }

            Ok(IpcCommand::AcceptTransfer { session_id }) => {
                let mut sessions = state.active_sessions.write().await;
                if let Some(session) = sessions.get_mut(&session_id) {
                    if let Some(tx) = session.response_tx.take() {
                        let _ = tx.send(true);
                    }
                }
                let _ = writer.write_all(b"{\"status\":\"accepted\"}\n").await;
            }
            Ok(IpcCommand::RejectTransfer { session_id }) => {
                let mut sessions = state.active_sessions.write().await;
                if let Some(session) = sessions.get_mut(&session_id) {
                    if let Some(tx) = session.response_tx.take() {
                        let _ = tx.send(false);
                    }
                }
                let _ = writer.write_all(b"{\"status\":\"rejected\"}\n").await;
            }
            Ok(IpcCommand::SetAutoAccept { enabled }) => {
                {
                    let mut cfg = state.config.write().await;
                    cfg.auto_accept = enabled;
                    let _ = cfg.save();
                }
                let _ = writer.write_all(b"{\"status\":\"ok\"}\n").await;
            }
            Ok(IpcCommand::SetAlias { alias }) => {
                {
                    let mut cfg = state.config.write().await;
                    cfg.alias = alias;
                    let _ = cfg.save();
                }
                let _ = writer.write_all(b"{\"status\":\"ok\"}\n").await;
            }
            Ok(IpcCommand::SubscribeEvents) => {
                let mut rx = state.event_tx.subscribe();
                let _ = writer.write_all(b"{\"status\":\"subscribed\"}\n").await;

                while let Ok(event) = rx.recv().await {
                    if let Ok(json) = serde_json::to_string(&event) {
                        if writer.write_all(format!("{}\n", json).as_bytes()).await.is_err() {
                            break;
                        }
                    }
                }
                break;
            }
            Err(e) => {
                let _ = writer
                    .write_all(format!("{{\"error\":\"{}\"}}\n", e).as_bytes())
                    .await;
            }
        }
    }
}
