//! Low-level mouse hook (`WH_MOUSE_LL`) that intercepts the side and middle
//! buttons, plus a `WH_*`-free `SetWinEventHook` for foreground-window changes
//! so per-app profiles switch automatically.
//!
//! Both hooks live on a dedicated thread that runs a Win32 message pump, as
//! `WH_MOUSE_LL` requires. The hook callback does the minimum possible work and
//! offloads key injection to the injector thread.

use crate::config::{Action, MouseButton};
use crate::injector::INJECT_SIGNATURE;
use crate::state;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HINSTANCE, HMODULE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, EVENT_SYSTEM_FOREGROUND, HC_ACTION, HHOOK, MSG,
    MSLLHOOKSTRUCT, WH_MOUSE_LL, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEWHEEL, WM_QUIT, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

/// Thread id of the hook thread, used to post `WM_QUIT` at shutdown.
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

// Keep release jitter out of both native and diverted wheel input.
static CLICK_GUARD: Mutex<ClickGuard> = Mutex::new(ClickGuard {
    down: false,
    released: None,
});
const RELEASE_GUARD: Duration = Duration::from_millis(150);

struct ClickGuard {
    down: bool,
    released: Option<Instant>,
}

impl ClickGuard {
    fn update(&mut self, msg: u32, now: Instant) {
        match msg {
            WM_MBUTTONDOWN => self.down = true,
            WM_MBUTTONUP => {
                self.down = false;
                self.released = Some(now);
            }
            _ => {}
        }
    }

    fn blocks(&self, now: Instant) -> bool {
        self.down
            || self
                .released
                .is_some_and(|released| now.duration_since(released) < RELEASE_GUARD)
    }
}

pub(crate) fn click_blocks_scroll() -> bool {
    !state::is_paused()
        && CLICK_GUARD
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .blocks(Instant::now())
}

fn suppress_click_scroll(msg: u32, paused: bool) -> bool {
    let mut guard = CLICK_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    guard.update(msg, now);
    msg == WM_MOUSEWHEEL && !paused && guard.blocks(now)
}

/// Spawn the hook thread. It installs the hooks and pumps messages until
/// [`request_stop`] is called.
pub fn spawn() {
    std::thread::Builder::new()
        .name("mouse-hook".into())
        .spawn(run)
        .expect("spawn hook thread");
}

/// Ask the hook thread to unhook and exit.
pub fn request_stop() {
    let tid = HOOK_THREAD_ID.load(Ordering::Relaxed);
    if tid != 0 {
        unsafe {
            let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

fn run() {
    unsafe {
        HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::Relaxed);

        let hinstance: HINSTANCE = match GetModuleHandleW(PCWSTR::null()) {
            Ok(hmod) => HINSTANCE(hmod.0),
            Err(e) => {
                tracing::error!("GetModuleHandleW failed: {e}");
                HINSTANCE::default()
            }
        };

        let mouse_hook =
            match SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), hinstance, 0) {
                Ok(h) => {
                    tracing::info!("installed WH_MOUSE_LL hook");
                    Some(h)
                }
                Err(e) => {
                    tracing::error!("SetWindowsHookExW failed: {e}");
                    None
                }
            };

        // Out-of-context WinEvent hook for foreground changes. hmod must be null
        // for out-of-context hooks.
        let winevent_hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            HMODULE(std::ptr::null_mut()),
            Some(foreground_changed_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        if winevent_hook.is_invalid() {
            tracing::warn!("SetWinEventHook (foreground) failed; per-app switching disabled");
        }

        // Message pump. WM_QUIT (posted by request_stop) returns 0 and ends it.
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret.0 <= 0 {
                break; // 0 == WM_QUIT, -1 == error
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if let Some(h) = mouse_hook {
            let _ = UnhookWindowsHookEx(h);
        }
        if !winevent_hook.is_invalid() {
            let _ = UnhookWinEvent(winevent_hook);
        }
        tracing::info!("hook thread stopped");
    }
}

/// Foreground-window-change callback: recompute the active per-app profile.
unsafe extern "system" fn foreground_changed_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: windows::Win32::Foundation::HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    state::reevaluate_active_profile();
}

/// Classify a mouse message into (button, is_down), or `None` if it is not one
/// of the buttons we remap.
fn classify(msg: u32, info: &MSLLHOOKSTRUCT) -> Option<(MouseButton, bool)> {
    match msg {
        WM_MBUTTONDOWN => Some((MouseButton::Middle, true)),
        WM_MBUTTONUP => Some((MouseButton::Middle, false)),
        WM_XBUTTONDOWN | WM_XBUTTONUP => {
            // High word of mouseData: 1 == XBUTTON1 (back), 2 == XBUTTON2 (fwd).
            let xbutton = (info.mouseData >> 16) as u16;
            let button = match xbutton {
                1 => MouseButton::Back,
                2 => MouseButton::Forward,
                _ => return None,
            };
            Some((button, msg == WM_XBUTTONDOWN))
        }
        _ => None,
    }
}

unsafe extern "system" fn low_level_mouse_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code == HC_ACTION as i32 && suppress_click_scroll(wparam.0 as u32, state::is_paused()) {
        return LRESULT(1);
    }
    if code == HC_ACTION as i32 && !state::is_paused() {
        let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);

        // Ignore events we synthesized ourselves.
        if info.dwExtraInfo != INJECT_SIGNATURE {
            if let Some((button, is_down)) = classify(wparam.0 as u32, info) {
                let map = state::active_map();
                match map.get(button) {
                    Action::PassThrough => {}
                    Action::Disabled => return LRESULT(1),
                    action => {
                        let device = state::device_status();
                        if !crate::hidpp::device::action_supported(
                            action,
                            &device.capabilities,
                            &device.dpi_list,
                        ) {
                            return CallNextHookEx(
                                HHOOK(std::ptr::null_mut()),
                                code,
                                wparam,
                                lparam,
                            );
                        }
                        if is_down {
                            state::inject(action.clone());
                        }
                        // Swallow both the down and the matching up so the
                        // native button event never reaches applications.
                        return LRESULT(1);
                    }
                }
            }
        }
    }

    CallNextHookEx(HHOOK(std::ptr::null_mut()), code, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_and_release_jitter_are_suppressed() {
        let now = Instant::now();
        let mut guard = ClickGuard {
            down: false,
            released: None,
        };
        assert!(!guard.blocks(now));
        guard.update(WM_MBUTTONDOWN, now);
        assert!(guard.blocks(now + Duration::from_secs(1)));
        let release = now + Duration::from_secs(1);
        guard.update(WM_MBUTTONUP, release);
        assert!(guard.blocks(release + Duration::from_millis(20)));
        assert!(guard.blocks(release + Duration::from_millis(149)));
        assert!(!guard.blocks(release + RELEASE_GUARD));
    }
}
