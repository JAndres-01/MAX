use crate::config::AppConfig;
use crate::protocol::tls::get_or_create_tls_certs;
use crate::protocol::{
    DeviceDto, DeviceType, FileDto, PeerDevice, PrepareUploadRequestDto, PrepareUploadResponseDto,
    TransferProgress,
};
use crate::state::{ActiveSession, AppState, IncomingTransferRequest, IpcEvent};
use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use futures_util::StreamExt;
use notify_rust::Notification;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UploadQuery {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "fileId")]
    pub file_id: String,
    pub token: String,
}

#[derive(Deserialize)]
pub struct CancelQuery {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

use axum::extract::ConnectInfo;

pub async fn start_server(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (port, protocol) = {
        let cfg = state.config.read().await;
        (cfg.port, cfg.protocol.clone())
    };

    let app = Router::new()
        .route("/", get(handle_pwa))
        .route("/clipboard", get(handle_pwa))
        .route("/api/clipboard", get(handle_get_clipboard).post(handle_post_clipboard))
        .route("/api/clipboard/history", get(handle_get_clipboard_history))
        .route("/ws/clipboard", get(handle_ws_clipboard))
        .route("/api/upload", post(handle_direct_upload))
        .route("/api/battery", get(handle_get_battery).post(handle_post_battery))
        .route("/api/notification", post(handle_post_notification))
        .route("/api/autolink", get(handle_get_autolink))
        .route("/api/autolink/ping", post(handle_post_autolink_ping))
        .route("/api/localsend/v2/info", get(handle_info))
        .route("/api/localsend/v2/register", post(handle_register))
        .route("/api/localsend/v2/prepare-upload", post(handle_prepare_upload))
        .route("/api/localsend/v2/upload", post(handle_upload))
        .route("/api/localsend/v2/cancel", post(handle_cancel))
        .route("/api/localsend/v2/show", post(handle_show))
        .route("/api/localsend/v1/info", get(handle_info))
        .route("/api/localsend/v1/register", post(handle_register))
        .route("/api/localsend/v1/send-request", post(handle_prepare_upload))
        .route("/api/localsend/v1/send", post(handle_upload))
        .route("/api/localsend/v1/cancel", post(handle_cancel))
        .route("/api/localsend/v1/show", post(handle_show))
        .route("/api/max/v1/peers", get(handle_max_peers))
        .route("/api/max/v1/send", post(handle_max_send))
        .route("/api/max/v1/respond", post(handle_max_respond))
        .route("/api/max/v1/config", get(handle_max_get_config).post(handle_max_post_config))
        .route("/api/max/v1/whitelist/remove", post(handle_max_remove_whitelist))
        .route("/api/max/v1/history", get(handle_max_get_history))
        .route("/api/max/v1/history/clear", post(handle_max_clear_history))
        .route("/api/max/v1/open", post(handle_max_open_path))
        .route("/api/max/v1/events", get(handle_max_ws_events))
        .layer(axum::extract::DefaultBodyLimit::disable())
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state.clone());

    // Dedicated Local Control Server for MAX Flutter UI (127.0.0.1:53320)
    let control_addr = SocketAddr::from(([127, 0, 0, 1], 53320));
    if let Ok(control_listener) = tokio::net::TcpListener::bind(control_addr).await {
        let control_app = app.clone();
        info!("MAX UI Control Server listening on http://{}", control_addr);
        tokio::spawn(async move {
            let _ = axum::serve(
                control_listener,
                control_app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await;
        });
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    if protocol == "https" {
        let cert_dir = AppConfig::config_path()
            .parent()
            .unwrap_or(&PathBuf::from("/tmp"))
            .join("certs");
        let tls_cfg = get_or_create_tls_certs(cert_dir)?;
        let rustls_config = RustlsConfig::from_pem(tls_cfg.cert_pem, tls_cfg.key_pem).await?;

        let https_app = app.clone();
        info!("LocalSend v2 HTTPS Server listening on https://{}", addr);
        tokio::spawn(async move {
            if let Err(e) = axum_server::bind_rustls(addr, rustls_config)
                .serve(https_app.into_make_service_with_connect_info::<SocketAddr>())
                .await
            {
                warn!("HTTPS Server bind notice (port may be shared with daemon): {}", e);
            }
        });

        // Also run an HTTP companion server on port 53318 for frictionless Apple Shortcuts & Safari PWA
        let http_companion_port = port + 1;
        let http_addr = SocketAddr::from(([0, 0, 0, 0], http_companion_port));
        if let Ok(listener) = tokio::net::TcpListener::bind(http_addr).await {
            info!("Synapse Continuity HTTP Web & Shortcuts Server listening on http://{}", http_addr);
            tokio::spawn(async move {
                if let Err(e) = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await {
                    error!("HTTP Companion Server error: {}", e);
                }
            });
        }
    } else {
        info!("LocalSend v2 HTTP Server listening on http://{}", addr);
        if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
            tokio::spawn(async move {
                if let Err(e) = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await {
                    error!("HTTP Server error: {}", e);
                }
            });
        }
    }

    Ok(())
}

async fn handle_info(State(state): State<Arc<AppState>>) -> Json<DeviceDto> {
    let cfg = state.config.read().await;
    Json(DeviceDto {
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
    })
}

async fn handle_register(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<DeviceDto>,
) -> Json<DeviceDto> {
    let client_ip = match addr {
        SocketAddr::V4(v4) => v4.ip().to_string(),
        SocketAddr::V6(v6) => v6.ip().to_string(),
    };

    let (my_fingerprint, cfg) = {
        let c = state.config.read().await;
        (c.fingerprint.clone(), c.clone())
    };

    if payload.fingerprint != my_fingerprint {
        let peer = PeerDevice {
            ip: client_ip,
            port: payload.port,
            alias: payload.alias.clone(),
            device_model: payload.device_model.clone(),
            device_type: payload.device_type.clone(),
            fingerprint: payload.fingerprint.clone(),
            protocol: payload.protocol.clone(),
            download: payload.download,
            last_seen: 0,
        };
        state.add_or_update_peer(peer).await;
    }

    Json(DeviceDto {
        alias: cfg.alias,
        version: "2.1".to_string(),
        device_model: Some(cfg.device_model),
        device_type: DeviceType::Desktop,
        fingerprint: cfg.fingerprint,
        port: cfg.port,
        protocol: cfg.protocol,
        download: true,
        announcement: None,
        announce: None,
    })
}

async fn handle_prepare_upload(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PrepareUploadRequestDto>,
) -> Response {
    let session_id = Uuid::new_v4().to_string();
    let (is_trusted, download_dir) = {
        let cfg = state.config.read().await;
        let trusted = cfg.auto_accept
            || cfg.trusted_devices.contains(&payload.info.fingerprint)
            || cfg.trusted_devices.contains(&payload.info.alias);
        (trusted, cfg.download_dir.clone())
    };

    let files_list: Vec<FileDto> = payload.files.values().cloned().collect();
    let total_size: u64 = files_list.iter().map(|f| f.size).sum();

    // Map file tokens
    let mut file_tokens = HashMap::new();
    let mut accepted_files = HashMap::new();

    for (id, file) in &payload.files {
        let token = Uuid::new_v4().to_string();
        file_tokens.insert(id.clone(), token);

        let dest_path = sanitize_destination_path(&download_dir, &file.file_name);
        accepted_files.insert(id.clone(), dest_path.to_string_lossy().to_string());
    }

    let (response_tx, response_rx) = tokio::sync::oneshot::channel::<bool>();
    let total_session_bytes: u64 = payload.files.values().map(|f| f.size).sum();

    {
        let mut sessions = state.active_sessions.write().await;
        sessions.insert(
            session_id.clone(),
            ActiveSession {
                session_id: session_id.clone(),
                sender_info: payload.info.clone(),
                files: payload.files.clone(),
                file_tokens: file_tokens.clone(),
                accepted_files: accepted_files.clone(),
                response_tx: Some(response_tx),
                cancel_tx: None,
                completed_file_ids: std::collections::HashSet::new(),
                completed_bytes: 0,
                total_session_bytes,
                start_time: std::time::Instant::now(),
            },
        );
    }

    let accepted = if is_trusted {
        true
    } else {
        let req_event = IncomingTransferRequest {
            session_id: session_id.clone(),
            sender: payload.info.clone(),
            files: files_list.clone(),
            total_size,
        };
        let _ = state.event_tx.send(IpcEvent::IncomingRequest(req_event));

        // Wait for UI approval or timeout after 60s
        match tokio::time::timeout(tokio::time::Duration::from_secs(60), response_rx).await {
            Ok(Ok(decision)) => decision,
            _ => false,
        }
    };

    if accepted {
        let response_dto = PrepareUploadResponseDto {
            session_id,
            files: file_tokens,
        };
        Json(response_dto).into_response()
    } else {
        let mut sessions = state.active_sessions.write().await;
        sessions.remove(&session_id);
        (StatusCode::FORBIDDEN, "Transfer declined").into_response()
    }
}

async fn handle_upload(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UploadQuery>,
    body: Body,
) -> Response {
    let (
        target_path,
        file_meta,
        sender_alias,
        total_session_files,
        total_session_bytes,
        base_completed_bytes,
        session_start_time,
        current_file_idx,
    ) = {
        let sessions = state.active_sessions.read().await;
        let session = match sessions.get(&query.session_id) {
            Some(s) => s,
            None => return (StatusCode::NOT_FOUND, "Session not found").into_response(),
        };

        if session.file_tokens.get(&query.file_id) != Some(&query.token) {
            return (StatusCode::FORBIDDEN, "Invalid token").into_response();
        }

        let path = session.accepted_files.get(&query.file_id).cloned();
        let meta = session.files.get(&query.file_id).cloned();
        let alias = session.sender_info.alias.clone();
        let count = session.files.len();
        let total_bytes = session.total_session_bytes;
        let completed_bytes = session.completed_bytes;
        let start = session.start_time;
        let idx = session.completed_file_ids.len() + 1;
        (path, meta, alias, count, total_bytes, completed_bytes, start, idx)
    };

    let target_path = match target_path {
        Some(p) => PathBuf::from(p),
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "Path missing").into_response(),
    };

