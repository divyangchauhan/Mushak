// No console window in release; keep one in debug for convenience.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod conflicts;
mod gestures;
mod gui;
mod hidpp;
mod hook;
mod injector;
mod logging;
mod profiles;
mod resident;
mod startup;
mod state;
mod statusfile;
mod tray;

fn main() {
    let settings_mode = std::env::args().any(|a| a == "--settings");

    let _log_guard = match logging::init(if settings_mode { "mushak-settings" } else { "mushak" }) {
        Ok(g) => Some(g),
        Err(e) => {
            eprintln!("logging init failed: {e:#}");
            None
        }
    };

    if settings_mode {
        settings_main();
    } else {
        resident::run();
    }
}

/// The `--settings` subprocess: just the settings window. It owns no hooks,
/// HID handle or tray — it edits config.toml and reads status.json.
fn settings_main() {
    tracing::info!("Mushak settings window starting");
    state::init(config::Config::load());
    gui::run();
    tracing::info!("Mushak settings window closed");
}
