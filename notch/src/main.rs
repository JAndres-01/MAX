//! MAX Native Notch — Living Top Notch with Nusa-style micro-interactions.
//! Phase 1: LocalSend P2P Transfer & Receive module.

use std::sync::Arc;
use tracing::info;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("[MAX] Iniciando MAX Notch...");

    // Initialize LocalSend core runtime in a dedicated background thread pool
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let _core_handle = rt.spawn(async {
        let config = max_core::config::AppConfig::load();
        let _state = Arc::new(max_core::state::AppState::new(config));
        info!("MAX Core background engine active");
    });

    let window = NotchWindow::new()?;
    println!("[MAX] Ventana creada con éxito. Mostrando en pantalla...");

    // Initial mock data to preview Nusa aesthetics
    window.set_peer_count(3);
    window.set_active_peer_name("Pixel 8 Pro (LocalSend)".into());
    window.set_is_expanded(true);

    window.on_send_files({
        let win = window.as_weak();
        move || {
            if let Some(w) = win.upgrade() {
                println!("[MAX] Click en Enviar hacia: {}", w.get_active_peer_name());
            }
        }
    });

    window.on_clear_staged({
        let win = window.as_weak();
        move || {
            if let Some(w) = win.upgrade() {
                println!("[MAX] Cola de archivos limpiada");
                w.set_staged_files(slint::ModelRc::new(slint::VecModel::default()));
            }
        }
    });

    window.run()?;
    Ok(())
}
