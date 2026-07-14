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
    let Some(p) = path() else {
        return SharedStatus::default();
    };
    std::fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
