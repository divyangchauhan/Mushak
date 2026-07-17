//! The always-resident process: system tray + mouse hook + HID device thread,
//! driven by a lightweight Win32 message pump (no GUI / GPU context, so idle
//! RAM stays small). The settings window is launched on demand as a separate
//! `--settings` process; the two communicate through `config.toml` (edits) and
//! `status.json` (live device status).

use crate::config::Config;
use crate::hidpp::device::DeviceCommand;
use crate::statusfile::{self, SharedStatus};
use crate::tray::Tray;
use crate::{hidpp, hook, injector, startup, state};
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

pub fn run() {
    tracing::info!("Mushak {} resident starting", env!("CARGO_PKG_VERSION"));

    // Only one resident per session. If another already owns the lock, it has
    // been asked to show its settings window; this launch just exits.
    let instance = match crate::single_instance::InstanceGuard::acquire() {
        Some(g) => g,
        None => {
            tracing::info!("another Mushak resident is already running; exiting");
            return;
        }
    };

    let cfg = Config::load();
    let inject_tx = injector::spawn();
    state::set_inject_tx(inject_tx);
    state::init(cfg);

    hook::spawn();
    let device_tx = hidpp::device::spawn();
    state::set_device_tx(device_tx);

    let tray = Tray::build(state::config().start_with_windows).expect("build tray icon");

    let exe = std::env::current_exe().unwrap_or_default();
    let config_path = Config::path().ok();
    let mut config_mtime = config_path.as_deref().and_then(mtime);
    let mut settings_child: Option<Child> = None;
    let mut last_status = Instant::now();
    let mut last_cfg_check = Instant::now();
    let mut quit = false;

    while !quit {
        pump_messages();

        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == *tray.open_id() {
                open_settings(&exe, &mut settings_child);
            } else if ev.id == *tray.quit_id() {
                quit = true;
            } else if ev.id == *tray.pause_id() {
                let paused = !state::is_paused();
                state::set_paused(paused);
                tray.set_paused(paused);
            } else if ev.id == *tray.startup_id() {
                toggle_startup(&tray, config_path.as_deref(), &mut config_mtime);
            }
        }
        while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::DoubleClick { .. } = ev {
                open_settings(&exe, &mut settings_child);
            }
        }

        // A second launch of the exe pulses this instead of starting up.
        if instance.take_show_settings_request() {
            open_settings(&exe, &mut settings_child);
        }

        update_tray_battery(&tray);

        if last_status.elapsed() >= Duration::from_secs(1) {
            statusfile::write(&SharedStatus {
                device: (*state::device_status()).clone(),
                paused: state::is_paused(),
            });
            last_status = Instant::now();
        }

        if last_cfg_check.elapsed() >= Duration::from_millis(500) {
            if let Some(p) = config_path.as_deref() {
                let m = mtime(p);
                if m != config_mtime {
                    // Only advance our tracked mtime once we parse cleanly, so a
                    // partial write from the settings process is retried.
                    match Config::load_strict() {
                        Ok(new_cfg) => {
                            config_mtime = m;
                            tray.set_startup(new_cfg.start_with_windows);
                            state::set_config(new_cfg);
                            state::device_command(DeviceCommand::ApplyDeviceConfig);
                            state::device_command(DeviceCommand::ApplyControlDiverts);
                            tracing::info!("reloaded config after external edit");
                        }
                        Err(e) => tracing::debug!("config not ready, retrying: {e:#}"),
                    }
                }
            }
            last_cfg_check = Instant::now();
        }

        if let Some(c) = &mut settings_child {
            if matches!(c.try_wait(), Ok(Some(_))) {
                settings_child = None;
            }
        }

        std::thread::sleep(Duration::from_millis(40));
    }

    tracing::info!("Mushak resident stopping");
    hook::request_stop();
    hidpp::device::request_stop();
    state::device_command(DeviceCommand::Shutdown);
    // Let the HID thread restore the gesture divert before we exit.
    std::thread::sleep(Duration::from_millis(300));
}

fn pump_messages() {
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}

fn update_tray_battery(tray: &Tray) {
    let s = state::device_status();
    let text = match (s.connected, s.battery_percent) {
        (true, Some(p)) => format!(
            "Battery: {p}%{}",
            if s.charging { " (charging)" } else { "" }
        ),
        (true, None) => "Battery: unknown".to_string(),
        (false, _) => "Device: disconnected".to_string(),
    };
    tray.set_battery(&text);
}

fn open_settings(exe: &Path, child: &mut Option<Child>) {
    if let Some(c) = child {
        if matches!(c.try_wait(), Ok(None)) {
            // A settings window is already open.
            return;
        }
    }
    match Command::new(exe).arg("--settings").spawn() {
        Ok(c) => *child = Some(c),
        Err(e) => tracing::error!("failed to launch settings window: {e}"),
    }
}

fn toggle_startup(tray: &Tray, config_path: Option<&Path>, config_mtime: &mut Option<SystemTime>) {
    let mut cfg = (*state::config()).clone();
    cfg.start_with_windows = !cfg.start_with_windows;
    if let Err(e) = startup::set(cfg.start_with_windows) {
        tracing::error!("failed to update startup setting: {e:#}");
    }
    tray.set_startup(cfg.start_with_windows);
    if let Err(e) = cfg.save() {
        tracing::error!("failed to save config: {e:#}");
    }
    // Skip the watcher's redundant reload of our own write.
    if let Some(p) = config_path {
        *config_mtime = mtime(p);
    }
    state::set_config(cfg);
}
