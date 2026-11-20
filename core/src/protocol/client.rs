use crate::config::AppConfig;
use crate::protocol::tls::get_or_create_tls_certs;
use crate::protocol::{DeviceDto, DeviceType, FileDto, PrepareUploadRequestDto, PrepareUploadResponseDto, TransferProgress};
use crate::state::{AppState, IpcEvent};
use futures_util::TryStreamExt;
use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tracing::info;
use uuid::Uuid;

static TRANSFER_HTTPS_CLIENT: OnceLock<Client> = OnceLock::new();
static TRANSFER_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
static SCAN_HTTPS_CLIENT: OnceLock<Client> = OnceLock::new();
static SCAN_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn build_client(
    target_protocol: &str,
    connect_timeout_ms: u64,
    request_timeout_ms: Option<u64>,
    tcp_keepalive_secs: Option<u64>,
    max_idle_per_host: usize,
    idle_timeout_secs: u64,
) -> Client {
    let mut builder = Client::builder()
        .danger_accept_invalid_certs(true)
        .connect_timeout(Duration::from_millis(connect_timeout_ms))
        .pool_max_idle_per_host(max_idle_per_host)
        .pool_idle_timeout(Duration::from_secs(idle_timeout_secs));

    if let Some(req_ms) = request_timeout_ms {
        builder = builder.timeout(Duration::from_millis(req_ms));
    }
    if let Some(keepalive_s) = tcp_keepalive_secs {
        builder = builder.tcp_keepalive(Duration::from_secs(keepalive_s));
    }

    if target_protocol == "https" {
        let cert_dir = AppConfig::config_path()
            .parent()
            .unwrap_or(&PathBuf::from("/tmp"))
            .join("certs");
        if let Ok(tls_cfg) = get_or_create_tls_certs(cert_dir) {
            let mut combined_pem = tls_cfg.cert_pem;
            combined_pem.extend_from_slice(b"\n");
            combined_pem.extend_from_slice(&tls_cfg.key_pem);
            if let Ok(identity) = reqwest::Identity::from_pem(&combined_pem) {
                builder = builder.identity(identity);
            }
        }
    }

    builder.build().unwrap_or_else(|_| Client::new())
}

fn collect_files_to_send(
    path: &PathBuf,
    base_parent: &PathBuf,
    out: &mut Vec<(String, PathBuf, u64)>,
) {
    if path.is_file() {
        if let Ok(meta) = std::fs::metadata(path) {
            let rel_name = match path.strip_prefix(base_parent) {
                Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                Err(_) => path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "file".to_string()),
            };
            out.push((rel_name, path.clone(), meta.len()));
        }
    } else if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let sub_path = entry.path();
                collect_files_to_send(&sub_path, base_parent, out);
            }
        }
    }
}

pub fn create_http_client(target_protocol: &str) -> reqwest::Client {
    if target_protocol == "https" {
        TRANSFER_HTTPS_CLIENT
            .get_or_init(|| build_client("https", 2500, None, Some(5), 2, 30))
            .clone()
    } else {
        TRANSFER_HTTP_CLIENT
            .get_or_init(|| build_client("http", 2500, None, Some(5), 2, 30))
            .clone()
    }
}

pub fn create_scan_client(target_protocol: &str) -> reqwest::Client {
    if target_protocol == "https" {
        SCAN_HTTPS_CLIENT
            .get_or_init(|| build_client("https", 400, Some(800), None, 1, 15))
            .clone()
    } else {
        SCAN_HTTP_CLIENT
            .get_or_init(|| build_client("http", 400, Some(800), None, 1, 15))
            .clone()
    }
}

