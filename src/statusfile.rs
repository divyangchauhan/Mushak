//! Tiny file-based IPC: the resident process publishes live device status here
//! and the settings subprocess reads it (the settings process has no HID
//! thread of its own).

use crate::config::app_dir;
use crate::hidpp::device::DeviceStatus;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SharedStatus {
    pub device: DeviceStatus,
    pub paused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_pid: Option<u32>,
}

fn path() -> Option<PathBuf> {
    app_dir().ok().map(|d| d.join("status.json"))
}

pub fn write(status: &SharedStatus) {
    let Some(p) = path() else { return };
    if let Ok(json) = serde_json::to_string(status) {
        let _ = std::fs::write(p, json);
    }
}

pub fn read() -> SharedStatus {
    read_update().unwrap_or_default()
}

/// A partially written status must not look like a disconnected mouse or a confirmed resume.
pub fn read_update() -> Option<SharedStatus> {
    let p = path()?;
    let data = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(&data).ok()
}
