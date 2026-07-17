//! Look-and-feel preferences for the settings window.
//!
//! Deliberately *not* part of `config.toml`. The resident process watches that
//! file's mtime and reacts to any change by re-sending `ApplyDeviceConfig` +
//! `ApplyControlDiverts` to the mouse — so persisting the theme there would
//! push HID++ traffic to the device every time someone toggled dark/light.
//! This file is owned by the settings process alone.

use super::theme::{Accent, Theme};
use crate::config::app_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
#[serde(default)]
pub struct UiPrefs {
    pub theme: Theme,
    pub accent: Accent,
}

fn path() -> Option<PathBuf> {
    app_dir().ok().map(|d| d.join("ui.toml"))
}

impl UiPrefs {
    pub fn load() -> Self {
        let Some(p) = path() else {
            return Self::default();
        };
        std::fs::read_to_string(p)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        match toml::to_string_pretty(self) {
            Ok(s) => {
                if let Err(e) = std::fs::write(p, s) {
                    tracing::warn!("saving ui prefs failed: {e}");
                }
            }
            Err(e) => tracing::warn!("serialising ui prefs failed: {e}"),
        }
    }
}
