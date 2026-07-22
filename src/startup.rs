//! "Start with Windows".
//!
//! There are two mechanisms, picked at runtime:
//!
//! * **Unpackaged** (the GitHub / Scoop / winget exe) writes the per-user Run
//!   registry key. This is the plain Win32 way and is what every current
//!   release uses.
//! * **Packaged** (an MSIX from the Microsoft Store) uses the WinRT
//!   `StartupTask` API instead. Inside MSIX the Run key is the wrong tool: the
//!   exe lives in the locked `WindowsApps` folder and must be launched through
//!   package activation, and Store certification rejects Run-key autostart.
//!
//! The two APIs are mutually exclusive: `StartupTask` needs package identity,
//! so it only works packaged; the Run key is inappropriate packaged. `set`
//! detects which world it is in and dispatches accordingly, so a single binary
//! serves both channels.

use anyhow::{Context, Result};
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, WIN32_ERROR};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE_NAME: PCWSTR = w!("Mushak");

/// Must match the `TaskId` on the `windows.startupTask` extension in the MSIX
/// `AppxManifest.xml` (see `packaging/msix/`). If they drift, enabling
/// start-with-Windows silently no-ops in the packaged build.
const STARTUP_TASK_ID: &str = "MushakStartup";

/// Enable or disable launching this app at user logon.
pub fn set(enabled: bool) -> Result<()> {
    if is_packaged() {
        set_via_startup_task(enabled)?;
    } else {
        set_via_run_key(enabled)?;
    }
    tracing::info!("start-with-windows set to {enabled}");
    Ok(())
}

/// True when running from an MSIX package (i.e. the process has package
/// identity). Unpackaged, `GetCurrentPackageFullName` returns
/// `APPMODEL_ERROR_NO_PACKAGE`; packaged, a zero-length buffer yields
/// `ERROR_INSUFFICIENT_BUFFER`.
fn is_packaged() -> bool {
    let mut len: u32 = 0;
    let rc: WIN32_ERROR = unsafe { GetCurrentPackageFullName(&mut len, PWSTR::null()) };
    rc == ERROR_INSUFFICIENT_BUFFER
}

/// Packaged path: toggle the manifest-declared StartupTask via WinRT.
fn set_via_startup_task(enabled: bool) -> Result<()> {
    use windows::core::HSTRING;
    use windows::ApplicationModel::{StartupTask, StartupTaskState};

    let task = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))
        .context("StartupTask::GetAsync")?
        .get()
        .context("awaiting StartupTask")?;

    if enabled {
        // The user can veto autostart in Task Manager > Startup, and policy can
        // block it. We can't override either, so we report success but log the
        // effective state so it's visible in the logs.
        let state = task
            .RequestEnableAsync()
            .context("RequestEnableAsync")?
            .get()
            .context("awaiting RequestEnableAsync")?;
        match state {
            StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy => {}
            StartupTaskState::DisabledByUser => {
                tracing::warn!("start-with-Windows is disabled by the user in Task Manager");
            }
            StartupTaskState::DisabledByPolicy => {
                tracing::warn!("start-with-Windows is disabled by system policy");
            }
            other => {
                tracing::warn!("start-with-Windows did not enable (state {other:?})");
            }
        }
    } else {
        task.Disable().context("StartupTask::Disable")?;
    }
    Ok(())
}

/// Unpackaged path: write/remove the per-user Run registry value.
fn set_via_run_key(enabled: bool) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
        .ok()
        .context("opening Run registry key")?;

        let result = if enabled {
            let exe = std::env::current_exe().context("current_exe")?;
            let quoted = format!("\"{}\"", exe.display());
            let data = to_wide_bytes(&quoted);
            RegSetValueExW(hkey, VALUE_NAME, 0, REG_SZ, Some(&data))
                .ok()
                .context("writing Run value")
        } else {
            // Deleting a missing value is fine; ignore not-found.
            let _ = RegDeleteValueW(hkey, VALUE_NAME);
            Ok(())
        };

        let _ = RegCloseKey(hkey);
        result?;
    }
    Ok(())
}

/// UTF-16LE bytes of `s` including a trailing NUL, for `REG_SZ`.
fn to_wide_bytes(s: &str) -> Vec<u8> {
    let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    let mut bytes = Vec::with_capacity(wide.len() * 2);
    for w in wide {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
}
