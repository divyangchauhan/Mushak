//! Detect (and, on request, terminate) other Logitech drivers that would fight
//! Mushak over the device — they hold the HID++ diverts and reprogram controls.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

/// Process-name fragments (lowercase) that indicate a conflicting driver.
const NEEDLES: [&str; 4] = ["logioptions", "logi options", "lghub", "lghub_agent"];

/// A running conflicting process.
#[derive(Clone, Debug)]
pub struct ConflictProc {
    pub pid: u32,
    pub name: String,
}

/// Every conflicting process currently running, with its PID.
///
/// Options+ runs as a cluster (agent, UI, updater, plugin hosts), so this
/// routinely returns several entries sharing a name.
pub fn scan() -> Vec<ConflictProc> {
    let mut found: Vec<ConflictProc> = Vec::new();
    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!("process snapshot failed: {e}");
                return found;
            }
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = wide_to_string(&entry.szExeFile).to_ascii_lowercase();
                if NEEDLES.iter().any(|n| name.contains(n)) {
                    found.push(ConflictProc {
                        pid: entry.th32ProcessID,
                        name,
                    });
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    found
}

/// The distinct names of conflicting processes currently running.
pub fn detect() -> Vec<String> {
    let mut names: Vec<String> = scan().into_iter().map(|p| p.name).collect();
    names.sort();
    names.dedup();
    names
}

/// What `kill_all` managed to do.
#[derive(Default, Debug, Clone)]
pub struct KillOutcome {
    pub killed: usize,
    /// Processes we could not open or terminate, with the reason.
    pub failed: Vec<(String, String)>,
}

impl KillOutcome {
    pub fn all_gone(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Terminate every conflicting process.
///
/// These are killed, not asked politely: Options+ has no documented quit IPC,
/// and its agent restarts the UI if only the UI goes away. Options+ normally
/// runs as the logged-in user, so this needs no elevation — but its updater
/// service can run as SYSTEM, which we will fail to open, hence `failed`.
pub fn kill_all() -> KillOutcome {
    let mut outcome = KillOutcome::default();
    for proc in scan() {
        match terminate(proc.pid) {
            Ok(()) => {
                tracing::info!("terminated {} (pid {})", proc.name, proc.pid);
                outcome.killed += 1;
            }
            Err(e) => {
                tracing::warn!("could not terminate {} (pid {}): {e}", proc.name, proc.pid);
                outcome.failed.push((proc.name, e));
            }
        }
    }
    outcome
}

fn terminate(pid: u32) -> Result<(), String> {
    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_TERMINATE, false, pid)
            .map_err(|e| format!("open failed: {}", friendly(&e)))?;
        let result = TerminateProcess(handle, 1).map_err(|e| format!("{}", friendly(&e)));
        let _ = CloseHandle(handle);
        result
    }
}

/// Win32 errors stringify with a lot of noise; the message alone is enough for
/// a banner.
fn friendly(e: &windows::core::Error) -> String {
    let msg = e.message();
    if msg.is_empty() {
        format!("{:#x}", e.code().0)
    } else {
        msg.trim().to_string()
    }
}

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}
