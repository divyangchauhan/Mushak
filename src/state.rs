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
static FOREGROUND_REQUIRES_NATIVE_WHEEL: AtomicBool = AtomicBool::new(false);
static UIACCESS_HELPER_AVAILABLE: AtomicBool = AtomicBool::new(false);
static WHEEL_INJECTION_FAILED: AtomicBool = AtomicBool::new(false);
static WHEEL_NATIVE_FALLBACK: AtomicBool = AtomicBool::new(false);
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

    // High-resolution wheel reports are normally diverted to HID++ and then
    // re-injected. UIPI blocks ordinary injection into elevated windows. A
    // verified UIAccess helper can bridge that boundary; otherwise switch the
    // device back to native wheel reports while the elevated window is active.
    FOREGROUND_REQUIRES_NATIVE_WHEEL.store(
        profiles::foreground_requires_native_wheel(),
        Ordering::Relaxed,
    );
    // A prior SendInput failure is tied to the old foreground window. Let the
    // new window attempt the configured path again.
    WHEEL_INJECTION_FAILED.store(false, Ordering::Relaxed);
    refresh_wheel_fallback();
}

pub fn wheel_native_fallback() -> bool {
    WHEEL_NATIVE_FALLBACK.load(Ordering::Relaxed)
}

fn refresh_wheel_fallback() {
    let fallback = needs_native_wheel_fallback(
        FOREGROUND_REQUIRES_NATIVE_WHEEL.load(Ordering::Relaxed),
        UIACCESS_HELPER_AVAILABLE.load(Ordering::Relaxed),
        WHEEL_INJECTION_FAILED.load(Ordering::Relaxed),
    );
    if WHEEL_NATIVE_FALLBACK.swap(fallback, Ordering::Relaxed) != fallback {
        tracing::info!(
            "wheel input switched to {} for foreground integrity",
            if fallback {
                "native"
            } else {
                "configured mode"
            }
        );
        device_command(DeviceCommand::ApplyWheelMode);
    }
}

fn needs_native_wheel_fallback(
    higher_integrity: bool,
    helper_available: bool,
    injection_failed: bool,
) -> bool {
    injection_failed || (higher_integrity && !helper_available)
}

/// Called only after the broker has verified that its child token really has
/// UIAccess. Losing the helper immediately re-enables native wheel fallback.
pub fn set_uiaccess_helper_available(available: bool) {
    if UIACCESS_HELPER_AVAILABLE.swap(available, Ordering::Relaxed) != available {
        tracing::info!(
            "UIAccess wheel helper {}",
            if available { "ready" } else { "unavailable" }
        );
        refresh_wheel_fallback();
    }
}

/// Whether the next wheel event must cross UIPI through the helper.
pub fn wheel_needs_uiaccess_helper() -> bool {
    FOREGROUND_REQUIRES_NATIVE_WHEEL.load(Ordering::Relaxed)
        && UIACCESS_HELPER_AVAILABLE.load(Ordering::Relaxed)
}

/// Direct SendInput can be blocked even if querying the foreground token was
/// inconclusive. Fail closed to native reports until focus changes.
pub fn report_wheel_injection_failure() {
    if !WHEEL_INJECTION_FAILED.swap(true, Ordering::Relaxed) {
        tracing::warn!("wheel SendInput was blocked; enabling native fallback");
        refresh_wheel_fallback();
    }
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

#[cfg(test)]
mod tests {
    use super::needs_native_wheel_fallback;

    #[test]
    fn elevated_foreground_fails_closed_without_the_helper() {
        assert!(needs_native_wheel_fallback(true, false, false));
        assert!(!needs_native_wheel_fallback(true, true, false));
        assert!(!needs_native_wheel_fallback(false, false, false));
        assert!(needs_native_wheel_fallback(false, true, true));
    }
}
