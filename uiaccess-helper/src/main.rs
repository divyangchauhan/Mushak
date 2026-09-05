#![windows_subsystem = "windows"]

//! Minimal UIAccess process for one operation: injecting vertical wheel input.
//!
//! There is intentionally no general command execution, keyboard support,
//! configuration parsing, filesystem access, networking, or discoverable IPC.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenUIAccess, TOKEN_QUERY};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken,
    QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT,
};

const READY_LINE: &[u8] = b"MUSHAK_UIACCESS_WHEEL_V1\n";
const INJECT_SIGNATURE: usize = 0x4D55_5348; // ASCII "MUSH"
/// 64 standard notches in one report is already far beyond physical use while
/// still allowing a fast free-spin burst. Anything larger is malformed.
const MAX_ABS_DELTA: i32 = 120 * 64;
const MAX_MESSAGES_PER_SECOND: u32 = 2_000;

fn main() {
    if !token_has_uiaccess() || !parent_is_installed_mushak() {
        std::process::exit(78);
    }

    let mut output = std::io::stdout().lock();
    if output
        .write_all(READY_LINE)
        .and_then(|_| output.flush())
        .is_err()
    {
        return;
    }
    drop(output);

    let mut input = std::io::stdin().lock();
    let mut bytes = [0u8; 4];
    let mut rate_window = Instant::now();
    let mut messages = 0u32;
    loop {
        if input.read_exact(&mut bytes).is_err() {
            return;
        }
        if rate_window.elapsed() >= Duration::from_secs(1) {
            rate_window = Instant::now();
            messages = 0;
        }
        messages += 1;
        if messages > MAX_MESSAGES_PER_SECOND {
            return;
        }
        let delta = i32::from_le_bytes(bytes);
        if !valid_delta(delta) {
            // A private channel producing invalid data indicates corruption or
            // a compromised parent. Fail closed instead of widening behavior.
            return;
        }
        if !inject_wheel(delta) {
            return;
        }
    }
}

/// A private pipe alone is not authentication: another process could launch
/// its own helper instance. Bind this process to the protected sibling
/// `mushak.exe` selected by Windows as our actual parent.
fn parent_is_installed_mushak() -> bool {
    let our_pid = unsafe { GetCurrentProcessId() };
    let Some(parent_pid) = parent_pid(our_pid) else {
        return false;
    };
    let Some(actual) = process_path(parent_pid) else {
        return false;
    };
    let Some(expected) = std::env::current_exe()
        .ok()
        .map(|p| p.with_file_name("mushak.exe"))
    else {
        return false;
    };
    same_path(&actual, &expected)
}

fn parent_pid(our_pid: u32) -> Option<u32> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = None;
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == our_pid {
                    found = Some(entry.th32ParentProcessID);
                    break;
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        found.filter(|pid| *pid != 0)
    }
}

fn process_path(pid: u32) -> Option<PathBuf> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        result.ok()?;
        (len > 0).then(|| PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))
    }
}

fn same_path(left: &PathBuf, right: &PathBuf) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.clone());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.clone());
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn token_has_uiaccess() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut enabled = 0u32;
        let mut returned = 0u32;
        let result = GetTokenInformation(
            token,
            TokenUIAccess,
            Some((&mut enabled as *mut u32).cast()),
            std::mem::size_of::<u32>() as u32,
            &mut returned,
        );
        let _ = CloseHandle(token);
        result.is_ok() && returned as usize == std::mem::size_of::<u32>() && enabled != 0
    }
}

fn valid_delta(delta: i32) -> bool {
    delta != 0 && delta.unsigned_abs() <= MAX_ABS_DELTA as u32
}

fn inject_wheel(delta: i32) -> bool {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: delta as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: INJECT_SIGNATURE,
            },
        },
    };
    unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 1 }
}
