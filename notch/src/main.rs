//! MAX Native Notch — Living Top Notch with Nusa-style micro-interactions.
//! Phase 1: LocalSend P2P Transfer & Receive module.

use std::sync::Arc;
use tracing::info;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("Starting MAX Native Notch (Nusa-style)");

    // Initialize LocalSend core state
    let config = max_core::config::AppConfig::load();
    let _state = Arc::new(max_core::state::AppState::new(config));

    let window = NotchWindow::new()?;
    window.set_peer_count(2);
    window.set_active_peer_name("Pixel 8 Pro".into());

    window.on_send_files({
        let win = window.as_weak();
        move || {
            if let Some(w) = win.upgrade() {
                info!("Sending files to peer: {}", w.get_active_peer_name());
            }
        }
    });

    window.on_clear_staged({
        let win = window.as_weak();
        move || {
            if let Some(w) = win.upgrade() {
                info!("Clearing staged files");
                w.set_staged_files(slint::ModelRc::new(slint::VecModel::default()));
            }
        }
    });

    window.run()?;
    Ok(())
}
