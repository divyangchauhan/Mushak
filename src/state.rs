//! Process-global shared state.
//!
//! The mouse-hook callback runs on a dedicated thread and must read the active
//! mapping and paused flag without blocking, so those live in lock-free
//! primitives (`ArcSwap`, `AtomicBool`). Config edits from the GUI go through
//! [`set_config`], which recomputes the active mapping for the current
//! foreground app.

use crate::config::{Action, ButtonMap, Config};
use crate::hidpp::device::{DeviceCommand, DeviceStatus};
use crate::profiles;
use arc_swap::ArcSwap;
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

static CONFIG: OnceLock<ArcSwap<Config>> = OnceLock::new();
static ACTIVE_MAP: OnceLock<ArcSwap<ButtonMap>> = OnceLock::new();
static PAUSED: AtomicBool = AtomicBool::new(false);
static INJECT_TX: OnceLock<Sender<Action>> = OnceLock::new();
static DEVICE_STATUS: OnceLock<ArcSwap<DeviceStatus>> = OnceLock::new();
static DEVICE_TX: OnceLock<Sender<DeviceCommand>> = OnceLock::new();

fn config_cell() -> &'static ArcSwap<Config> {
    CONFIG.get_or_init(|| ArcSwap::from_pointee(Config::default()))
}

fn active_cell() -> &'static ArcSwap<ButtonMap> {
    ACTIVE_MAP.get_or_init(|| ArcSwap::from_pointee(ButtonMap::default()))
}

/// Install the initial config and derive the active mapping.
pub fn init(config: Config) {
    config_cell().store(Arc::new(config));
    reevaluate_active_profile();
}

/// Current config snapshot (cheap, lock-free).
pub fn config() -> Arc<Config> {
    config_cell().load_full()
}

/// Replace the whole config (from the GUI), then recompute the active mapping.
/// The caller is responsible for persisting and for notifying the device
/// thread if device settings changed.
pub fn set_config(config: Config) {
    config_cell().store(Arc::new(config));
    reevaluate_active_profile();
}

/// The mapping the hook is currently enforcing.
pub fn active_map() -> Arc<ButtonMap> {
    active_cell().load_full()
}

pub fn set_active_map(map: ButtonMap) {
    active_cell().store(Arc::new(map));
}

/// Recompute the active mapping from the current config + foreground process.
/// Called on foreground-window changes and on config edits.
pub fn reevaluate_active_profile() {
    let cfg = config();
    let process = profiles::foreground_process_name();
    let map = profiles::resolve_button_map(&cfg, process.as_deref());
    if process.is_some() {
        tracing::debug!(
            "active profile for {:?}: {:?}",
            process.as_deref().unwrap_or("<unknown>"),
            map
        );
    }
    set_active_map(map);
}

pub fn is_paused() -> bool {
    PAUSED.load(Ordering::Relaxed)
}

pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::Relaxed);
    tracing::info!("remapping {}", if paused { "paused" } else { "resumed" });
}

pub fn set_inject_tx(tx: Sender<Action>) {
    let _ = INJECT_TX.set(tx);
}

/// Queue an action for the injector thread. Never blocks the hook callback.
pub fn inject(action: Action) {
    if let Some(tx) = INJECT_TX.get() {
        if tx.try_send(action).is_err() {
            tracing::warn!("injector queue full/closed; dropped action");
        }
    }
}

// ---- Device status / commands --------------------------------------------

fn device_status_cell() -> &'static ArcSwap<DeviceStatus> {
    DEVICE_STATUS.get_or_init(|| ArcSwap::from_pointee(DeviceStatus::default()))
}

pub fn device_status() -> Arc<DeviceStatus> {
    device_status_cell().load_full()
}

pub fn set_device_status(status: DeviceStatus) {
    device_status_cell().store(Arc::new(status));
}

/// Read-modify-write a single field of the published device status.
pub fn update_device_status<F: FnOnce(&mut DeviceStatus)>(f: F) {
    let current = device_status();
    let mut next = (*current).clone();
    f(&mut next);
    set_device_status(next);
}

pub fn set_device_tx(tx: Sender<DeviceCommand>) {
    let _ = DEVICE_TX.set(tx);
}

/// Send a command to the device thread (no-op if it isn't running).
pub fn device_command(cmd: DeviceCommand) {
    if let Some(tx) = DEVICE_TX.get() {
        let _ = tx.try_send(cmd);
    }
}
