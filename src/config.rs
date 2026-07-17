//! Configuration model + TOML persistence.
//!
//! The config is intentionally serialization-friendly: every enum uses either
//! unit variants (serialized as bare strings) or an internally-tagged `kind`
//! field so the resulting `config.toml` stays human-editable.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Keyboard modifier keys that can accompany a remapped action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Win,
}

/// What a button / gesture outcome does when triggered.
///
/// Every concrete action ultimately reduces to a synthesized key chord, so a
/// single `Key` variant models plain shortcuts, media keys, volume keys,
/// virtual-desktop switches and task view. `mods` may be empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Action {
    /// Let the original OS event through untouched.
    PassThrough,
    /// Swallow the event and do nothing.
    Disabled,
    /// Synthesize a key chord (`mods` held while `vk` is tapped).
    Key {
        #[serde(default)]
        mods: Vec<Modifier>,
        /// Windows virtual-key code (e.g. 0x57 == 'W').
        vk: u16,
    },
}

impl Action {
    pub fn key(mods: &[Modifier], vk: u16) -> Self {
        Action::Key {
            mods: mods.to_vec(),
            vk,
        }
    }

    /// True when this action replaces the native event (so the up-event of a
    /// physical button should also be swallowed).
    pub fn intercepts(&self) -> bool {
        !matches!(self, Action::PassThrough)
    }
}

impl Default for Action {
    fn default() -> Self {
        Action::PassThrough
    }
}

/// The three hook-remappable physical buttons on the MX Master 2S.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Side "back" button (XBUTTON1).
    Back,
    /// Side "forward" button (XBUTTON2).
    Forward,
    /// Wheel / middle click.
    Middle,
}

/// Actions for the hook-remappable buttons within a single profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ButtonMap {
    pub back: Action,
    pub forward: Action,
    pub middle: Action,
}

impl Default for ButtonMap {
    fn default() -> Self {
        ButtonMap {
            back: Action::PassThrough,
            forward: Action::PassThrough,
            middle: Action::PassThrough,
        }
    }
}

impl ButtonMap {
    pub fn get(&self, button: MouseButton) -> &Action {
        match button {
            MouseButton::Back => &self.back,
            MouseButton::Forward => &self.forward,
            MouseButton::Middle => &self.middle,
        }
    }
}

/// A named profile. The default profile has an empty `match_processes`; app
/// profiles list one or more executable basenames (case-insensitive, e.g.
/// `"chrome.exe"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub match_processes: Vec<String>,
    #[serde(default)]
    pub buttons: ButtonMap,
}

impl Profile {
    pub fn matches(&self, process: &str) -> bool {
        let p = process.to_ascii_lowercase();
        self.match_processes
            .iter()
            .any(|m| m.to_ascii_lowercase() == p)
    }
}

/// SmartShift wheel behavior (HID++ feature 0x2110).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartShiftMode {
    /// Permanent ratchet (`autoDisengage = 255`).
    AlwaysRatchet,
    /// Permanent freespin (`wheelMode = freespin`).
    AlwaysFreespin,
    /// Auto-disengage based on a sensitivity threshold.
    SmartShift,
}

impl Default for SmartShiftMode {
    fn default() -> Self {
        SmartShiftMode::SmartShift
    }
}

/// Device-side settings applied over HID++ on launch / reconnect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    pub smartshift: SmartShiftMode,
    /// Auto-disengage threshold 1..=30 (only used in `SmartShift` mode).
    pub smartshift_threshold: u8,
    /// High-resolution (smooth) scrolling on/off.
    pub hires_scroll: bool,
    /// Invert vertical scroll direction.
    pub invert_scroll: bool,
    /// Resolution in DPI (validated against the device's supported list).
    pub dpi: u16,
    /// How far the wheel must move, in high-resolution increments, before a
    /// scroll counts as deliberate. The wheel reports 8 increments per detent,
    /// and reports them between detents too, so a finger merely resting on it
    /// rocks it a unit or two and scrolls the page. Movement below this is held
    /// back until it adds up; once scrolling starts it runs freely until the
    /// wheel goes still, so a real scroll is never clipped.
    ///
    /// 0 disables the deadzone. Only applies while `hires_scroll` is on — that
    /// is the only mode where the wheel reports to us rather than straight to
    /// Windows.
    pub scroll_deadzone: u8,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        DeviceConfig {
            smartshift: SmartShiftMode::SmartShift,
            smartshift_threshold: 10,
            hires_scroll: true,
            invert_scroll: false,
            dpi: 1000,
            // Half a detent: an accidental nudge is a unit or two, a deliberate
            // detent is a full 8 and so still scrolls on the very first click.
            scroll_deadzone: 4,
        }
    }
}