    let file_meta = match file_meta {
        Some(m) => m,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "Metadata missing").into_response(),
    };

    if let Some(parent) = target_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    let raw_file = match File::create(&target_path).await {
        Ok(f) => f,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create file: {}", e),
            )
                .into_response()
        }
    };
    let mut file = tokio::io::BufWriter::with_capacity(131_072, raw_file);

    let recv_session_tag = format!("recv::{}::{}", sender_alias, query.session_id);
    let mut stream = body.into_data_stream();
    let mut bytes_written: u64 = 0;
    let file_total_bytes = file_meta.size;
    let total_all = total_session_bytes.max(file_total_bytes).max(1);
    let mut last_emit = Instant::now() - std::time::Duration::from_millis(160);

    let progress_file_name = if total_session_files > 1 {
        format!("{} ({}/{})", file_meta.file_name, current_file_idx, total_session_files)
    } else {
        file_meta.file_name.clone()
    };

    while let Some(chunk_res) = stream.next().await {
        // Check if session was cancelled via /cancel
        {
            let sessions = state.active_sessions.read().await;
            if !sessions.contains_key(&query.session_id) {
                drop(file);
                let _ = tokio::fs::remove_file(&target_path).await;
                // handle_cancel already removed the session and emitted TransferCompleted
                return (StatusCode::GONE, "Cancelled").into_response();
            }
        }

        match chunk_res {
            Ok(chunk) => {
                if let Err(e) = file.write_all(&chunk).await {
                    let (completed_count, total_count, all_done_folder) = {
                        let mut sessions = state.active_sessions.write().await;
                        if let Some(session) = sessions.remove(&query.session_id) {
                            let comp = session.completed_file_ids.len();
                            let total = session.files.len();
                            let folder = target_path.parent().map(|p| p.to_string_lossy().to_string());
                            (comp, total, folder)
                        } else {
                            (0, 0, None)
                        }
                    };
                    drop(file);
                    let _ = tokio::fs::remove_file(&target_path).await;

                    let is_partial = completed_count > 0;
                    let msg = if is_partial {
                        format!("{} DE {} ARCHIVOS RECIBIDOS", completed_count, total_count)
                    } else {
                        "La transferencia fallo".to_string()
                    };

                    let _ = state.event_tx.send(IpcEvent::TransferCompleted {
                        session_id: recv_session_tag.clone(),
                        success: false,
                        message: msg,
                        file_path: None,
                        folder_path: all_done_folder,
                        is_incoming: true,
                        file_count: completed_count,
                        total_file_count: total_count,
                        is_partial,
                    });
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Write error: {}", e),
                    )
                        .into_response();
                }
                bytes_written += chunk.len() as u64;

                let now_transferred = base_completed_bytes.saturating_add(bytes_written);

                if bytes_written >= file_total_bytes || last_emit.elapsed().as_millis() >= 150 {
                    last_emit = Instant::now();
                    let elapsed = session_start_time.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        now_transferred as f64 / elapsed
                    } else {
                        0.0
                    };

                    let completed_count = {
                        let sessions = state.active_sessions.read().await;
                        sessions
                            .get(&query.session_id)
                            .map(|s| s.completed_file_ids.len())
                            .unwrap_or(0)
                    };

                    let _ = state.event_tx.send(IpcEvent::TransferProgress(TransferProgress {
                        session_id: recv_session_tag.clone(),
                        file_id: query.file_id.clone(),
                        file_name: progress_file_name.clone(),
                        bytes_transferred: now_transferred,
                        total_bytes: total_all,
                        speed_bytes_per_sec: speed,
                        is_finished: false,
                        is_error: false,
                        error_message: None,
                        files_completed: completed_count,
                        total_files: total_session_files,
                    }));
                }
            }
            Err(_) => {
                let (completed_count, total_count, all_done_folder) = {
                    let mut sessions = state.active_sessions.write().await;
                    if let Some(session) = sessions.remove(&query.session_id) {
                        let comp = session.completed_file_ids.len();
                        let total = session.files.len();
                        let folder = target_path.parent().map(|p| p.to_string_lossy().to_string());
                        (comp, total, folder)
                    } else {
                        (0, 0, None)
                    }
                };
                drop(file);
                let _ = tokio::fs::remove_file(&target_path).await;

                let is_partial = completed_count > 0;
                let msg = if is_partial {
                    format!("{} DE {} ARCHIVOS RECIBIDOS", completed_count, total_count)
                } else {
                    "La transferencia fallo".to_string()
                };

                let _ = state.event_tx.send(IpcEvent::TransferCompleted {
                    session_id: recv_session_tag.clone(),
                    success: false,
                    message: msg,
                    file_path: None,
                    folder_path: all_done_folder,
                    is_incoming: true,
                    file_count: completed_count,
                    total_file_count: total_count,
                    is_partial,
                });
                return (StatusCode::BAD_REQUEST, "Stream aborted").into_response();
            }
        }
    }

    if bytes_written < file_total_bytes {
        let (completed_count, total_count, all_done_folder) = {
            let mut sessions = state.active_sessions.write().await;
            if let Some(session) = sessions.remove(&query.session_id) {
                let comp = session.completed_file_ids.len();
                let total = session.files.len();
                let folder = target_path.parent().map(|p| p.to_string_lossy().to_string());
                (comp, total, folder)
            } else {
                (0, 0, None)
            }
        };
        drop(file);
        let _ = tokio::fs::remove_file(&target_path).await;

        let is_partial = completed_count > 0;
        let msg = if is_partial {
            format!("{} DE {} ARCHIVOS RECIBIDOS", completed_count, total_count)
        } else {
            "La transferencia fallo".to_string()
        };

        let _ = state.event_tx.send(IpcEvent::TransferCompleted {
            session_id: recv_session_tag.clone(),
            success: false,
            message: msg,
            file_path: None,
            folder_path: all_done_folder,
            is_incoming: true,
            file_count: completed_count,
            total_file_count: total_count,
            is_partial,
        });
        return (StatusCode::BAD_REQUEST, "Incomplete upload").into_response();
    }

    let _ = file.flush().await;
    drop(file);

    // Record history item for this specific file
    let target_path_str = target_path.to_string_lossy().to_string();
    let history_item = crate::state::TransferHistoryItem {
        id: uuid::Uuid::new_v4().to_string(),
        file_name: file_meta.file_name.clone(),
        file_path: Some(target_path_str.clone()),
        file_size: bytes_written,
        is_incoming: true,
        device_alias: sender_alias.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        success: true,
    };
    state.record_history_item(history_item).await;

    // Check if this was the last file in the session!
    let (is_all_done, total_files_count, all_done_folder, total_session_done_bytes) = {
        let mut sessions = state.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(&query.session_id) {
            session.completed_file_ids.insert(query.file_id.clone());
            session.completed_bytes = session.completed_bytes.saturating_add(bytes_written);
            let done = session.completed_file_ids.len() >= session.files.len();
            let folder = target_path.parent().map(|p| p.to_string_lossy().to_string());
            let count = session.files.len();
            let cbytes = session.completed_bytes;
            (done, count, folder, cbytes)
        } else {
            (true, 1, target_path.parent().map(|p| p.to_string_lossy().to_string()), bytes_written)
        }
    };

    if is_all_done {
        // Clean up session
        {
            let mut sessions = state.active_sessions.write().await;
            sessions.remove(&query.session_id);
        }

        let _ = state.event_tx.send(IpcEvent::TransferProgress(TransferProgress {
            session_id: recv_session_tag.clone(),
            file_id: query.file_id.clone(),
            file_name: progress_file_name,
            bytes_transferred: total_all,
            total_bytes: total_all,
            speed_bytes_per_sec: 0.0,
            is_finished: true,
            is_error: false,
            error_message: None,
            files_completed: total_files_count,
            total_files: total_files_count,
        }));

        let completion_msg = if total_files_count > 1 {
            format!(
                "{} TE ENVIO {} ARCHIVOS",
                sender_alias.to_uppercase(),
                total_files_count
            )
        } else {
            format!(
                "{} TE ENVIO {}",
                sender_alias.to_uppercase(),
                file_meta.file_name.to_uppercase()
            )
        };

        let _ = state.event_tx.send(IpcEvent::TransferCompleted {
            session_id: recv_session_tag,
            success: true,
            message: completion_msg,
            file_path: if total_files_count == 1 { Some(target_path_str) } else { None },
            folder_path: all_done_folder.clone(),
            is_incoming: true,
            file_count: total_files_count,
            total_file_count: total_files_count,
            is_partial: false,
        });

        let summary_text = if total_files_count > 1 {
            format!("{} files received", total_files_count)
        } else {
            "File received".to_string()
        };
        let body_text = format!("Saved to {}", all_done_folder.as_deref().unwrap_or("downloads"));

        let _ = Notification::new()
            .summary(&summary_text)
            .body(&body_text)
            .icon("document-save")
            .show();

        crate::state::trim_working_set();
    } else {
        // More files still pending in this session!
        let completed_now = {
            let sessions = state.active_sessions.read().await;
            sessions
                .get(&query.session_id)
                .map(|s| s.completed_file_ids.len())
                .unwrap_or(1)
        };

        let _ = state.event_tx.send(IpcEvent::TransferProgress(TransferProgress {
            session_id: recv_session_tag.clone(),
            file_id: query.file_id.clone(),
            file_name: progress_file_name,
            bytes_transferred: total_session_done_bytes,
            total_bytes: total_all,
            speed_bytes_per_sec: 0.0,
            is_finished: false,
            is_error: false,
            error_message: None,
            files_completed: completed_now,
            total_files: total_files_count,
        }));
    }

    StatusCode::OK.into_response()
}

