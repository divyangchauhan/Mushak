//! Foreground-application detection and per-app profile resolution.

use crate::config::{ButtonMap, Config};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, FALSE, HANDLE, HWND};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
    TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
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

/// True when Mushak must hand wheel input back to the physical device for the
/// current foreground window.
///
/// `SendInput` cannot cross a UIPI boundary. Failure to inspect a real
/// foreground process is therefore treated as protected instead of assuming
/// that injection is safe; protected/elevated processes are exactly the ones
/// most likely to deny token inspection from a normal user process.
pub fn foreground_requires_native_wheel() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return false;
        }
        let Some(target) = window_process(hwnd) else {
            return true;
        };
        let target_level = process_integrity_level(target);
        let _ = CloseHandle(target);

        needs_native_wheel(process_integrity_level(GetCurrentProcess()), target_level)
    }
}

unsafe fn window_process(hwnd: HWND) -> Option<HANDLE> {
    if hwnd.0.is_null() {
        return None;
    }
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return None;
    }
    OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid).ok()
}

/// Read the mandatory integrity RID from a process token. The returned value
/// is ordered (low < medium < high < system), which is all UIPI comparison
/// needs here.
unsafe fn process_integrity_level(process: HANDLE) -> Option<u32> {
    let mut token = HANDLE::default();
    OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;

    let mut needed = 0;
    let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut needed);
    if needed == 0 {
        let _ = CloseHandle(token);
        return None;
    }

    // `usize` gives the backing allocation enough alignment for
    // TOKEN_MANDATORY_LABEL; the SID itself follows it in the same buffer.
    let words = (needed as usize).div_ceil(std::mem::size_of::<usize>());
    let mut storage = vec![0usize; words];
    let result = GetTokenInformation(
        token,
        TokenIntegrityLevel,
        Some(storage.as_mut_ptr().cast()),
        needed,
        &mut needed,
    );
    let _ = CloseHandle(token);
    result.ok()?;

    let label = &*(storage.as_ptr() as *const TOKEN_MANDATORY_LABEL);
    let count = GetSidSubAuthorityCount(label.Label.Sid);
    if count.is_null() || *count == 0 {
        return None;
    }
    let rid = GetSidSubAuthority(label.Label.Sid, (*count - 1) as u32);
    (!rid.is_null()).then(|| *rid)
}

fn needs_native_wheel(ours: Option<u32>, target: Option<u32>) -> bool {
    match (ours, target) {
        (Some(ours), Some(target)) => target > ours,
        // Unknown is not evidence that SendInput is allowed. Fail closed so
        // the mouse emits real wheel reports, which UIPI does not block.
        _ => true,
    }
}

fn basename(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
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

#[cfg(test)]
mod tests {
    use super::needs_native_wheel;

    #[test]
    fn higher_integrity_requires_native_wheel_input() {
        assert!(needs_native_wheel(Some(0x2000), Some(0x3000)));
    }

    #[test]
    fn same_or_lower_integrity_can_use_injection() {
        assert!(!needs_native_wheel(Some(0x2000), Some(0x2000)));
        assert!(!needs_native_wheel(Some(0x3000), Some(0x2000)));
    }

    #[test]
    fn an_unknown_integrity_level_fails_closed_to_native_input() {
        assert!(needs_native_wheel(None, Some(0x3000)));
        assert!(needs_native_wheel(Some(0x2000), None));
    }
}
