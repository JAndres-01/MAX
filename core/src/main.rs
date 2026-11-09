#[cfg(unix)]
mod autolink;
#[cfg(unix)]
mod bluetooth;
#[cfg(unix)]
mod clipboard;
#[cfg(unix)]
mod config;
#[cfg(unix)]
mod ipc;
#[cfg(unix)]
mod mdns;
#[cfg(unix)]
mod protocol;
#[cfg(unix)]
mod pwa;
#[cfg(unix)]
mod state;

use clap::{Parser, Subcommand};
use config::AppConfig;
use ipc::{IpcCommand, DEFAULT_SOCKET_PATH};
use state::AppState;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "synapse-drop")]
#[command(about = "Native LocalSend v2 daemon and CLI for Hyprland/CachyOS", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the LocalSend background daemon (Multicast Discovery + REST API + IPC)
    Daemon,
    /// Get current daemon status
    Status,
    /// List active peers discovered on local network
    List,
    /// Get current iPhone battery status
    Battery,
    /// Get autonomous connection / autolink status
    Autolink,
    /// Control the offline Wi-Fi Direct Hotspot (on, off, toggle, status)
    Hotspot {
        /// Action: on, off, toggle, status
        action: Option<String>,
    },
    /// Send files to a peer
    Send {
        /// Target peer IP address
        target_ip: String,
        /// Target port (default: 53317)
        #[arg(short, long)]
        port: Option<u16>,
        /// Protocol (https or http)
        #[arg(long)]
        protocol: Option<String>,
        /// Files to send
        #[arg(required = true)]
        files: Vec<String>,
    },
    /// Send plain text or message to a peer
    Text {
        /// Target peer IP address
        target_ip: String,
        /// Target port (default: 53317)
        #[arg(short, long)]
        port: Option<u16>,
        /// Protocol (https or http)
        #[arg(long)]
        protocol: Option<String>,
        /// Text message to send
        text: String,
    },
    /// Accept an incoming transfer request
    Accept {
        session_id: String,
    },
    /// Reject an incoming transfer request
    Reject {
        session_id: String,
    },
    /// Explore local directory structure and return JSON (Internal Explorer)
    Explore {
        /// Target folder path (default: Downloads or Home)
        path: Option<String>,
    },
    /// Set auto-accept incoming transfers
    SetAutoAccept {
        enabled: bool,
    },
    /// Set custom alias name
    SetAlias {
        alias: String,
    },
    /// Pick files via system dialog and send to target peer
    PickSend {
        /// Target peer IP address
        target_ip: String,
        /// Target port (default: 53317)
        #[arg(short, long)]
        port: Option<u16>,
        /// Protocol (https or http)
        #[arg(long)]
        protocol: Option<String>,
    },
    /// Toggle QuickShell Drop popup in top-bar
    Toggle,
}

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Daemon) {
        Commands::Daemon => run_daemon().await?,
        Commands::Status => send_client_command(IpcCommand::GetStatus).await?,
        Commands::List => send_client_command(IpcCommand::GetPeers).await?,
        Commands::Battery => send_client_command(IpcCommand::GetBattery).await?,
        Commands::Autolink => send_client_command(IpcCommand::GetAutolinkStatus).await?,
        Commands::Hotspot { action } => handle_hotspot_command(action).await?,
        Commands::Explore { path } => send_client_command(IpcCommand::ExploreDir { path }).await?,
        Commands::Send {
            target_ip,
            port,
            protocol,
            files,
        } => {
            send_client_command(IpcCommand::SendFiles {
                target_ip,
                target_port: port,
                protocol,
                files,
            })
            .await?;
        }
        Commands::Text {
            target_ip,
            port,
            protocol,
            text,
        } => {
            send_client_command(IpcCommand::SendText {
                target_ip,
                target_port: port,
                protocol,
                text,
            })
            .await?;
        }
        Commands::PickSend {
            target_ip,
            port,
            protocol,
        } => {
            run_pick_and_send(target_ip, port, protocol).await?;
        }
        Commands::Accept { session_id } => {
            send_client_command(IpcCommand::AcceptTransfer { session_id }).await?;
        }
        Commands::Reject { session_id } => {
            send_client_command(IpcCommand::RejectTransfer { session_id }).await?;
        }
        Commands::SetAutoAccept { enabled } => {
            send_client_command(IpcCommand::SetAutoAccept { enabled }).await?;
        }
        Commands::SetAlias { alias } => {
            send_client_command(IpcCommand::SetAlias { alias }).await?;
        }
        Commands::Toggle => {
            // Trigger QuickShell toggle via serpantinum manager
            let _ = tokio::process::Command::new("serpantinum")
                .args(["msg", "toggle", "airdrop"])
                .spawn();
        }
    }

    Ok(())
}

