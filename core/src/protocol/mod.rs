pub mod discovery;
pub mod server;
pub mod client;
pub mod tls;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Device types according to LocalSend specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Mobile,
    Desktop,
    Web,
    Headless,
    Server,
}

impl Default for DeviceType {
    fn default() -> Self {
        DeviceType::Desktop
    }
}

/// Device info model for discovery and registration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDto {
    pub alias: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    #[serde(default)]
    pub device_type: DeviceType,
    pub fingerprint: String,
    pub port: u16,
    pub protocol: String, // "http" or "https"
    pub download: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announcement: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announce: Option<bool>,
}

/// Discovered peer representation in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerDevice {
    pub ip: String,
    pub port: u16,
    pub alias: String,
    pub device_model: Option<String>,
    pub device_type: DeviceType,
    pub fingerprint: String,
    pub protocol: String,
    pub download: bool,
    pub last_seen: u64,
}

/// File metadata sent in prepare-upload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDto {
    pub id: String,
    pub file_name: String,
    pub size: u64,
    pub file_type: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub preview: Option<String>,
}

/// Prepare-upload request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareUploadRequestDto {
    pub info: DeviceDto,
    pub files: HashMap<String, FileDto>,
}

/// Prepare-upload response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareUploadResponseDto {
    pub session_id: String,
    pub files: HashMap<String, String>, // fileId -> token
}

/// Transfer state for IPC and UI feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub session_id: String,
    pub file_id: String,
    pub file_name: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: f64,
    pub is_finished: bool,
    pub is_error: bool,
    pub error_message: Option<String>,
    #[serde(default)]
    pub files_completed: usize,
    #[serde(default = "default_one_file")]
    pub total_files: usize,
}

fn default_one_file() -> usize {
    1
}
