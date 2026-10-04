# MX Master 3 test checklist

This branch adds the MX Master 3 Bluetooth ID `046D:B023`. Its Unifying
receiver uses the existing `046D:C52B` path. Do not call the model supported
until a real MX Master 3 passes this checklist.

## Before testing

1. Record the Windows version and whether the mouse uses Bluetooth or the
   Unifying receiver.
2. Quit Logitech Options, Options+, G HUB, and any other mouse utility.
3. Install the test build, then open Mushak from the system tray.
4. Open the Device tab. It should show `Wireless Mouse MX Master 3`, the
   connection type, firmware, battery, and current DPI.

## Checks

- Remap Back, Forward, and Middle. Check each mapping in two applications.
- Try Always ratchet, Always freespin, and SmartShift. Move the flick
  sensitivity slider and confirm the mouse changes.
- Change DPI and confirm the Device tab reports the new value.
- Turn high-resolution scrolling on and off. Check slow movement, a fast spin,
  direction reversal, and invert direction.
- Test thumb-button tap and all four swipe directions.
- Leave the mouse idle until it sleeps, then wake it. Repeat the checks that
  change device state.
- Disconnect and reconnect the mouse.
- Quit Mushak from the tray. Normal buttons and scrolling must still work.

Run the checklist once over Bluetooth and once with the receiver if both are
available.

## What to send back

Copy this block into the reply or issue:

```text
Windows version:
Connection: Bluetooth / Unifying receiver
Firmware shown by Mushak:
Device name shown by Mushak:
Buttons: pass / fail, notes
SmartShift: pass / fail, notes
DPI: pass / fail, notes
High-resolution scrolling: pass / fail, notes
Invert scrolling: pass / fail, notes
Thumb gestures: pass / fail, notes
Sleep and wake: pass / fail, notes
Disconnect and reconnect: pass / fail, notes
Behavior after quitting Mushak: pass / fail, notes
```

Attach the newest file from `%APPDATA%\Mushak\logs\` after reproducing any
failure. Say what you clicked or changed just before the failure.
