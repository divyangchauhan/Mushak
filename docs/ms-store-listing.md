# Microsoft Store listing content

Use this copy for the English Partner Center listing.

## Product settings

- Product name: Mushak
- Category: Utilities and tools
- Price: Free
- Audience: Public
- Device family: Windows Desktop

## Short description

Native Windows settings for Logitech HID++ mice. Remap buttons, set supported
DPI and wheel options, and switch mappings by app.

## Description

Mushak is a Windows utility for Logitech HID++ mice. It runs
in the system tray and opens its settings window only when needed.

Use Mushak to:

- Click controls on an MX Master mouse diagram to open an action popover.
- Remap the back, forward, middle, and supported extra buttons to keyboard
  shortcuts, media keys, or custom key combinations.
- Choose ratchet, free-spin, or SmartShift wheel mode and adjust flick
  sensitivity.
- Enable high-resolution scrolling, invert direction, and filter accidental
  wheel movement.
- Map the thumb button to a tap or four swipe directions.
- Choose a DPI value reported by the mouse and read available battery,
  firmware, and connection information.
- Create per-application profiles that switch with the active program.

Mushak discovers mice over Bluetooth, Unifying, Bolt, compatible HID++ receivers,
and wired USB where the mouse supports it. It shows only settings supported by
the connected mouse. Movement gestures require a gesture control with raw XY.
It restores configured settings when the mouse reconnects or wakes from sleep.

The app includes experimental support for the original MX Master, MX Master 2S,
MX Master 3, MX Master 3S, MX Master 4, their Mac and business variants, and MX
Vertical. Protocol tests cover these families. Physical validation across all
models and connections is pending. MX Vertical does not have the Master family's
wheel and gesture features.

The app remembers the last mouse name after disconnection and restart, while
showing it as disconnected. It shows "No device" until a mouse has connected.
Internal touchpads and empty receivers do not count as connected mice.

The resident process uses about 1.5 MB of memory on the tested system. Mushak is
written in Rust and has no webview, telemetry, analytics, advertising, or remote
service.

Do not run Logitech Options, Options+, or G HUB at the same time because they
may compete for the same device controls. Mushak does not implement Flow,
Actions Ring, or haptic intensity controls.

## Product features

- Button remapping and reported extra mouse controls
- Ratchet, free-spin, and SmartShift on compatible mice
- High-resolution scrolling and direction control on compatible mice
- Thumb-button tap and swipe gestures where raw XY is available
- Mouse-reported DPI choices and available live device information
- Profiles that switch with the active application
- Bluetooth, Unifying, Bolt, compatible receivers, and supported wired USB
- Last mouse name retained with disconnected status
- Local configuration with no data collection

## Search terms

1. mx master
2. logitech
3. mouse remap
4. smartshift
5. options plus alternative
6. mouse dpi
7. mx vertical

## Screenshots

Upload these 1366 by 768 PNG files in order:

1. `screenshot-1-buttons.png`
   Caption: Remap the side and middle buttons for each application.
2. `screenshot-2-scroll.png`
   Caption: Choose ratchet, free-spin, or SmartShift and tune the wheel.
3. `screenshot-3-gestures.png`
   Caption: Assign actions to a thumb-button tap and four swipe directions.
4. `screenshot-4-profiles.png`
   Caption: Create and reorder profiles that follow the active application.
5. `screenshot-5-device.png`
   Caption: Read battery, firmware, connection, and DPI from the mouse.

The files are in `docs/store-assets/`. Use `StoreLogo-300x300.png` as the 1:1
app tile icon.

## Links and legal fields

- Website: <https://github.com/divyangchauhan/Mushak>
- Support URL: <https://github.com/divyangchauhan/Mushak/issues>
- Privacy URL: <https://github.com/divyangchauhan/Mushak/blob/main/PRIVACY.md>
- Copyright: Copyright 2026 Divyang Chauhan
- License: MIT, <https://github.com/divyangchauhan/Mushak/blob/main/LICENSE>
- What's new: Visual button configuration for the original MX Master, 2S, 3,
  3S, and 4, including Mac and Business variants. Click a marker or label to
  choose an action. Extra controls appear only when reported by the mouse.
  Includes the Store runtime fix. Physical validation across all models and
  connections remains pending.
