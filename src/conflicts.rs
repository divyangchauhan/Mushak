//! Detect other Logitech drivers that would fight Mushak over the device
//! (they hold the HID++ diverts and reprogram controls).

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

/// Process-name fragments (lowercase) that indicate a conflicting driver.
const NEEDLES: [&str; 4] = ["logioptions", "logi options", "lghub", "lghub_agent"];

/// Return the set of conflicting process names currently running.
pub fn detect() -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
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
                    found.push(name);
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    found.sort();
    found.dedup();
    found
}

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}
