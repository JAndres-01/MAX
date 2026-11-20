pub mod autolink;
pub mod bluetooth;
pub mod clipboard;
pub mod config;
#[cfg(unix)]
pub mod ipc;
pub mod mdns;
pub mod protocol;
pub mod pwa;
pub mod state;

use config::AppConfig;
use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

static ENGINE_RUNNING: AtomicBool = AtomicBool::new(false);

/// Starts the MAX LocalSend v2 engine inside a background Tokio runtime thread.
/// Safe to call from Flutter via `dart:ffi` (`DynamicLibrary.open`).
#[no_mangle]
pub extern "C" fn max_core_start() -> i32 {
    if ENGINE_RUNNING.swap(true, Ordering::SeqCst) {
        return 0; // Already running
    }

    std::thread::spawn(|| {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(4)
            .thread_name("max-core-worker")
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(_) => return,
        };

        rt.block_on(async {
            let _ = rustls::crypto::ring::default_provider().install_default();

            let mut config = AppConfig::load();
            AppConfig::ensure_sound_assets();
            if config.alias == "Synapse Drop" || config.alias.is_empty() {
                config.alias = "MAX".to_string();
            }
            config.port = 53317;

            let cert_dir = AppConfig::config_path()
                .parent()
                .unwrap_or(&std::path::PathBuf::from("/tmp"))
                .join("certs");
            if let Ok(tls_cfg) = protocol::tls::get_or_create_tls_certs(cert_dir) {
                if config.protocol == "https" {
                    config.fingerprint = tls_cfg.fingerprint;
                }
            }

            let state = Arc::new(AppState::new(config));

            // Start mDNS / Bonjour (_localsend._tcp) & Continuity Autolink watchers
            autolink::spawn_autolink_watcher(state.clone());
            mdns::spawn_mdns_publisher(state.clone());
            mdns::spawn_mdns_browser(state.clone());

            // Start LocalSend v2 HTTPS/HTTP Server (port 53317) + MAX UI Control Server (port 53320)
            if let Err(e) = protocol::server::start_server(state.clone()).await {
                eprintln!("MAX Core Error starting server: {:?}", e);
            }

            // Start Multicast UDP + Active Subnet Discovery (port 53317)
            if let Err(e) = protocol::discovery::start_discovery(state.clone()).await {
                eprintln!("MAX Core Error starting discovery: {:?}", e);
            }

            // Keep runtime alive indefinitely while app is open
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    });

    1
}