async fn handle_cancel(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CancelQuery>,
) -> StatusCode {
    let removed_session = {
        let mut sessions = state.active_sessions.write().await;
        sessions.remove(&query.session_id)
    };
    if let Some(abort_handle) = state.active_send_abort.write().await.take() {
        abort_handle.abort();
    }

    let download_dir = state.config.read().await.download_dir.to_string_lossy().to_string();

    if let Some(session) = removed_session {
        let completed = session.completed_file_ids.len();
        let total = session.files.len();
        let is_partial = completed > 0;
        let sender_alias = session.sender_info.alias;
        let recv_session_tag = format!("recv::{}::{}", sender_alias, query.session_id);

        let msg = if is_partial {
            format!("{} DE {} ARCHIVOS RECIBIDOS", completed, total)
        } else {
            "La transferencia fallo".to_string()
        };

        let _ = state.event_tx.send(IpcEvent::TransferCompleted {
            session_id: recv_session_tag,
            success: false,
            message: msg,
            file_path: None,
            folder_path: if is_partial { Some(download_dir.clone()) } else { None },
            is_incoming: true,
            file_count: completed,
            total_file_count: total,
            is_partial,
        });

        if is_partial {
            let _ = Notification::new()
                .summary(&format!("Transferencia parcial: {}/{} recibidos", completed, total))
                .body(&format!("Guardados en {}", download_dir))
                .icon("document-save")
                .show();
        }
    } else {
        let _ = state.event_tx.send(IpcEvent::TransferCompleted {
            session_id: query.session_id,
            success: false,
            message: "La transferencia fallo".to_string(),
            file_path: None,
            folder_path: None,
            is_incoming: true,
            file_count: 0,
            total_file_count: 0,
            is_partial: false,
        });
    }
    StatusCode::OK
}

