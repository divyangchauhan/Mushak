//! Remember identity only. Connection state and capabilities always come from discovery.

use super::device::DeviceStatus;
use std::path::PathBuf;

pub struct DeviceHistory {
    path: Option<PathBuf>,
    name: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RememberedDevice {
    name: String,
}

impl DeviceHistory {
    pub fn load() -> Self {
        Self::from_path(
            crate::config::app_dir()
                .ok()
                .map(|d| d.join("last-device.json")),
        )
    }

    fn from_path(path: Option<PathBuf>) -> Self {
        let name = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| serde_json::from_slice::<RememberedDevice>(&bytes).ok())
            .map(|device| device.name)
            .filter(|name| !name.trim().is_empty());
        Self { path, name }
    }

    pub fn disconnected(&self) -> DeviceStatus {
        DeviceStatus {
            name: self.name.clone(),
            ..Default::default()
        }
    }

    pub fn observe(&mut self, status: DeviceStatus) -> DeviceStatus {
        if !status.connected {
            return self.disconnected();
        }
        let name = status
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("Mouse");
        if self.name.as_deref() != Some(name) {
            self.name = Some(name.to_owned());
            if let Some(path) = &self.path {
                let device = RememberedDevice {
                    name: name.to_owned(),
                };
                let result = serde_json::to_vec(&device)
                    .map_err(std::io::Error::other)
                    .and_then(|bytes| std::fs::write(path, bytes));
                if let Err(error) = result {
                    tracing::warn!("could not remember mouse identity: {error}");
                }
            }
        }
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnect_and_restart_keep_identity_without_live_settings() {
        let path = std::env::temp_dir().join(format!("mushak-history-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut history = DeviceHistory::from_path(Some(path.clone()));
        assert_eq!(history.disconnected().display_name(), "No device");
        let mut mouse = DeviceStatus {
            connected: true,
            name: Some("MX Vertical".into()),
            dpi_current: Some(1200),
            ..Default::default()
        };
        mouse.capabilities.dpi = true;
        assert!(history.observe(mouse).connected);
        let disconnected = history.observe(DeviceStatus::default());
        assert!(!disconnected.connected);
        assert_eq!(disconnected.display_name(), "MX Vertical");
        assert!(!disconnected.capabilities.dpi);
        assert!(disconnected.dpi_current.is_none());
        let mut restarted = DeviceHistory::from_path(Some(path.clone()));
        assert_eq!(restarted.disconnected().display_name(), "MX Vertical");
        assert!(!restarted.disconnected().connected);
        restarted.observe(DeviceStatus {
            connected: true,
            name: Some("MX Master 3S".into()),
            ..Default::default()
        });
        assert_eq!(
            DeviceHistory::from_path(Some(path.clone()))
                .disconnected()
                .display_name(),
            "MX Master 3S"
        );
        std::fs::write(&path, b"invalid JSON").unwrap();
        assert_eq!(
            DeviceHistory::from_path(Some(path.clone()))
                .disconnected()
                .display_name(),
            "No device"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unnamed_mouse_is_remembered_without_reusing_another_mouses_name() {
        let mut history = DeviceHistory::from_path(None);
        history.observe(DeviceStatus {
            connected: true,
            name: Some("MX Master".into()),
            ..Default::default()
        });
        history.observe(DeviceStatus {
            connected: true,
            ..Default::default()
        });
        assert_eq!(history.disconnected().display_name(), "Mouse");
    }
}
