//! Native window chrome for the settings window.
//!
//! The design draws its own 46px title bar, but miniquad's `Conf` has no
//! borderless option on Windows — `get_win_style()` always sets `WS_CAPTION`
//! for a non-fullscreen window. It does expose the raw `HWND`, so we take the
//! window over after creation and restyle it ourselves.

use ply_engine::prelude::miniquad;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::HiDpi::AdjustWindowRectExForDpi;
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageW, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_STYLE, HTCAPTION,
    SET_WINDOW_POS_FLAGS, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOZORDER, SW_MINIMIZE, WINDOW_EX_STYLE,
    WM_NCLBUTTONDOWN, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP,
    WS_SIZEBOX, WS_SYSMENU, WS_VISIBLE,
};

/// The main window's HWND, or `None` before the window exists.
fn hwnd() -> Option<HWND> {
    let raw = miniquad::window::windows_hwnd();
    if raw.is_null() {
        return None;
    }
    Some(HWND(raw))
}

/// Strip the OS title bar and size the window to the design's dimensions,
/// keeping the resize border and the taskbar's minimise/restore animations
/// (which need `WS_MINIMIZEBOX`/`WS_MAXIMIZEBOX`).
///
/// `logical_w`/`logical_h` are design points. miniquad's `Conf` sizes the
/// window in *physical* pixels while `screen_width()` — what Ply lays out
/// against — reports *logical* points, so asking Conf for 968 yields a 484pt
/// canvas on a 200% display and the design overflows it. Scaling here, once the
/// window exists and `dpi_scale()` is known, is what keeps the two in step.
///
/// Safe to call more than once; call it after the first frame, once miniquad
/// has actually created the window.
pub fn make_frameless(logical_w: f32, logical_h: f32) {
    let Some(h) = hwnd() else {
        tracing::warn!("no HWND yet; window stays decorated");
        return;
    };
    let style = WS_POPUP
        | WS_VISIBLE
        | WS_SYSMENU
        | WS_MINIMIZEBOX
        | WS_MAXIMIZEBOX
        | WS_SIZEBOX
        | WS_CLIPSIBLINGS
        | WS_CLIPCHILDREN;
    let scale = miniquad::window::dpi_scale().max(1.0);
    // SetWindowPos sizes the *outer* window, but Ply lays out against the
    // client area, so ask Windows how much the sizing border adds and grow by
    // it — otherwise the canvas comes up a frame-width short of the design.
    let mut r = RECT {
        left: 0,
        top: 0,
        right: (logical_w * scale) as i32,
        bottom: (logical_h * scale) as i32,
    };
    unsafe {
        SetWindowLongPtrW(h, GWL_STYLE, style.0 as isize);
        let dpi = (96.0 * scale) as u32;
        if AdjustWindowRectExForDpi(&mut r, style, false, WINDOW_EX_STYLE(0), dpi).is_err() {
            tracing::warn!("AdjustWindowRectExForDpi failed; window may be a few px short");
        }
        let (cx, cy) = (r.right - r.left, r.bottom - r.top);
        // Without SWP_FRAMECHANGED the non-client area is not recalculated and
        // the caption keeps painting until the window is moved.
        let _ = SetWindowPos(
            h,
            None,
            0,
            0,
            cx,
            cy,
            SET_WINDOW_POS_FLAGS(SWP_FRAMECHANGED.0 | SWP_NOMOVE.0 | SWP_NOZORDER.0),
        );
        tracing::debug!("window restyled frameless: {cx}x{cy}px @ {scale}x");
    }
}

/// Start an OS-driven window drag. Handing the drag to Windows via
/// `WM_NCLBUTTONDOWN`/`HTCAPTION` gets us snap-to-edge and Aero Snap for free,
/// which tracking the pointer ourselves would not.
pub fn begin_drag() {
    let Some(h) = hwnd() else { return };
    unsafe {
        // The pointer is captured by our client area from the button press;
        // Windows ignores the non-client drag unless we let go of it first.
        let _ = ReleaseCapture();
        SendMessageW(h, WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), LPARAM(0));
    }
}

pub fn minimize() {
    let Some(h) = hwnd() else { return };
    unsafe {
        let _ = ShowWindow(h, SW_MINIMIZE);
    }
}