async fn run_pick_and_send(
    target_ip: String,
    port: Option<u16>,
    protocol: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("zenity")
        .args(["--file-selection", "--multiple", "--separator=\n", "--title=Select files to send with SynapseDrop"])
        .output()
        .await;

    let files: Vec<String> = match output {
        Ok(out) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout);
            s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
        }
        _ => {
            let kd_output = tokio::process::Command::new("kdialog")
                .args(["--getopenfilename", "--multiple", "--separate-output", ".", "All Files (*)"])
                .output()
                .await;
            match kd_output {
                Ok(out) if out.status.success() => {
                    let s = String::from_utf8_lossy(&out.stdout);
                    s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
                }
                _ => return Ok(()),
            }
        }
    };

    if files.is_empty() {
        return Ok(());
    }

    send_client_command(IpcCommand::SendFiles {
        target_ip,
        target_port: port,
        protocol,
        files,
    })
    .await?;

    Ok(())
}

async fn run_daemon() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if UnixStream::connect(DEFAULT_SOCKET_PATH).await.is_ok() {
        println!("SynapseDrop daemon is already running in background.");
        return Ok(());
    }

    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "synapse_drop=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut config = AppConfig::load();
    AppConfig::ensure_sound_assets();
    let cert_dir = AppConfig::config_path()
        .parent()
        .unwrap_or(&std::path::PathBuf::from("/tmp"))
        .join("certs");
    if let Ok(tls_cfg) = protocol::tls::get_or_create_tls_certs(cert_dir) {
        if config.protocol == "https" {
            config.fingerprint = tls_cfg.fingerprint;
            let _ = config.save();
        }
    }
    let state = Arc::new(AppState::new(config));

    // 0. Start Wayland Clipboard Sync Watcher
    clipboard::spawn_clipboard_watcher(state.clone());

    // 1. Start Bluetooth BLE Battery Telemetry Watcher
    bluetooth::spawn_bluetooth_watcher(state.clone());

    // 2. Start Autonomous Continuity Autolink Watcher
    autolink::spawn_autolink_watcher(state.clone());

    // 2b. Start Dynamic mDNS Publisher & Browser
    mdns::spawn_mdns_publisher(state.clone());
    mdns::spawn_mdns_browser(state.clone());

    // 3. Start Multicast UDP Discovery
    protocol::discovery::start_discovery(state.clone()).await?;

    // 4. Start HTTP/HTTPS REST Server
    protocol::server::start_server(state.clone()).await?;

    // 5. Start Unix Domain Socket IPC for QuickShell
    ipc::start_ipc_server(state.clone()).await?;

    tracing::info!("SynapseDrop Daemon is running smoothly. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down SynapseDrop Daemon...");

    Ok(())
}

async fn send_client_command(cmd: IpcCommand) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stream = match UnixStream::connect(DEFAULT_SOCKET_PATH).await {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error: SynapseDrop daemon is not running. Start it with 'synapse-drop daemon'");
            std::process::exit(1);
        }
    };

    let (reader, mut writer) = stream.into_split();
    let json = serde_json::to_string(&cmd)?;
    writer.write_all(format!("{}\n", json).as_bytes()).await?;

    let mut lines = BufReader::new(reader).lines();
    if let Some(resp) = lines.next_line().await? {
        println!("{}", resp);
    }

    Ok(())
}

async fn handle_hotspot_command(action: Option<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut act = action.unwrap_or_else(|| "status".to_string()).to_lowercase();

    if act == "toggle" {
        let active = is_hotspot_active().await;
        act = if active { "off".to_string() } else { "on".to_string() };
    }

    match act.as_str() {
        "on" | "start" | "enable" => {
            println!("Starting Hyprand offline Wi-Fi hotspot...");
            let out = tokio::process::Command::new("nmcli")
                .args(["connection", "up", "Hyprand"])
                .output()
                .await?;
            if out.status.success() {
                println!("Hyprand hotspot is ACTIVE! (SSID: Hyprand, Key: synapsedrop123, IP: 10.42.0.1)");
            } else {
                eprintln!("Failed to start hotspot: {}", String::from_utf8_lossy(&out.stderr));
            }
        }
        "off" | "stop" | "disable" => {
            println!("Stopping Hyprand offline Wi-Fi hotspot...");
            let out = tokio::process::Command::new("nmcli")
                .args(["connection", "down", "Hyprand"])
                .output()
                .await?;
            if out.status.success() {
                println!("Hyprand hotspot stopped successfully. Reconnecting to local Wi-Fi...");
                // Tell NetworkManager to immediately reconnect to available known Wi-Fi networks
                let _ = tokio::process::Command::new("nmcli")
                    .args(["device", "connect", "wlan0"])
                    .output()
                    .await;
            } else {
                eprintln!("Failed to stop hotspot: {}", String::from_utf8_lossy(&out.stderr));
            }
        }
        _ => {
            let active = is_hotspot_active().await;
            if active {
                println!("Hyprand offline hotspot: ACTIVE (SSID: Hyprand, IP: 10.42.0.1)");
            } else {
                println!("Hyprand offline hotspot: INACTIVE");
            }
        }
    }

    Ok(())
}

async fn is_hotspot_active() -> bool {
    if let Ok(out) = tokio::process::Command::new("nmcli")
        .args(["-t", "-f", "NAME,STATE", "connection", "show", "--active"])
        .output()
        .await
    {
        let s = String::from_utf8_lossy(&out.stdout);
        s.lines().any(|l| l.starts_with("Hyprand:"))
    } else {
        false
    }
}

#[cfg(not(unix))]
fn main() {
    println!("MAX Core runs as an embedded dynamic library (max_core.dll) on Windows.");
}