pub async fn send_files(
    state: Arc<AppState>,
    target_ip: String,
    target_port: u16,
    target_protocol: String,
    file_paths: Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = create_http_client(&target_protocol);

    let (my_info, _session_id) = {
        let cfg = state.config.read().await;
        (
            DeviceDto {
                alias: cfg.alias.clone(),
                version: "2.1".to_string(),
                device_model: Some(cfg.device_model.clone()),
                device_type: DeviceType::Desktop,
                fingerprint: cfg.fingerprint.clone(),
                port: cfg.port,
                protocol: cfg.protocol.clone(),
                download: true,
                announcement: None,
                announce: None,
            },
            Uuid::new_v4().to_string(),
        )
    };

    let mut files_map = HashMap::new();
    let mut file_ids_and_paths = Vec::new();
    let mut collected: Vec<(String, PathBuf, u64)> = Vec::new();

    let mut canonical_paths: Vec<PathBuf> = Vec::new();
    for p in &file_paths {
        if p.exists() {
            let canon = p.canonicalize().unwrap_or_else(|_| p.clone());
            canonical_paths.push(canon);
        }
    }
    canonical_paths.sort_by(|a, b| a.as_os_str().len().cmp(&b.as_os_str().len()));

    let mut filtered_paths: Vec<PathBuf> = Vec::new();
    for p in canonical_paths {
        let is_subsumed = filtered_paths.iter().any(|parent| {
            parent.is_dir() && p != *parent && p.starts_with(parent)
        });
        if !is_subsumed {
            filtered_paths.push(p);
        }
    }

    for path in &filtered_paths {
        if !path.exists() {
            continue;
        }
        if path.is_file() {
            if let Ok(meta) = tokio::fs::metadata(path).await {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "file".to_string());
                collected.push((file_name, path.clone(), meta.len()));
            }
        } else if path.is_dir() {
            let base_parent = path.parent().unwrap_or(path).to_path_buf();
            collect_files_to_send(path, &base_parent, &mut collected);
        }
    }

    let mut total_batch_bytes: u64 = 0;
    for (file_name, file_path, file_size) in collected {
        total_batch_bytes = total_batch_bytes.saturating_add(file_size);
        let file_id = Uuid::new_v4().to_string();
        let file_dto = FileDto {
            id: file_id.clone(),
            file_name,
            size: file_size,
            file_type: "application/octet-stream".to_string(),
            sha256: None,
            preview: None,
        };

        files_map.insert(file_id.clone(), file_dto);
        file_ids_and_paths.push((file_id, file_path));
    }

    if files_map.is_empty() {
        return Err("No hay archivos validos para enviar".into());
    }

    let prepare_url = format!(
        "{}://{}:{}/api/localsend/v2/prepare-upload",
        target_protocol, target_ip, target_port
    );

    let prepare_req = PrepareUploadRequestDto {
        info: my_info,
        files: files_map.clone(),
    };

    info!("Sending prepare-upload to {}", prepare_url);
    let resp = client.post(&prepare_url).json(&prepare_req).send().await?;

    if !resp.status().is_success() {
        return Err("La transferencia fue rechazada".into());
    }

    let prepare_resp: PrepareUploadResponseDto = resp.json().await?;
    let start_time = Instant::now();
    let mut cumulative_bytes_sent: u64 = 0;

    let total_file_count = file_ids_and_paths.len();

    for (file_idx, (file_id, path)) in file_ids_and_paths.iter().enumerate() {
        let token = match prepare_resp.files.get(file_id) {
            Some(t) => t,
            None => continue,
        };

        let file_meta = &files_map[file_id];
        let upload_url = format!(
            "{}://{}:{}/api/localsend/v2/upload?sessionId={}&fileId={}&token={}",
            target_protocol, target_ip, target_port, prepare_resp.session_id, file_id, token
        );

        let file = File::open(&path).await?;
        let file_name = file_meta.file_name.clone();

        let state_progress = state.clone();
        let sid = prepare_resp.session_id.clone();
        let fid = file_id.clone();
        let fname = file_name.clone();
        let base_sent = cumulative_bytes_sent;
        let total_all = total_batch_bytes.max(1);
        let current_completed_files = file_idx;

        let reader_stream = ReaderStream::with_capacity(file, 65536);
        let mut file_bytes_sent: u64 = 0;
        let mut last_emit = Instant::now() - std::time::Duration::from_millis(160);

        let tracked_stream = reader_stream.inspect_ok(move |chunk| {
            file_bytes_sent += chunk.len() as u64;
            let now_sent = base_sent.saturating_add(file_bytes_sent);
            let is_done = now_sent >= total_all;

            if is_done || last_emit.elapsed().as_millis() >= 150 {
                last_emit = Instant::now();
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    now_sent as f64 / elapsed
                } else {
                    0.0
                };

                let _ = state_progress.event_tx.send(IpcEvent::TransferProgress(TransferProgress {
                    session_id: sid.clone(),
                    file_id: fid.clone(),
                    file_name: fname.clone(),
                    bytes_transferred: now_sent,
                    total_bytes: total_all,
                    speed_bytes_per_sec: speed,
                    is_finished: is_done,
                    is_error: false,
                    error_message: None,
                    files_completed: current_completed_files,
                    total_files: total_file_count,
                }));
            }
        });

        let body = reqwest::Body::wrap_stream(tracked_stream);
        let upload_resp = client.post(&upload_url).body(body).send().await?;

        if !upload_resp.status().is_success() {
            return Err("La transferencia fue interrumpida".into());
        }

        cumulative_bytes_sent = cumulative_bytes_sent.saturating_add(file_meta.size);
    }

    let target_alias = {
        let peers = state.peers.read().await;
        peers.values()
            .find(|p| p.ip == target_ip)
            .map(|p| p.alias.clone())
            .unwrap_or_else(|| target_ip.clone())
    };

    let first_path = file_ids_and_paths.first().map(|(_, p)| p.to_string_lossy().to_string());
    let first_name = file_ids_and_paths.first()
        .and_then(|(_, p)| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "archivo".to_string());
    let display_name = if file_ids_and_paths.len() > 1 {
        format!("{} archivos", file_ids_and_paths.len())
    } else {
        first_name
    };

    let _ = state.event_tx.send(IpcEvent::TransferCompleted {
        session_id: prepare_resp.session_id.clone(),
        success: true,
        message: "Archivos enviados correctamente".to_string(),
        file_path: first_path.clone(),
        folder_path: None,
        is_incoming: false,
        file_count: file_ids_and_paths.len(),
        total_file_count: file_ids_and_paths.len(),
        is_partial: false,
    });

    let history_item = crate::state::TransferHistoryItem {
        id: uuid::Uuid::new_v4().to_string(),
        file_name: display_name,
        file_path: first_path,
        file_size: cumulative_bytes_sent,
        is_incoming: false,
        device_alias: target_alias,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        success: true,
    };
    state.record_history_item(history_item).await;
    crate::state::trim_working_set();

    Ok(())
}

