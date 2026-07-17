//! Single-instance guard for the resident process.
//!
//! Only one Mushak resident may run per user session. We enforce this with a
//! named mutex in the session-local (`Local\`) namespace: the first resident
//! creates it, and any later launch sees `ERROR_ALREADY_EXISTS` and bows out.
//! Rather than silently dying, the second launch signals a named event so the
//! running resident pops its settings window open — the same thing a user
//! double-clicking the tray icon expects.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE,
};

/// Session-scoped so two different logged-in users each get their own resident.
const MUTEX_NAME: PCWSTR = w!("Local\\Mushak.Resident.SingleInstance");
const EVENT_NAME: PCWSTR = w!("Local\\Mushak.Resident.ShowSettings");

/// Held for the lifetime of the resident. Dropping (or exiting) releases the
/// mutex so the next launch can take over.
pub struct InstanceGuard {
    mutex: HANDLE,
    show_settings: HANDLE,
}

impl InstanceGuard {
    /// Try to become the one resident. Returns `None` if another resident
    /// already owns the mutex (in which case it has been asked to show its
    /// settings window, and the caller should exit).
    pub fn acquire() -> Option<InstanceGuard> {
        unsafe {
            // CreateMutexW returns a valid handle even when the named object
            // already exists; the distinction is in GetLastError.
            let mutex = match CreateMutexW(None, false, MUTEX_NAME) {
                Ok(h) => h,
                Err(e) => {
                    // Can't arbitrate — don't block the app; run anyway.
                    tracing::warn!("single-instance mutex create failed: {e:#}; continuing");
                    return Some(InstanceGuard {
                        mutex: HANDLE::default(),
                        show_settings: HANDLE::default(),
                    });
                }
            };

            if windows::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS {
                // Someone got here first. Ask them to surface their window.
                let _ = CloseHandle(mutex);
                signal_show_settings();
                return None;
            }

            // We're the owner. Create the manual-reset event the running
            // resident polls (initially unsignaled).
            let show_settings = CreateEventW(None, true, false, EVENT_NAME)
                .unwrap_or_else(|e| {
                    tracing::warn!("show-settings event create failed: {e:#}");
                    HANDLE::default()
                });

            Some(InstanceGuard {
                mutex,
                show_settings,
            })
        }
    }

    /// True if a second launch asked us to open settings since the last check.
    /// Consumes the signal (resets the event).
    pub fn take_show_settings_request(&self) -> bool {
        if self.show_settings.is_invalid() {
            return false;
        }
        unsafe {
            if WaitForSingleObject(self.show_settings, 0) == WAIT_OBJECT_0 {
                let _ = windows::Win32::System::Threading::ResetEvent(self.show_settings);
                return true;
            }
        }
        false
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.show_settings.is_invalid() {
                let _ = CloseHandle(self.show_settings);
            }
            if !self.mutex.is_invalid() {
                let _ = CloseHandle(self.mutex);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_is_refused_and_signals_owner() {
        // First acquire owns the mutex.
        let owner = InstanceGuard::acquire().expect("first acquire owns the lock");
        assert!(
            !owner.take_show_settings_request(),
            "no request should be pending yet"
        );

        // A second acquire (same process, same named mutex) is refused and
        // pulses the owner's show-settings event.
        assert!(
            InstanceGuard::acquire().is_none(),
            "second acquire must be refused while the first is held"
        );
        assert!(
            owner.take_show_settings_request(),
            "the refused launch should have asked the owner to show settings"
        );
        // Signal is consumed after one read.
        assert!(!owner.take_show_settings_request());

        // Once the owner drops, a fresh acquire succeeds again.
        drop(owner);
        let next = InstanceGuard::acquire().expect("lock is free after owner drops");
        drop(next);
    }
}

/// Open the running resident's event and pulse it, so it opens its settings
/// window. Best-effort: if the event isn't there (racing shutdown), we simply
/// exit quietly.
fn signal_show_settings() {
    unsafe {
        match OpenEventW(EVENT_MODIFY_STATE, false, EVENT_NAME) {
            Ok(ev) => {
                let _ = SetEvent(ev);
                let _ = CloseHandle(ev);
                tracing::info!("another instance is running; asked it to show settings");
            }
            Err(e) => {
                tracing::info!("another instance is running (couldn't signal it: {e:#})");
            }
        }
    }
}
