use crate::state::{AppState, ClipboardItem, IpcEvent};
use notify_rust::Notification;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tracing::{error, info, warn};
use uuid::Uuid;

#[cfg(target_os = "linux")]
fn prepare_wayland_command(cmd: &mut Command) {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        if let Ok(entries) = std::fs::read_dir("/run/user") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    return path.to_string_lossy().to_string();
                }
            }
        }
        "/run/user/1000".to_string()
    });
    cmd.env("XDG_RUNTIME_DIR", &runtime_dir);

    // Check if WAYLAND_DISPLAY in current env is valid
    let mut found_display = None;
    if let Ok(disp) = std::env::var("WAYLAND_DISPLAY") {
        if std::path::Path::new(&runtime_dir).join(&disp).exists() {
            found_display = Some(disp);
        }
    }

    // If missing or invalid, scan runtime directory for wayland-* sockets
    if found_display.is_none() {
        if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
            let mut sockets: Vec<String> = entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with("wayland-") && !name.ends_with(".lock") {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect();
            sockets.sort();
            if let Some(sock) = sockets.pop() {
                found_display = Some(sock);
            }
        }
    }

    if let Some(disp) = found_display {
        cmd.env("WAYLAND_DISPLAY", disp);
    }
}

#[cfg(target_os = "linux")]
pub async fn get_wayland_clipboard() -> Result<String, String> {
    let mut cmd = Command::new("wl-paste");
    cmd.args(["--no-newline"]);
    prepare_wayland_command(&mut cmd);

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run wl-paste: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn get_wayland_clipboard() -> Result<String, String> {
    Err("Direct clipboard polling not implemented for this OS".to_string())
}

#[cfg(target_os = "linux")]
pub async fn set_wayland_clipboard(text: &str) -> Result<(), String> {
    let mut cmd = Command::new("wl-copy");
    cmd.stdin(std::process::Stdio::piped());
    prepare_wayland_command(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn wl-copy: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        let _ = stdin.write_all(text.as_bytes()).await;
        let _ = stdin.flush().await;
    }

    let wait_res = tokio::time::timeout(Duration::from_millis(1500), child.wait()).await;
    match wait_res {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(format!("wl-copy exited with status: {}", status)),
        Ok(Err(e)) => Err(format!("wl-copy failed: {}", e)),
        Err(_) => {
            // Timeout elapsed, kill process to prevent hang
            let _ = child.kill().await;
            Ok(())
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn set_wayland_clipboard(_text: &str) -> Result<(), String> {
    Ok(())
}

pub async fn add_clipboard_entry(
    state: &Arc<AppState>,
    text: String,
    source: &str,
    device_name: Option<String>,
    notify_desktop: bool,
) -> ClipboardItem {
    let clean_text = text.trim().to_string();
    let is_url = clean_text.starts_with("http://") || clean_text.starts_with("https://");
    let item_type = if is_url { "url" } else { "text" }.to_string();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let item = ClipboardItem {
        id: Uuid::new_v4().to_string(),
        text: clean_text.clone(),
        item_type,
        timestamp: now,
        source: source.to_string(),
    };

    {
        let mut history = state.clipboard_history.write().await;
        if source == "pc" {
            // Avoid duplicate echoes from internal PC polling loop
            if let Some(top) = history.first() {
                if top.text == clean_text {
                    return item;
                }
            }
        }

        // For mobile pushes, move item to top of history even if sent again
        history.retain(|i| i.text != clean_text);
        history.insert(0, item.clone());
        if history.len() > 30 {
            history.truncate(30);
        }
    }

    let _ = state.clipboard_ws_tx.send(item.clone());
    let _ = state.event_tx.send(IpcEvent::ClipboardUpdated(item.clone()));

    if notify_desktop && source != "pc" {
        let dev = device_name.unwrap_or_else(|| "iPhone".to_string());
        let summary_title = format!("Enviado desde {}", dev);
        let preview = if clean_text.len() > 80 {
            format!("{}...", &clean_text[..80])
        } else {
            clean_text.clone()
        };

        let _ = Notification::new()
            .summary(&summary_title)
            .body(&preview)
            .icon("edit-copy")
            .timeout(notify_rust::Timeout::Milliseconds(4000))
            .show();
    }

    item
}

pub fn spawn_clipboard_watcher(state: Arc<AppState>) {
    #[cfg(target_os = "linux")]
    tokio::spawn(async move {
        let mut last_known_text = match get_wayland_clipboard().await {
            Ok(t) => t,
            Err(_) => String::new(),
        };

        if !last_known_text.is_empty() {
            add_clipboard_entry(&state, last_known_text.clone(), "pc", None, false).await;
        }

        loop {
            tokio::time::sleep(Duration::from_millis(600)).await;

            match get_wayland_clipboard().await {
                Ok(current_text) => {
                    if !current_text.is_empty() && current_text != last_known_text {
                        last_known_text = current_text.clone();
                        add_clipboard_entry(&state, current_text, "pc", None, false).await;
                    }
                }
                Err(_) => {
                    // Wayland compositor temporarily busy or locked
                }
            }
        }
    });
    #[cfg(not(target_os = "linux"))]
    {
        let _ = state;
    }
}