pub async fn send_text(
    state: Arc<AppState>,
    target_ip: String,
    target_port: u16,
    target_protocol: String,
    text: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = create_http_client(&target_protocol);

    let (my_info, session_id) = {
        let cfg = state.config.read().await;
        (
            DeviceDto {
                alias: cfg.alias.clone(),
                version: "2.1".to_string(),
                device_model: Some(cfg.device_model.clone()),
                device_type: DeviceType::Desktop,
                fingerprint: cfg.fingerprint.clone(),
                port: cfg.port,
                protocol: cfg.protocol.clone(),
                download: true,
                announcement: None,
                announce: None,
            },
            Uuid::new_v4().to_string(),
        )
    };

    let file_id = Uuid::new_v4().to_string();
    let text_bytes = text.as_bytes();
    let text_preview = if text.len() > 30 {
        format!("{}.txt", &text[..30].replace(['\n', '\r', '/', '\\'], " "))
    } else {
        format!("{}.txt", text.replace(['\n', '\r', '/', '\\'], " "))
    };

    let file_dto = FileDto {
        id: file_id.clone(),
        file_name: text_preview,
        size: text_bytes.len() as u64,
        file_type: "text/plain".to_string(),
        sha256: None,
        preview: Some(text.clone()),
    };

    let mut files_map = HashMap::new();
    files_map.insert(file_id.clone(), file_dto);

    let prepare_url = format!(
        "{}://{}:{}/api/localsend/v2/prepare-upload",
        target_protocol, target_ip, target_port
    );

    let prepare_req = PrepareUploadRequestDto {
        info: my_info,
        files: files_map,
    };

    info!("Sending text prepare-upload to {}", prepare_url);
    let resp = client.post(&prepare_url).json(&prepare_req).send().await?;

    if !resp.status().is_success() {
        let err_msg = format!("Peer declined text transfer: status {}", resp.status());
        let _ = state.event_tx.send(IpcEvent::TransferCompleted {
            session_id: session_id.clone(),
            success: false,
            message: err_msg.clone(),
            file_path: None,
            folder_path: None,
            is_incoming: false,
            file_count: 0,
            total_file_count: 0,
            is_partial: false,
        });
        return Err(err_msg.into());
    }

    let prepare_resp: PrepareUploadResponseDto = resp.json().await?;
    let token = prepare_resp
        .files
        .get(&file_id)
        .ok_or("Missing token for text upload")?;

    let upload_url = format!(
        "{}://{}:{}/api/localsend/v2/upload?sessionId={}&fileId={}&token={}",
        target_protocol, target_ip, target_port, prepare_resp.session_id, file_id, token
    );

    let upload_resp = client.post(&upload_url).body(text_bytes.to_vec()).send().await?;
    if !upload_resp.status().is_success() {
        let _ = state.event_tx.send(IpcEvent::TransferCompleted {
            session_id: prepare_resp.session_id.clone(),
            success: false,
            message: "Text upload failed".to_string(),
            file_path: None,
            folder_path: None,
            is_incoming: false,
            file_count: 0,
            total_file_count: 1,
            is_partial: false,
        });
        return Err(format!("Text upload failed: status {}", upload_resp.status()).into());
    }

    let _ = state.event_tx.send(IpcEvent::TransferCompleted {
        session_id: prepare_resp.session_id.clone(),
        success: true,
        message: "Text sent successfully".to_string(),
        file_path: None,
        folder_path: None,
        is_incoming: false,
        file_count: 1,
        total_file_count: 1,
        is_partial: false,
    });

    Ok(())
}
