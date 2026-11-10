//! MAX Native Notch — Living Top Notch with Nusa-style micro-interactions.
//! Phase 1: LocalSend P2P Transfer & Receive module.

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("Starting MAX Native Notch (Nusa-style)");

    // Initialize the core engine config and state
    let config = max_core::config::AppConfig::load();
    let state = Arc::new(max_core::state::AppState::new(config));
    
    let alias = state.config.read().await.alias.clone();
    info!("Engine initialized with device alias: {}", alias);

    Ok(())
}
