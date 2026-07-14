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

use eframe::egui;

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

/// The `--settings` subprocess: just the eframe settings window. It owns no
/// hooks, HID handle or tray — it edits config.toml and reads status.json.
fn settings_main() {
    tracing::info!("Mushak settings window starting");
    state::init(config::Config::load());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Mushak Settings")
            .with_inner_size([780.0, 580.0])
            .with_min_inner_size([560.0, 420.0]),
        ..Default::default()
    };

    let result = eframe::run_native(
        "Mushak",
        options,
        Box::new(|cc| Ok(Box::new(gui::App::new(cc)) as Box<dyn eframe::App>)),
    );
    if let Err(e) = result {
        tracing::error!("eframe run_native failed: {e}");
    }
    tracing::info!("Mushak settings window closed");
}