async fn handle_show() -> StatusCode {
    StatusCode::OK
}

#[derive(Deserialize, Debug)]
pub struct PostClipboardPayload {
    #[serde(alias = "Text", alias = "TEXT", alias = "content", alias = "Content", alias = "clipboard", alias = "Clipboard", alias = "message", alias = "Message", alias = "data", alias = "Data", alias = "body", alias = "Body")]
    pub text: Option<String>,
    #[serde(alias = "device", alias = "Device", alias = "deviceName", alias = "device_name", alias = "sender", alias = "alias")]
    pub device: Option<String>,
}

async fn handle_pwa() -> impl IntoResponse {
    ([(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")], crate::pwa::PWA_HTML)
}

async fn handle_get_clipboard(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let history = state.clipboard_history.read().await;
    let item_opt = history.first().cloned();
    drop(history);

    let item = if let Some(item) = item_opt {
        item
    } else {
        match crate::clipboard::get_wayland_clipboard().await {
            Ok(text) if !text.is_empty() => {
                crate::clipboard::add_clipboard_entry(&state, text, "pc", None, false).await
            }
            _ => crate::state::ClipboardItem {
                id: "".to_string(),
                text: "".to_string(),
                item_type: "text".to_string(),
                timestamp: 0,
                source: "pc".to_string(),
            },
        }
    };

    if params.get("format").map(|s| s.as_str()) == Some("raw") {
        ([(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")], item.text).into_response()
    } else {
        Json(item).into_response()
    }
}

async fn handle_post_clipboard(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let raw_str = String::from_utf8_lossy(&body).to_string();
    info!("handle_post_clipboard received raw body ({} bytes): '{}'", body.len(), raw_str);

    let mut text = String::new();
    let mut device_opt = None;

    // 1. Try parsing struct with case-insensitive aliases
    if let Ok(payload) = serde_json::from_slice::<PostClipboardPayload>(&body) {
        if let Some(t) = payload.text {
            text = t;
        }
        if let Some(d) = payload.device {
            device_opt = Some(d);
        }
    }

    // 2. If empty, check arbitrary JSON value/object
    if text.is_empty() {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(s) = val.as_str() {
                text = s.to_string();
            } else if let Some(obj) = val.as_object() {
                for (k, v) in obj {
                    if k.eq_ignore_ascii_case("device") || k.eq_ignore_ascii_case("alias") {
                        if let Some(d) = v.as_str() {
                            device_opt = Some(d.to_string());
                        }
                    } else if text.is_empty() {
                        if let Some(s) = v.as_str() {
                            text = s.to_string();
                        }
                    }
                }
            }
        }
    }

    // 3. Fallback to raw string if it's not JSON or parsing failed
    if text.is_empty() {
        let trimmed = raw_str.trim();
        if !trimmed.is_empty() {
            text = trimmed.to_string();
        }
    }

    info!("handle_post_clipboard resolved text: '{}'", text);

    if text.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Empty text"}))).into_response();
    }

    if let Err(e) = crate::clipboard::set_wayland_clipboard(&text).await {
        warn!("Warning: Failed to set wayland clipboard directly: {} (saving to history & notifications)", e);
    }

    // Resolve friendly device name
    let client_ip = match addr {
        SocketAddr::V4(v4) => v4.ip().to_string(),
        SocketAddr::V6(v6) => v6.ip().to_string(),
    };

    let mut device_name = device_opt.unwrap_or_default();

    if device_name.is_empty() {
        let peers = state.peers.read().await;
        if let Some(peer) = peers.get(&client_ip) {
            device_name = peer.alias.clone();
        }
    }

    if device_name.is_empty() {
        if let Some(ua) = headers.get("user-agent").and_then(|v| v.to_str().ok()) {
            if ua.contains("iPad") {
                device_name = "iPad".to_string();
            } else if ua.contains("iPhone") || ua.contains("Shortcuts") || ua.contains("CFNetwork") || ua.contains("Darwin") {
                device_name = "iPhone".to_string();
            } else if ua.contains("Android") {
                device_name = "Android".to_string();
            } else if ua.contains("Macintosh") || ua.contains("Mac OS") {
                device_name = "Mac".to_string();
            }
        }
    }

    if device_name.is_empty() {
        device_name = "iPhone".to_string();
    }

    let item = crate::clipboard::add_clipboard_entry(&state, text.clone(), "iphone", Some(device_name), true).await;

    (StatusCode::OK, Json(serde_json::json!({"success": true, "item": item}))).into_response()
}

struct DirectUploadStreamResult {
    bytes_written: u64,
    multipart_filename: Option<String>,
    magic_prefix: [u8; 16],
    magic_len: usize,
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn extract_boundary_from_content_type(content_type: &str) -> Option<String> {
    for param in content_type.split(';').skip(1) {
        let trimmed = param.trim();
        if trimmed.len() > 9 && trimmed[..9].eq_ignore_ascii_case("boundary=") {
            let val = trimmed[9..].trim().trim_matches('"').trim_matches('\'').trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn extract_multipart_filename(header_part: &str) -> Option<String> {
    let lower = header_part.to_ascii_lowercase();
    if let Some(fn_pos) = lower.find("filename=") {
        let fn_str = &header_part[fn_pos + 9..];
        let fn_clean = fn_str.lines().next().unwrap_or("").trim_matches('"').trim_matches('\'').trim();
        if !fn_clean.is_empty() {
            let end_fn = fn_clean.find('"').or_else(|| fn_clean.find(';')).unwrap_or(fn_clean.len());
            let parsed = fn_clean[..end_fn].trim();
            if !parsed.is_empty() {
                return Some(parsed.to_string());
            }
        }
    }
    None
}

fn append_magic_prefix(magic_prefix: &mut [u8; 16], magic_len: &mut usize, data: &[u8]) {
    if *magic_len < 16 && !data.is_empty() {
        let take = (16 - *magic_len).min(data.len());
        magic_prefix[*magic_len..*magic_len + take].copy_from_slice(&data[..take]);
        *magic_len += take;
    }
}

async fn write_payload_chunk(
    writer: &mut tokio::io::BufWriter<File>,
    chunk: &[u8],
    closing_marker: &[u8],
    tail: &mut Vec<u8>,
    closing_found: &mut bool,
    magic_prefix: &mut [u8; 16],
    magic_len: &mut usize,
    bytes_written: &mut u64,
) -> Result<(), (StatusCode, String)> {
    if chunk.is_empty() || *closing_found {
        return Ok(());
    }

    if closing_marker.is_empty() {
        append_magic_prefix(magic_prefix, magic_len, chunk);
        writer
            .write_all(chunk)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Write error: {}", e)))?;
        *bytes_written += chunk.len() as u64;
        return Ok(());
    }

    tail.extend_from_slice(chunk);
    if let Some(idx) = find_subslice(tail, closing_marker) {
        let payload = &tail[..idx];
        if !payload.is_empty() {
            append_magic_prefix(magic_prefix, magic_len, payload);
            writer
                .write_all(payload)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Write error: {}", e)))?;
            *bytes_written += payload.len() as u64;
        }
        tail.clear();
        *closing_found = true;
    } else if tail.len() >= closing_marker.len() {
        let safe_len = tail.len() - (closing_marker.len() - 1);
        let payload = &tail[..safe_len];
        append_magic_prefix(magic_prefix, magic_len, payload);
        writer
            .write_all(payload)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Write error: {}", e)))?;
        *bytes_written += safe_len as u64;
        tail.drain(..safe_len);
    }

    Ok(())
}

async fn stream_direct_upload_to_file<S, E>(
    mut stream: S,
    content_type: &str,
    dest_path: &std::path::Path,
) -> Result<DirectUploadStreamResult, (StatusCode, String)>
where
    S: futures_util::Stream<Item = Result<axum::body::Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    let raw_file = File::create(dest_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Create file error: {}", e)))?;
    let mut writer = tokio::io::BufWriter::with_capacity(131_072, raw_file);

    let header_is_multipart = content_type.trim().to_ascii_lowercase().starts_with("multipart/form-data");
    let header_boundary = extract_boundary_from_content_type(content_type);

    let mut probe_buf = Vec::<u8>::with_capacity(4096);
    let mut header_end_pos: Option<usize> = None;
    let mut stream_eof = false;

    loop {
        if header_is_multipart {
            if let Some(pos) = find_subslice(&probe_buf, b"\r\n\r\n") {
                header_end_pos = Some(pos);
                break;
            }
        } else if probe_buf.len() >= 2 && !probe_buf.starts_with(b"--") {
            break;
        } else if probe_buf.starts_with(b"--") {
            if let Some(pos) = find_subslice(&probe_buf, b"\r\n\r\n") {
                let hdr_lower = String::from_utf8_lossy(&probe_buf[..pos]).to_ascii_lowercase();
                if probe_buf.len() > 100
                    || hdr_lower.contains("content-disposition")
                    || hdr_lower.contains("filename=")
                {
                    header_end_pos = Some(pos);
                    break;
                }
            }
        }

        if probe_buf.len() >= 8192 {
            break;
        }

        match stream.next().await {
            Some(Ok(chunk)) => probe_buf.extend_from_slice(&chunk),
            Some(Err(e)) => {
                return Err((StatusCode::BAD_REQUEST, format!("Stream error: {}", e)));
            }
            None => {
                stream_eof = true;
                break;
            }
        }
    }

    if probe_buf.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Empty body".to_string()));
    }

    let mut multipart_filename: Option<String> = None;
    let mut closing_marker = Vec::<u8>::new();
    let payload_start: usize;

    if let Some(pos) = header_end_pos {
        let header_part = String::from_utf8_lossy(&probe_buf[..pos]);
        multipart_filename = extract_multipart_filename(&header_part);
        if let Some(boundary) = header_boundary {
            closing_marker = format!("\r\n--{}", boundary).into_bytes();
        } else {
            let first_line_end = find_subslice(&probe_buf[..pos], b"\n").unwrap_or(pos);
            let mut first_line = &probe_buf[..first_line_end];
            if first_line.ends_with(b"\r") {
                first_line = &first_line[..first_line.len() - 1];
            }
            if first_line.starts_with(b"--") && first_line.len() > 2 {
                closing_marker.extend_from_slice(b"\r\n");
                closing_marker.extend_from_slice(first_line);
            }
        }
        payload_start = pos + 4;
    } else {
        payload_start = 0;
    }

    let mut tail = Vec::<u8>::new();
    let mut closing_found = false;
    let mut magic_prefix = [0u8; 16];
    let mut magic_len = 0usize;
    let mut bytes_written = 0u64;

    if payload_start < probe_buf.len() {
        write_payload_chunk(
            &mut writer,
            &probe_buf[payload_start..],
            &closing_marker,
            &mut tail,
            &mut closing_found,
            &mut magic_prefix,
            &mut magic_len,
            &mut bytes_written,
        )
        .await?;
    }
    drop(probe_buf);

    if !stream_eof {
        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Stream error: {}", e)))?;
            if !closing_found {
                write_payload_chunk(
                    &mut writer,
                    &chunk,
                    &closing_marker,
                    &mut tail,
                    &mut closing_found,
                    &mut magic_prefix,
                    &mut magic_len,
                    &mut bytes_written,
                )
                .await?;
            }
        }
    }

    if !closing_marker.is_empty() && !closing_found && !tail.is_empty() {
        append_magic_prefix(&mut magic_prefix, &mut magic_len, &tail);
        writer
            .write_all(&tail)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Write error: {}", e)))?;
        bytes_written += tail.len() as u64;
    }

    writer
        .flush()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Flush error: {}", e)))?;
    drop(writer);

    if bytes_written == 0 {
        return Err((StatusCode::BAD_REQUEST, "Empty body".to_string()));
    }

    Ok(DirectUploadStreamResult {
        bytes_written,
        multipart_filename,
        magic_prefix,
        magic_len,
    })
}

async fn handle_direct_upload(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<HashMap<String, String>>,
    headers: axum::http::HeaderMap,
    body: Body,
) -> impl IntoResponse {
    info!("handle_direct_upload streaming request from {}", addr);
    let download_dir = {
        let cfg = state.config.read().await;
        cfg.download_dir.clone()
    };

    let content_type = headers.get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    let download_dir_path = PathBuf::from(&download_dir);
    if let Err(e) = tokio::fs::create_dir_all(&download_dir_path).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Create dir error: {}", e)})),
        )
            .into_response();
    }

    let temp_path = download_dir_path.join(format!(".max_upload_{}.tmp", Uuid::new_v4()));
    let stream = body.into_data_stream();

    let upload_res = match stream_direct_upload_to_file(stream, content_type, &temp_path).await {
        Ok(res) => res,
        Err((status, msg)) => {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return (status, Json(serde_json::json!({"error": msg}))).into_response();
        }
    };

    let total_written = upload_res.bytes_written;
    let multipart_filename = upload_res.multipart_filename;

    // 1. Determine filename and extension
    let filename_param = params.get("filename").or_else(|| params.get("name")).cloned();
    let filename_header = headers.get("x-filename")
        .or_else(|| headers.get("x-file-name"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let cd_filename = headers.get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .and_then(|cd| {
            if let Some(pos) = cd.find("filename=") {
                let s = &cd[pos + 9..];
                let trimmed = s.trim_matches('"').trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
            None
        });

    let (magic_ext, magic_category) =
        detect_extension_from_magic_bytes(&upload_res.magic_prefix[..upload_res.magic_len]);

    let header_ext = match content_type {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/heic" => "heic",
        "image/heif" => "heif",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "video/mp4" => "mp4",
        "video/quicktime" => "mov",
        "application/pdf" => "pdf",
        "application/zip" => "zip",
        "text/plain" => "txt",
        _ => magic_ext,
    };

    let best_ext = if magic_ext != "bin" { magic_ext } else { header_ext };

    let now = chrono_or_timestamp();

    let raw_name_opt = filename_param
        .or(filename_header)
        .or(cd_filename)
        .or(multipart_filename)
        .map(|s| clean_and_dedup_filename(&s))
        .filter(|s| !s.is_empty() && s != "unnamed_file" && s != "file");

    let mut final_name = if let Some(name) = raw_name_opt {
        name
    } else {
        format!("iPhone_{}_{}.{}", magic_category, now, best_ext)
    };

    // Ensure the filename has a valid extension
    if std::path::Path::new(&final_name).extension().is_none() && !best_ext.is_empty() && best_ext != "bin" {
        final_name = format!("{}.{}", final_name, best_ext);
    }

    let target_path = sanitize_destination_path(&download_dir_path, &final_name);

    if tokio::fs::rename(&temp_path, &target_path).await.is_err() {
        if let Err(e) = tokio::fs::copy(&temp_path, &target_path).await {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Write error: {}", e)})),
            )
                .into_response();
        }
        let _ = tokio::fs::remove_file(&temp_path).await;
    }

    // Resolve friendly device name
    let client_ip = match addr {
        SocketAddr::V4(v4) => v4.ip().to_string(),
        SocketAddr::V6(v6) => v6.ip().to_string(),
    };

    let mut device_name = String::new();
    {
        let peers = state.peers.read().await;
        if let Some(peer) = peers.get(&client_ip) {
            device_name = peer.alias.clone();
        }
    }
    if device_name.is_empty() {
        if let Some(ua) = headers.get("user-agent").and_then(|v| v.to_str().ok()) {
            if ua.contains("iPad") {
                device_name = "iPad".to_string();
            } else if ua.contains("iPhone") || ua.contains("Shortcuts") || ua.contains("CFNetwork") || ua.contains("Darwin") {
                device_name = "iPhone".to_string();
            } else if ua.contains("Android") {
                device_name = "Android".to_string();
            } else if ua.contains("Macintosh") || ua.contains("Mac OS") {
                device_name = "Mac".to_string();
            }
        }
    }
    if device_name.is_empty() {
        device_name = "iPhone".to_string();
    }

    let file_display_name = target_path.file_name().and_then(|s| s.to_str()).unwrap_or(&final_name);
    let summary_title = format!("Enviado desde {}", device_name);
    let body_msg = format!("📁 Guardado: {}", file_display_name);

    let is_img = best_ext == "jpg" || best_ext == "png" || best_ext == "heic" || best_ext == "webp" || best_ext == "gif";
    let icon_name = if is_img { "image-x-generic" } else { "document-save" };

    let _ = Notification::new()
        .summary(&summary_title)
        .body(&body_msg)
        .icon(icon_name)
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();

    let session_id = format!("recv::{}::{}", device_name, Uuid::new_v4());
    let target_path_str = target_path.to_string_lossy().to_string();
    let folder_path_str = target_path.parent().map(|p| p.to_string_lossy().to_string());

    let _ = state.event_tx.send(IpcEvent::TransferCompleted {
        session_id,
        success: true,
        message: format!("{} TE ENVIO {}", device_name.to_uppercase(), file_display_name.to_uppercase()),
        file_path: Some(target_path_str.clone()),
        folder_path: folder_path_str,
        is_incoming: true,
        file_count: 1,
        total_file_count: 1,
        is_partial: false,
    });

    let history_item = crate::state::TransferHistoryItem {
        id: uuid::Uuid::new_v4().to_string(),
        file_name: file_display_name.to_string(),
        file_path: Some(target_path_str),
        file_size: total_written,
        is_incoming: true,
        device_alias: device_name.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        success: true,
    };
    state.record_history_item(history_item).await;
    crate::state::trim_working_set();

    (StatusCode::OK, Json(serde_json::json!({
        "success": true,
        "filename": file_display_name,
        "path": target_path.to_string_lossy(),
        "size": total_written
    }))).into_response()
}

async fn handle_get_clipboard_history(State(state): State<Arc<AppState>>) -> Json<Vec<crate::state::ClipboardItem>> {
    let history = state.clipboard_history.read().await;
    Json(history.clone())
}

async fn handle_ws_clipboard(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_clipboard_socket(socket, state))
}

async fn handle_clipboard_socket(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let mut rx = state.clipboard_ws_tx.subscribe();

    // Send latest clip initially
    {
        let history = state.clipboard_history.read().await;
        if let Some(item) = history.first() {
            if let Ok(json) = serde_json::to_string(item) {
                let _ = socket.send(axum::extract::ws::Message::Text(json)).await;
            }
        }
    }

    loop {
        tokio::select! {
            Ok(item) = rx.recv() => {
                if let Ok(json) = serde_json::to_string(&item) {
                    if socket.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        let _ = crate::clipboard::set_wayland_clipboard(&text).await;
                        crate::clipboard::add_clipboard_entry(&state, text, "iphone", Some("iPhone Web PWA".to_string()), true).await;
                    }
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

fn sanitize_destination_path(dir: &PathBuf, filename: &str) -> PathBuf {
    // Prevent directory traversal attacks while preserving valid relative paths (e.g., Folder/sub/doc.txt)
    let mut safe_path = dir.clone();
    let components: Vec<&str> = filename
        .split('/')
        .flat_map(|s| s.split('\\'))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect();

    if components.is_empty() {
        safe_path.push("unnamed_file");
    } else {
        for comp in components {
            safe_path.push(comp);
        }
    }

    if !safe_path.exists() {
        return safe_path;
    }

    let parent = safe_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| dir.clone());
    let file_name = safe_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let stem = std::path::Path::new(&file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let ext = std::path::Path::new(&file_name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let mut counter = 1;
    let mut dest = safe_path;
    while dest.exists() {
        dest = parent.join(format!("{}_{}{}", stem, counter, ext));
        counter += 1;
    }
    dest
}

fn detect_extension_from_magic_bytes(bytes: &[u8]) -> (&'static str, &'static str) {
    if bytes.len() >= 4 && &bytes[0..4] == b"%PDF" {
        return ("pdf", "Document");
    }
    if bytes.len() >= 3 && &bytes[0..3] == b"\xFF\xD8\xFF" {
        return ("jpg", "Photo");
    }
    if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        return ("png", "Image");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return ("webp", "Image");
    }
    if bytes.len() >= 4 && &bytes[0..4] == b"GIF8" {
        return ("gif", "Image");
    }
    if bytes.len() >= 8 && (&bytes[4..8] == b"ftyp" || &bytes[4..8] == b"moov") {
        if bytes.len() >= 12 && (&bytes[8..12] == b"heic" || &bytes[8..12] == b"heix" || &bytes[8..12] == b"mif1") {
            return ("heic", "Photo");
        }
        return ("mp4", "Video");
    }
    if bytes.len() >= 4 && &bytes[0..4] == b"PK\x03\x04" {
        return ("zip", "Archive");
    }
    if bytes.len() >= 2 && &bytes[0..2] == b"\x1f\x8b" {
        return ("tar.gz", "Archive");
    }
    ("bin", "File")
}

fn chrono_or_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now)
}

fn clean_and_dedup_filename(raw: &str) -> String {
    let s = raw.trim().trim_matches('"').trim_matches('\'').to_string();
    let path = std::path::Path::new(&s);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(&s);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let stem_len = stem.len();
    let clean_stem = if stem_len >= 6 && stem_len % 2 == 0 {
        let (first, second) = stem.split_at(stem_len / 2);
        if first == second {
            first
        } else {
            stem
        }
    } else {
        stem
    };

    if ext.is_empty() {
        clean_stem.to_string()
    } else {
        format!("{}.{}", clean_stem, ext)
    }
}

#[derive(Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BatteryReportDto {
    #[serde(deserialize_with = "deserialize_battery_pct", alias = "Percentage", alias = "battery", alias = "Battery", alias = "level", alias = "Level")]
    pub percentage: u8,
    #[serde(default, alias = "is_charging", alias = "isCharging", alias = "charging", alias = "Charging")]
    pub is_charging: bool,
    #[serde(default = "default_iphone_name", alias = "device_name", alias = "deviceName", alias = "device", alias = "Device", alias = "name", alias = "Name")]
    pub device_name: String,
}

fn deserialize_battery_pct<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f <= 1.0 && f > 0.0 {
                    Ok((f * 100.0).round() as u8)
                } else {
                    Ok(f.round().clamp(0.0, 100.0) as u8)
                }
            } else if let Some(i) = n.as_u64() {
                Ok(i.min(100) as u8)
            } else {
                Err(D::Error::custom("Invalid battery percentage number"))
            }
        }
        serde_json::Value::String(s) => {
            let clean = s.trim().trim_end_matches('%');
            if let Ok(f) = clean.parse::<f64>() {
                if f <= 1.0 && f > 0.0 {
                    Ok((f * 100.0).round() as u8)
                } else {
                    Ok(f.round().clamp(0.0, 100.0) as u8)
                }
            } else {
                Err(D::Error::custom("Invalid battery percentage string"))
            }
        }
        _ => Err(D::Error::custom("Expected number or string for battery percentage")),
    }
}

fn default_iphone_name() -> String {
    "iPhone".to_string()
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationReportDto {
    #[serde(default = "default_app_name")]
    pub app: String,
    pub title: String,
    pub message: String,
}

fn default_app_name() -> String {
    "iPhone".to_string()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutolinkStatusDto {
    pub connected: bool,
    pub battery: Option<crate::state::DeviceBatteryState>,
}

async fn handle_get_battery(State(state): State<Arc<AppState>>) -> Json<Option<crate::state::DeviceBatteryState>> {
    let battery = state.battery_state.read().await.clone();
    Json(battery)
}

async fn handle_post_battery(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BatteryReportDto>,
) -> (StatusCode, Json<serde_json::Value>) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    info!(
        "Live battery telemetry received from iPhone ({}): {}% (charging: {})",
        payload.device_name, payload.percentage, payload.is_charging
    );

    let new_state = crate::state::DeviceBatteryState {
        percentage: payload.percentage,
        is_charging: payload.is_charging,
        device_name: payload.device_name,
        last_updated: now,
    };

    {
        let mut b = state.battery_state.write().await;
        *b = Some(new_state.clone());
    }

    let _ = state.event_tx.send(IpcEvent::BatteryUpdated(new_state));

    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn handle_post_notification(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NotificationReportDto>,
) -> (StatusCode, Json<serde_json::Value>) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let notification_event = crate::state::DeviceNotification {
        app: payload.app.clone(),
        title: payload.title.clone(),
        message: payload.message.clone(),
        timestamp: now,
    };

    // Show native desktop notification on Hyprland
    let _ = Notification::new()
        .summary(&format!("📱 {} • {}", payload.app, payload.title))
        .body(&payload.message)
        .icon("phone-apple-iphone")
        .appname("SynapseDrop")
        .show();

    let _ = state.event_tx.send(IpcEvent::NotificationReceived(notification_event));

    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn handle_get_autolink(State(state): State<Arc<AppState>>) -> Json<AutolinkStatusDto> {
    let connected = *state.autolink_connected.read().await;
    let battery = state.battery_state.read().await.clone();
    Json(AutolinkStatusDto { connected, battery })
}

async fn handle_post_autolink_ping(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    {
        let mut conn = state.autolink_connected.write().await;
        *conn = true;
    }
    let _ = state.event_tx.send(IpcEvent::AutolinkStatus {
        connected: true,
        ip: None,
        device_name: "iPhone".to_string(),
    });
    (StatusCode::OK, Json(serde_json::json!({ "status": "pong" })))
}

// =========================================================================
// MAX FLUTTER UI CONTROL ENDPOINTS
// =========================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaxSendRequest {
    pub target_ip: String,
    pub target_port: Option<u16>,
    pub protocol: Option<String>,
    pub files: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaxRespondRequest {
    pub session_id: String,
    pub accept: bool,
    #[serde(default)]
    pub trust_always: bool,
}

async fn handle_max_peers(State(state): State<Arc<AppState>>) -> Json<Vec<PeerDevice>> {
    let peers = state.peers.read().await;
    let list: Vec<PeerDevice> = peers.values().cloned().collect();
    Json(list)
}

async fn handle_max_send(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MaxSendRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let state_sender = Arc::clone(&state);
    let port = payload.target_port.unwrap_or(53317);
    let proto = payload.protocol.unwrap_or_else(|| "https".to_string());
    let paths: Vec<PathBuf> = payload.files.into_iter().map(PathBuf::from).collect();
    let target_ip = payload.target_ip.clone();

    // Abort any previous active send task
    if let Some(prev) = state.active_send_abort.write().await.take() {
        prev.abort();
    }

    let join_handle = tokio::spawn(async move {
        if let Err(e) = crate::protocol::client::send_files(
            Arc::clone(&state_sender),
            target_ip.clone(),
            port,
            proto,
            paths,
        )
        .await
        {
            error!("MAX failed to send files: {}", e);
            let _ = state_sender.event_tx.send(IpcEvent::TransferCompleted {
                session_id: "send_err".to_string(),
                success: false,
                message: "La transferencia fallo".to_string(),
                file_path: None,
                folder_path: None,
                is_incoming: false,
                file_count: 0,
                total_file_count: 0,
                is_partial: false,
            });
        }
    });

    *state.active_send_abort.write().await = Some(join_handle.abort_handle());

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "sending_started" })),
    )
}

