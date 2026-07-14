//! Foreground-application detection and per-app profile resolution.

use crate::config::{ButtonMap, Config};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, FALSE};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Basename of the executable owning the current foreground window, e.g.
/// `"chrome.exe"`. Returns `None` if there is no foreground window or the
/// process cannot be queried (e.g. an elevated process while we run
/// unelevated).
pub fn foreground_process_name() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid).ok()?;

        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);

        if result.is_err() || len == 0 {
            return None;
        }

        let full = String::from_utf16_lossy(&buf[..len as usize]);
        Some(basename(&full))
    }
}

fn basename(path: &str) -> String {
    path.rsplit(['\\', '/'])
        .next()
        .unwrap_or(path)
        .to_string()
}

/// Pick the button mapping for the given foreground process: the first profile
/// whose `match_processes` contains it, else the default profile.
pub fn resolve_button_map(cfg: &Config, process: Option<&str>) -> ButtonMap {
    if let Some(proc) = process {
        for p in &cfg.profiles {
            if p.matches(proc) {
                return p.buttons.clone();
            }
        }
    }
    cfg.default_profile.buttons.clone()
}
