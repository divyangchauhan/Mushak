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
mod single_instance;
mod startup;
mod state;
mod statusfile;
mod tray;

fn main() {
    #[cfg(debug_assertions)]
    if std::env::args().any(|a| a == "--ui-smoke") {
        gui::run_ui_smoke();
        return;
    }
    if std::env::args().any(|a| a == "--diagnose-device") {
        // Read-only discovery. Do not load config, start hooks, or apply settings.
        match hidpp::device::Device::connect() {
            Ok(_) => println!(
                "{}",
                serde_json::to_string_pretty(&*state::device_status()).unwrap()
            ),
            Err(error) => match hidpp::transport::HidppTransport::other_mouse_status() {
                Ok(Some(status)) => println!("{}", serde_json::to_string_pretty(&status).unwrap()),
                _ => {
                    eprintln!("{error:#}");
                    std::process::exit(1);
                }
            },
        }
        return;
    }
    let settings_mode = std::env::args().any(|a| a == "--settings");

    let _log_guard = match logging::init(if settings_mode {
        "mushak-settings"
    } else {
        "mushak"
    }) {
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