async fn handle_max_respond(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MaxRespondRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut sessions = state.active_sessions.write().await;
    if let Some(session) = sessions.get_mut(&payload.session_id) {
        if payload.accept && payload.trust_always {
            let alias = session.sender_info.alias.trim().to_string();
            let target_name = if !alias.is_empty() {
                alias
            } else {
                session.sender_info.fingerprint.clone()
            };
            let mut cfg = state.config.write().await;
            if !cfg.trusted_devices.contains(&target_name) {
                cfg.trusted_devices.push(target_name);
            }
            let _ = cfg.save();
            let updated = cfg.clone();
            let _ = state.event_tx.send(IpcEvent::ConfigUpdated(updated));
        }
        if let Some(tx) = session.response_tx.take() {
            let _ = tx.send(payload.accept);
            return (
                StatusCode::OK,
                Json(serde_json::json!({ "status": "responded", "accepted": payload.accept })),
            );
        }
    }
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "Session not found" })),
    )
}

async fn handle_max_get_config(State(state): State<Arc<AppState>>) -> Json<AppConfig> {
    let cfg = state.config.read().await;
    Json(cfg.clone())
}

async fn handle_max_post_config(
    State(state): State<Arc<AppState>>,
    Json(new_cfg): Json<AppConfig>,
) -> (StatusCode, Json<serde_json::Value>) {
    new_cfg.update_autostart_entry();
    if let Err(e) = new_cfg.save() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to save config: {}", e) })),
        );
    }

    *state.config.write().await = new_cfg.clone();
    let _ = state.event_tx.send(IpcEvent::ConfigUpdated(new_cfg));

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "updated" })),
    )
}

