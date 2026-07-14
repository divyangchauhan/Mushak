//! Keyboard-injection worker.
//!
//! The low-level mouse hook must return fast, so it never calls `SendInput`
//! directly — it queues an [`Action`] here and this thread performs the
//! injection. Injected events are tagged with [`INJECT_SIGNATURE`] in
//! `dwExtraInfo` so the hook can recognize and ignore them.

use crate::config::{Action, Modifier};
use crossbeam_channel::{Receiver, Sender};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, MOUSEEVENTF_MOVE, MOUSEINPUT, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU,
    VK_SHIFT,
};

/// Marker written to `dwExtraInfo` on every event we synthesize, so our own
/// hook does not re-process them. ASCII "MUSH".
pub const INJECT_SIGNATURE: usize = 0x4D55_5348;

/// Spawn the injector thread and return the channel used to feed it.
pub fn spawn() -> Sender<Action> {
    let (tx, rx) = crossbeam_channel::unbounded::<Action>();
    std::thread::Builder::new()
        .name("injector".into())
        .spawn(move || run(rx))
        .expect("spawn injector thread");
    tx
}

fn run(rx: Receiver<Action>) {
    tracing::debug!("injector thread started");
    while let Ok(action) = rx.recv() {
        perform(&action);
    }
    tracing::debug!("injector thread stopped");
}

fn modifier_vk(m: Modifier) -> VIRTUAL_KEY {
    match m {
        Modifier::Ctrl => VK_CONTROL,
        Modifier::Alt => VK_MENU,
        Modifier::Shift => VK_SHIFT,
        Modifier::Win => VK_LWIN,
    }
}

fn key_event(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let flags = if up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECT_SIGNATURE,
            },
        },
    }
}

/// Execute an action by synthesizing the corresponding key chord.
pub fn perform(action: &Action) {
    let (mods, vk) = match action {
        Action::Key { mods, vk } => (mods, *vk),
        // PassThrough / Disabled never reach the injector.
        _ => return,
    };

    let mut inputs: Vec<INPUT> = Vec::with_capacity(mods.len() * 2 + 2);

    // Modifiers down (in order), key down, key up, modifiers up (reverse).
    for m in mods {
        inputs.push(key_event(modifier_vk(*m), false));
    }
    inputs.push(key_event(VIRTUAL_KEY(vk), false));
    inputs.push(key_event(VIRTUAL_KEY(vk), true));
    for m in mods.iter().rev() {
        inputs.push(key_event(modifier_vk(*m), true));
    }

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        tracing::warn!(
            "SendInput injected {}/{} events for {:?}",
            sent,
            inputs.len(),
            action
        );
    } else {
        tracing::debug!("injected {:?}", action);
    }
}

/// Move the cursor by a relative amount. Used as a safety net when raw XY is
/// diverted globally (rather than only while the gesture button is held), so
/// the pointer can never end up frozen.
pub fn move_cursor(dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: INJECT_SIGNATURE,
            },
        },
    };
    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}