/// Software gesture configuration (thumb button, HID++ 0x1B04 divert).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GestureConfig {
    /// Master enable for the gesture button divert.
    pub enabled: bool,
    /// Movement (in accumulated raw units) below which a press counts as a tap.
    pub tap_threshold: i32,
    pub tap: Action,
    pub up: Action,
    pub down: Action,
    pub left: Action,
    pub right: Action,
}

impl Default for GestureConfig {
    fn default() -> Self {
        // Options+-like defaults: tap = task view, left/right = switch desktop,
        // up/down = volume.
        GestureConfig {
            enabled: true,
            tap_threshold: 60,
            tap: Action::key(&[Modifier::Win], vk::TAB),
            up: Action::key(&[], vk::VOLUME_UP),
            down: Action::key(&[], vk::VOLUME_DOWN),
            left: Action::key(&[Modifier::Win, Modifier::Ctrl], vk::LEFT),
            right: Action::key(&[Modifier::Win, Modifier::Ctrl], vk::RIGHT),
        }
    }
}

/// Top-level persisted configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub default_profile: Profile,
    pub profiles: Vec<Profile>,
    pub device: DeviceConfig,
    pub gestures: GestureConfig,
    pub start_with_windows: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_profile: Profile {
                name: "Default".to_string(),
                match_processes: Vec::new(),
                buttons: ButtonMap::default(),
            },
            profiles: Vec::new(),
            device: DeviceConfig::default(),
            gestures: GestureConfig::default(),
            start_with_windows: false,
        }
    }
}

/// `%APPDATA%\Mushak` (created lazily by callers that write into it).
pub fn app_dir() -> Result<PathBuf> {
    let appdata = std::env::var_os("APPDATA").context("APPDATA env var not set")?;
    Ok(PathBuf::from(appdata).join("Mushak"))
}

impl Config {
    /// `%APPDATA%\Mushak\config.toml`.
    pub fn path() -> Result<PathBuf> {
        Ok(app_dir()?.join("config.toml"))
    }

    /// Load from disk, falling back to defaults if the file is missing.
    pub fn load() -> Config {
        match Self::try_load() {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!("using default config ({e:#})");
                Config::default()
            }
        }
    }

    fn try_load() -> Result<Config> {
        let path = Self::path()?;
        if !path.exists() {
            tracing::info!("no config at {}, writing defaults", path.display());
            let cfg = Config::default();
            cfg.save()?;
            return Ok(cfg);
        }
        Self::load_strict()
    }

    /// Parse the config file, erroring (rather than falling back to defaults) on
    /// any problem. Used by the resident's file watcher so a partially-written
    /// file from the settings process is retried, not clobbered.
    pub fn load_strict() -> Result<Config> {
        let path = Self::path()?;
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let cfg: Config = toml::from_str(&text).context("parsing config.toml")?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        tracing::info!("saved config to {}", path.display());
        Ok(())
    }
}

/// A tiny set of Windows virtual-key codes used by the default config and GUI
/// presets. Kept here so `config.rs` has no `windows` dependency.
pub mod vk {
    pub const TAB: u16 = 0x09;
    pub const ENTER: u16 = 0x0D;
    pub const ESCAPE: u16 = 0x1B;
    pub const LEFT: u16 = 0x25;
    pub const UP: u16 = 0x26;
    pub const RIGHT: u16 = 0x27;
    pub const DOWN: u16 = 0x28;

    pub const VOLUME_MUTE: u16 = 0xAD;
    pub const VOLUME_DOWN: u16 = 0xAE;
    pub const VOLUME_UP: u16 = 0xAF;
    pub const MEDIA_NEXT: u16 = 0xB0;
    pub const MEDIA_PREV: u16 = 0xB1;
    pub const MEDIA_STOP: u16 = 0xB2;
    pub const MEDIA_PLAY_PAUSE: u16 = 0xB3;

    pub const BROWSER_BACK: u16 = 0xA6;
    pub const BROWSER_FORWARD: u16 = 0xA7;

    /// 'A'..='Z' == 0x41..=0x5A, '0'..='9' == 0x30..=0x39.
    pub const fn letter(c: u8) -> u16 {
        c.to_ascii_uppercase() as u16
    }
}