#[derive(Deserialize)]
struct WhitelistRemoveRequest {
    identifier: String,
}

async fn handle_max_remove_whitelist(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WhitelistRemoveRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut cfg = state.config.write().await;
    cfg.trusted_devices.retain(|item| item != &req.identifier);
    let _ = cfg.save();
    let updated = cfg.clone();
    let _ = state.event_tx.send(IpcEvent::ConfigUpdated(updated));

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "removed" })),
    )
}

async fn handle_max_get_history(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<crate::state::TransferHistoryItem>> {
    let history = state.transfer_history.read().await;
    Json(history.clone())
}

async fn handle_max_clear_history(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    state.clear_history().await;
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "cleared" })),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaxOpenPathRequest {
    pub path: String,
    #[serde(default)]
    pub select: bool,
}

async fn handle_max_open_path(
    Json(payload): Json<MaxOpenPathRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let target = std::path::Path::new(&payload.path);

    #[cfg(target_os = "windows")]
    {
        if payload.select {
            let _ = std::process::Command::new("explorer.exe")
                .arg(format!("/select,\"{}\"", payload.path))
                .spawn();
        } else {
            let _ = std::process::Command::new("cmd")
                .args(&["/C", "start", "", &payload.path])
                .spawn();
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if payload.select {
            let dir = if target.is_dir() {
                target
            } else {
                target.parent().unwrap_or(target)
            };
            let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
        } else {
            let _ = std::process::Command::new("xdg-open").arg(&payload.path).spawn();
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "opened" })),
    )
}

