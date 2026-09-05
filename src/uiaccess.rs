//! Lifecycle and private IPC for the optional wheel-only UIAccess helper.
//!
//! The helper is deliberately a separate executable with no config parser,
//! HID access, keyboard injection, or user-facing UI. Standard input is an
//! inherited anonymous pipe, so no unrelated process can discover and connect
//! to a named endpoint. The helper also verifies its actual token before it
//! announces readiness; an unsigned copy or one outside Program Files exits
//! without ever being trusted by the resident.

use crossbeam_channel::{Receiver, Sender};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

const HELPER_EXE: &str = "mushak-uiaccess-helper.exe";
const READY_LINE: &str = "MUSHAK_UIACCESS_WHEEL_V1";
const RETRY_DELAY: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(100);
const QUEUE_CAPACITY: usize = 256;

static WHEEL_TX: OnceLock<Sender<i32>> = OnceLock::new();
static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static RESET_HELPER: AtomicBool = AtomicBool::new(false);

pub fn spawn() {
    let (tx, rx) = crossbeam_channel::bounded(QUEUE_CAPACITY);
    let _ = WHEEL_TX.set(tx);
    std::thread::Builder::new()
        .name("uiaccess-wheel-broker".into())
        .spawn(move || run(rx))
        .expect("spawn UIAccess wheel broker");
}

pub fn request_stop() {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

/// Queue one already-scaled Windows wheel delta without blocking the HID
/// thread. `false` means the caller must fail closed to native device input.
pub fn wheel(delta: i32) -> bool {
    let queued = WHEEL_TX.get().is_some_and(|tx| tx.try_send(delta).is_ok());
    if !queued {
        RESET_HELPER.store(true, Ordering::Relaxed);
    }
    queued
}

fn run(rx: Receiver<i32>) {
    tracing::debug!("UIAccess wheel broker started");
    while !SHUTDOWN.load(Ordering::Relaxed) {
        drain(&rx);
        match start_helper() {
            Ok((mut child, mut input)) => {
                crate::state::set_uiaccess_helper_available(true);
                serve(&rx, &mut child, &mut input);
                crate::state::set_uiaccess_helper_available(false);
                let _ = child.kill();
                let _ = child.wait();
            }
            Err(e) => {
                crate::state::set_uiaccess_helper_available(false);
                tracing::debug!("UIAccess wheel helper not ready: {e}");
            }
        }
        drain(&rx); // Never replay wheel motion collected during an outage.
        wait_for_retry();
    }
    crate::state::set_uiaccess_helper_available(false);
    tracing::debug!("UIAccess wheel broker stopped");
}

fn start_helper() -> Result<(Child, ChildStdin), String> {
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let path = current
        .parent()
        .ok_or_else(|| "resident executable has no parent directory".to_string())?
        .join(HELPER_EXE);
    if !path.is_file() {
        return Err(format!("{} is not installed", path.display()));
    }

    let mut child = Command::new(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("starting {} failed: {e}", path.display()))?;
    let input = child
        .stdin
        .take()
        .ok_or_else(|| "helper stdin pipe was not created".to_string())?;
    let output = child
        .stdout
        .take()
        .ok_or_else(|| "helper stdout pipe was not created".to_string())?;

    // The helper writes this only after GetTokenInformation(TokenUIAccess)
    // confirms that Windows granted the privilege. EOF means the signature,
    // install location, or manifest did not satisfy Windows policy.
    let mut line = String::new();
    BufReader::new(output)
        .read_line(&mut line)
        .map_err(|e| format!("reading helper handshake failed: {e}"))?;
    if line.trim_end() != READY_LINE {
        let status = child
            .try_wait()
            .ok()
            .flatten()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "no status".to_string());
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "helper did not receive UIAccess (handshake missing; {status})"
        ));
    }
    Ok((child, input))
}

fn serve(rx: &Receiver<i32>, child: &mut Child, input: &mut ChildStdin) {
    loop {
        if SHUTDOWN.load(Ordering::Relaxed) {
            return;
        }
        if RESET_HELPER.swap(false, Ordering::Relaxed) {
            return;
        }
        if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
            return;
        }
        match rx.recv_timeout(POLL) {
            Ok(delta) => {
                // Focus may have changed after this event was queued. Never
                // replay a privileged-path wheel event into a later window.
                if !crate::state::wheel_needs_uiaccess_helper() {
                    continue;
                }
                if input.write_all(&delta.to_le_bytes()).is_err() {
                    return;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn drain(rx: &Receiver<i32>) {
    while rx.try_recv().is_ok() {}
}

fn wait_for_retry() {
    let steps = RETRY_DELAY.as_millis() / POLL.as_millis();
    for _ in 0..steps {
        if SHUTDOWN.load(Ordering::Relaxed) {
            return;
        }
        std::thread::sleep(POLL);
    }
}
