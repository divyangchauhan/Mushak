//! "Start with Windows" via the per-user Run registry key.

use anyhow::{Context, Result};
use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE_NAME: PCWSTR = w!("Mushak");

/// Enable or disable launching this executable at user logon.
pub fn set(enabled: bool) -> Result<()> {
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
    tracing::info!("start-with-windows set to {enabled}");
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
