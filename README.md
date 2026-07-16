# Mushak

A lightweight Windows replacement for Logitech Options+, targeting the **MX
Master 2S** mouse. Pure Rust, native GUI (egui — no Electron, no webview).

- Binary ~3.8 MB, idle RAM well under 30 MB.
- Button remapping via a low-level mouse hook (works even without HID++).
- HID++ 2.0 device control: SmartShift, DPI, hi-res/invert scrolling, battery,
  firmware, and software thumb-button gestures.
- Per-application profiles, system tray, start-with-Windows.

## Requirements

- Windows 10/11 (x64).
- The MX Master 2S connected either via a **Logitech Unifying receiver**
  (USB `046D:C52B`) or directly over **Bluetooth** (`046D:B019`). Both are
  supported and auto-detected; if a receiver is plugged in but the mouse is on
  Bluetooth, Mushak falls back to Bluetooth automatically.

## Build

Install the Rust toolchain (MSVC) from <https://rustup.rs>, then:

```powershell
cargo build --release
# binary at target\release\mushak.exe
```

The release profile is size-optimized (`opt-level = "z"`, LTO, `strip`,
`panic = "abort"`).

Run it:

```powershell
.\target\release\mushak.exe
```

Mushak runs **in the system tray** with no window. Double-click the tray icon
(or use the tray menu → *Open Settings*) to open the settings window; it launches
as a separate process and closing it just closes that window. Use the tray menu
→ *Quit* to exit Mushak entirely.

### Running as administrator

The low-level mouse hook cannot intercept input directed at windows running at a
**higher integrity level** than Mushak. If you want remapping to work over
elevated/admin apps (Task Manager, elevated terminals, some installers), run
`mushak.exe` **as administrator**.

## Settings

The window has five tabs:

- **Buttons** — remap Back / Forward / Middle per profile. Each can be a preset
  shortcut, a media key, *Pass through*, or *Disabled*, or a custom modifier+key
  chord.
- **Scroll** — SmartShift mode (Always ratchet / Always freespin / SmartShift
  with a flick-sensitivity slider), high-resolution scrolling, invert direction.
- **Gestures** — actions for tap / swipe up / down / left / right of the thumb
  button, plus the tap-vs-swipe threshold. Defaults mirror Options+ (tap = Task
  View, left/right = switch virtual desktop, up/down = volume).
- **Profiles** — add per-application profiles matched by executable name (e.g.
  `chrome.exe`). Use *Pick a window* and click the target app within 4 seconds
  to capture its process name. Reorder / delete profiles; first match wins.
- **Device** — connection type, battery, firmware, live DPI (slider bounded by
  the device's own supported list), and the discovered HID++ feature table.

Config is saved to `%APPDATA%\Mushak\config.toml` (human-editable TOML) and
applied on launch and reconnect.

## Logs

Debug logs, including hex dumps of every HID++ packet, are written to
`%APPDATA%\Mushak\logs\mushak.log.<date>`. These are invaluable for debugging
against a specific firmware — HID++ behavior varies. Override the level with the
`RUST_LOG` environment variable (e.g. `RUST_LOG=mushak=trace`).

## How it works

Mushak runs as **two processes** so the always-on part stays tiny (idle ≈ 6 MB
private / 16 MB working set — a GPU-backed GUI can't hit that if it's always
loaded):

- **Resident process** (`mushak.exe`) — the tray plus three worker threads,
  driven by a lightweight Win32 message pump (no GUI, no GPU context):
  1. *Mouse-hook thread* — `WH_MOUSE_LL` intercepts and swallows the side/middle
     buttons, plus a `SetWinEventHook` on foreground changes so per-app profiles
     switch automatically. The callback does minimal work.
  2. *Injector thread* — synthesizes keystrokes via `SendInput`, tagged so the
     hook ignores its own injected events.
  3. *HID device thread* — owns the raw HID handle; runs HID++ 2.0 feature
     discovery, applies settings, and reads notifications (battery, diverted
     gesture events). Reconnects automatically.
- **Settings process** (`mushak.exe --settings`) — launched on demand from the
  tray and closed when you're done. Only this short-lived process loads the
  egui/OpenGL GUI.

The two communicate through files in `%APPDATA%\Mushak`: the settings process
writes `config.toml` (the resident watches it and re-applies within ~1 s), and
the resident publishes live device status to `status.json` (the settings window
reads it for the Device tab).

### Gestures

The 2S has no usable hardware gesture feature for this purpose, so gestures are
done in software: the thumb button (control CID `0x00C3`) is *diverted* via
HID++ feature `0x1B04` (REPROG_CONTROLS_V4), along with its raw XY. While the
button is held, the mouse reports raw motion to Mushak (the cursor stays put);
on release the accumulated vector is classified into a tap or a directional
swipe and the mapped action is injected. As a safety net, if raw XY ever arrives
while the button is *not* held, the motion is re-injected as cursor movement so
the pointer can never freeze.

> Note: this particular firmware (MPM 12.01) also exposes feature `0x6501`
> (GESTURE_2), but Mushak uses the `0x1B04` divert approach for full software
> control of the outcomes.

## Conflicts

Do **not** run Logitech Options / Options+ / LogiOptionsMgr or G HUB at the same
time — they hold the HID++ diverts and will fight Mushak over the device. Mushak
detects these and shows a warning banner. Quit them for gestures / SmartShift /
DPI to work reliably.

### Diverts left behind by other software

Options+ *diverts* the side buttons over HID++ so it can remap them itself: a
diverted button stops sending a normal mouse event and instead reports as an
HID++ control event. That divert lives in the mouse and survives the app — if
you quit Options+, the buttons stay diverted until the mouse power-cycles, which
leaves them dead for everything else (including Mushak's hook, which only sees
real mouse events).

So on every connect Mushak asserts the divert state of *all* reprogrammable
controls, not just the ones it uses: the thumb button is diverted when gestures
are enabled, and every other button is explicitly restored to native reporting.
This is what makes remapping work on a mouse that Options+ has touched.

## Reconnection

The 2S sleeps aggressively. Mushak re-applies diverts and settings on a 30-second
cadence (they reset when the mouse powers down) and re-runs discovery if the HID
handle drops. Over a Unifying receiver, wake is also signalled by the receiver's
HID++ 1.0 `0x41` notification.

## Limitations

- Tested primarily over Bluetooth. Receiver mode is implemented (device-index
  probing 1..6) but the receiver's short-report `0x41` wake notification is only
  seen if the short collection is available; the periodic re-apply is the
  backstop either way.
- Applying the saved config on launch *does* change the mouse (DPI, SmartShift,
  hi-res) — this is intentional.

## License

MIT.