async fn handle_max_ws_events(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_max_events_socket(socket, state))
}

async fn handle_max_events_socket(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let mut rx = state.event_tx.subscribe();

    // Send initial peers snapshot
    {
        let peers = state.peers.read().await;
        let peers_list: Vec<PeerDevice> = peers.values().cloned().collect();
        let initial_evt = IpcEvent::PeersUpdated(peers_list);
        if let Ok(json) = serde_json::to_string(&initial_evt) {
            let _ = socket.send(axum::extract::ws::Message::Text(json)).await;
        }
    }

    loop {
        tokio::select! {
            res = rx.recv() => {
                match res {
                    Ok(evt) => {
                        if let Ok(json) = serde_json::to_string(&evt) {
                            if socket.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                    Err(_) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;
    use futures_util::stream;
    use std::convert::Infallible;

    #[tokio::test]
    async fn streams_multipart_upload_with_split_boundary_across_chunks() {
        let temp_dir = std::env::temp_dir().join(format!("max_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let dest = temp_dir.join("out.png");

        let boundary = "----WebKitBoundaryTest123";
        let payload: Vec<u8> = b"\x89PNG\r\n\x1a\nhello\r\n--not-end\r\nworld".to_vec();
        let mut full_body = Vec::new();
        full_body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"photo.png\"\r\nContent-Type: image/png\r\n\r\n"
            )
            .as_bytes(),
        );
        full_body.extend_from_slice(&payload);
        full_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        // Split into tiny 5-byte chunks so headers and closing boundary cross chunk boundaries.
        let chunks: Vec<Result<Bytes, Infallible>> = full_body
            .chunks(5)
            .map(|c| Ok(Bytes::copy_from_slice(c)))
            .collect();

        let content_type = format!("multipart/form-data; boundary=\"{boundary}\"");
        let res = stream_direct_upload_to_file(stream::iter(chunks), &content_type, &dest)
            .await
            .expect("streaming multipart upload should succeed");

        assert_eq!(res.multipart_filename.as_deref(), Some("photo.png"));
        assert_eq!(res.bytes_written, payload.len() as u64);
        assert_eq!(
            detect_extension_from_magic_bytes(&res.magic_prefix[..res.magic_len]),
            ("png", "Image")
        );
        assert_eq!(std::fs::read(&dest).unwrap(), payload);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn streams_raw_binary_upload_without_multipart() {
        let temp_dir = std::env::temp_dir().join(format!("max_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let dest = temp_dir.join("raw.pdf");

        let payload = b"%PDF-1.7\n--some comment inside raw stream\n%%EOF".to_vec();
        let chunks: Vec<Result<Bytes, Infallible>> = payload
            .chunks(7)
            .map(|c| Ok(Bytes::copy_from_slice(c)))
            .collect();

        let res = stream_direct_upload_to_file(
            stream::iter(chunks),
            "application/pdf",
            &dest,
        )
        .await
        .expect("streaming raw upload should succeed");

        assert_eq!(res.multipart_filename, None);
        assert_eq!(res.bytes_written, payload.len() as u64);
        assert_eq!(
            detect_extension_from_magic_bytes(&res.magic_prefix[..res.magic_len]),
            ("pdf", "Document")
        );
        assert_eq!(std::fs::read(&dest).unwrap(), payload);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn streams_short_raw_payload_starting_with_dashes() {
        let temp_dir = std::env::temp_dir().join(format!("max_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let dest = temp_dir.join("short.txt");

        let payload = b"-- markdown horizontal rule\n\nhello".to_vec();
        let chunks: Vec<Result<Bytes, Infallible>> = vec![Ok(Bytes::from(payload.clone()))];

        let res = stream_direct_upload_to_file(
            stream::iter(chunks),
            "text/plain",
            &dest,
        )
        .await
        .expect("short raw upload starting with -- should not be parsed as multipart");

        assert_eq!(res.multipart_filename, None);
        assert_eq!(res.bytes_written, payload.len() as u64);
        assert_eq!(std::fs::read(&dest).unwrap(), payload);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}


