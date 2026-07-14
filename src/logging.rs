//! File logging setup (rotating daily file at debug level) plus a hex-dump
//! helper for raw HID++ traffic.

use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// Initialize logging. `name` selects the log file base (so the resident and
/// settings processes don't write the same file). Returns a guard that must be
/// held for the lifetime of the process.
pub fn init(name: &str) -> Result<WorkerGuard> {
    let log_dir = crate::config::app_dir()
        .context("resolving %APPDATA%")?
        .join("logs");
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("creating {}", log_dir.display()))?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, format!("{name}.log"));
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    // Default to debug for our crate; override with RUST_LOG if present.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("mushak=debug,warn"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_target(true)
        .with_thread_names(true)
        .with_writer(writer)
        .init();

    tracing::info!(
        "logging to {} (set RUST_LOG to override level)",
        log_dir.display()
    );
    Ok(guard)
}

/// Format a byte slice as a compact hex string, e.g. `10 01 00 5a`.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{b:02x}"));
    }
    s
}
