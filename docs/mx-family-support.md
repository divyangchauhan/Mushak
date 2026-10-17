# Device discovery and MX mouse support

Requested on 2026-10-14. Implement this work in the order below, with separate
commits for the requirements, the discovery and settings fixes, and MX family
support.

## Required behavior

- Show the connected mouse's reported name in the settings window. Do not use
  an MX Master model name as the fallback for an unidentified device.
- Discover HID++ mice without restricting discovery to the MX Master 2S product
  ID. Try every eligible interface rather than keeping one receiver and one
  Bluetooth candidate.
- Accept Bluetooth, Unifying receivers, Bolt receivers, other HID++ receivers,
  and direct wired USB connections. Preserve the actual connection kind in
  status and labels. Receiver discovery must skip keyboards and empty slots.
- Obtain DPI choices from the mouse. Offer only reported, supported values.
  If discovery fails or DPI configuration is unavailable, hide the DPI control.
  Do not substitute a generic DPI range or send an unvalidated DPI value.
- Hide settings that the connected mouse does not support. Apply this to
  SmartShift, high-resolution scrolling, inversion, gestures, DPI, and controls
  specific to a model. Capability decisions must come from feature discovery,
  successful capability reads, and control enumeration.
- Distinguish an unavailable device from a sleeping device. Do not report every
  discovery failure as sleep.
- After the discovery and settings fixes, implement support for the original
  MX Master, MX Master 2S, MX Master 3, MX Master 3S, MX Master 4, their Mac and
  business variants, and MX Vertical. Use reported capabilities instead of an
  exact device-name whitelist. MX Vertical is a separate MX model and must not
  inherit unsupported Master wheel or gesture settings.
- Handle feature variants needed by these mice, including battery reporting,
  SmartShift, DPI, and available reprogrammable controls. Keep existing config
  files readable. Preserve native behavior when a feature is absent.

## Scope and constraints

Connection discovery covers HID++ interfaces that Mushak can communicate with.
An arbitrary third-party mouse or receiver does not implement Logitech HID++.
Do not claim configurable DPI, battery reporting, or other vendor settings for
hardware that does not expose a supported protocol.

The current app manages one active HID++ mouse at a time. Multiple candidates
must be tried reliably, but a device picker, simultaneous per-device profiles,
Flow, and a replacement for Logitech's Actions Ring are outside this change.
Existing Windows button hooks remain global.

## Acceptance and validation

- Test candidate classification for Bluetooth, Unifying, Bolt, other receivers,
  and USB. Test multiple candidates and receiver mouse selection.
- Test reported-name parsing, unknown/disconnected labels, capability visibility,
  discrete DPI values and encoded DPI ranges, feature variants, and config
  compatibility. Include unsupported-feature cases.
- Build the Windows release and run the repository's relevant automated checks.
- Check the locally available mouse through real HID++ discovery without
  changing the installed package or its persistent configuration.
- For every new model and each available connection, verify detection, name,
  battery, supported DPI choices, button mappings, applicable wheel controls,
  applicable gestures, sleep/wake, reconnect, and native behavior after exit.
- Record automated checks separately from physical device checks. An added PID
  or a passing synthetic test does not prove a model works on Windows.

## Implementation record

### Discovery and settings

- Replaced the 2S product-ID filter with Logitech vendor-defined HID collection
  discovery. Retain all candidate paths, prefer long-report collections, classify
  the HID bus and receiver identity, and verify a HID++ 2.0 mouse before use.
- Read names and device kinds through `0x0005`. Receiver probing skips keyboards
  and empty slots. An unrecognized USB receiver can be identified by a responding
  mouse slot without requiring a new product-ID entry.
- Publish capabilities through status IPC. Hide unsupported Scroll and Gestures
  sections and unsupported DPI, battery, and individual wheel controls.
- DPI sliders select indices into the device's validated choices. Empty or
  malformed lists never produce fallback DPI writes. Wheel scaling uses the
  discovered multiplier, with no model-specific multiplier fallback.
- Added `mushak.exe --diagnose-device` for read-only name, connection, feature,
  capability, and DPI discovery. It does not load config, install hooks, or apply
  device settings.
- Automated validation passed 25 tests. The existing single-instance test was
  excluded because the installed resident owns its named mutex. The test run
  did not stop or replace the installed app.
- `cargo build --release --locked` and `git diff --check` passed.
- The read-only hardware check returned `no Logitech HID++ interface found`.
  Windows lists the old 2S Bluetooth and Unifying mouse entries as unavailable;
  the active pointing device is a Synaptics touchpad. Physical validation is
  pending a connected mouse.

### MX family implementation

- Discovery accepts reported HID++ mouse identities across the Master family,
  Mac and business variants, MX Vertical, and other HID++ mice. Product IDs do
  not decide which mouse settings appear.
- Standard HID mouse collections also provide a name and connection fallback
  for other vendors and connections without HID++. Their unknown vendor settings
  remain hidden. A receiver product string identifies the receiver, not its
  paired mouse model; the fallback labels that case as a mouse via the receiver.
- Added battery voltage `0x1001` and unified battery `0x1004`. Voltage and coarse
  battery levels remain voltage and levels in the UI, with no invented percentage.
- Added enhanced SmartShift `0x2111`, with its different read/write functions.
  Preserve the existing torque byte when changing mode or sensitivity.
- Added extended adjustable DPI `0x2202`, paged DPI lists, sensor discovery, and
  preservation of independent Y resolution and lift-off distance on DPI writes.
- Enumerated button controls populate the settings rows. Extra mappings live in
  each profile under `buttons.controls`, keyed by control ID. Existing configs
  remain readable. Foreground profile changes reapply extra button reporting.
- Select gestures by a reported gesture control and raw-XY support. A DPI button
  that reports raw movement remains a DPI button. Gesture buttons without raw XY
  can be remapped as ordinary extra buttons, with movement gestures hidden.
- Pause and exit restore extra buttons to native reporting. Shutdown waits for
  the device thread to finish restoration. Reconnect notifications schedule
  reapplication after the current request, avoiding nested HID++ requests.
- Model names, DPI limits, wheel multipliers, and physical button positions no
  longer come from a fixed MX Master 2S layout.
- The binary and MSIX descriptions now describe HID++ mice. Existing Scoop,
  WinGet, and Store listing documents still describe their released 2S version.
  This work has not updated any published package or installed application.

The protocol tests simulate exchanges for the original Master, 2S, 3, 3S, 4,
Mac/business names, MX Vertical, and an unknown future mouse. They check discovery,
receiver keyboard exclusion, feature indices, valid DPI writes, enhanced
SmartShift writes, extended DPI preservation, and temporary control restoration.
These are protocol fixtures, not recordings of every model or firmware.

The single-instance test now uses test-only mutex and event names, so the entire
suite can run while the installed resident remains active.

The later live read-only probe found `Wireless Mouse MX Master 2S` over Bluetooth,
firmware `MPM 12.01.B0006`, battery 50%, and 77 reported DPI choices from 200 to
4000 in steps of 50. It reported wheel multiplier 8, SmartShift, high-resolution
scrolling, inversion, raw-XY gestures, and Middle, Back, Forward, and wheel-mode
buttons. The [captured JSON](validation/mx-master-2s-bluetooth.json) records this
discovery. This check applied no device settings and did not replace the installed
resident.

Final checks passed `cargo test --locked`, with 37 tests and no exclusions,
`cargo build --release --locked`, formatting checks for every changed Rust file,
and `git diff --check`. The release executable also completed read-only discovery
of the 2S Bluetooth device with exit code 0 and the same 77 DPI choices and wheel
multiplier. Use `Start-Process -Wait` with redirected output when collecting
diagnostics from the Windows GUI-subsystem release in PowerShell.

Strict `cargo clippy --locked --all-targets -- -D warnings` remains blocked by
existing repository lints. The original checkout reproduces eight lint issues;
six remain here: two manual Default implementations, a redundant format call,
the action picker's complex type, the wheel module's pre-existing Rust minimum
version mismatch, and test-module ordering in the single-instance module. No new
Clippy lint remains in the added implementation.

## Physical validation status

### Disconnected mouse identity

The local test build exposed a discovery bug. The standard HID fallback accepted
the built-in `HIDI2C Device` touchpad as a connected mouse. It also inferred a
connected mouse from a receiver's USB collection, even with no responding mouse.

The fallback now accepts present, openable USB and Bluetooth mouse collections,
excludes internal I2C devices and named touchpads, and leaves receiver connection
verification to HID++ slot discovery. A receiver without a responding HID++ mouse
does not prove that a mouse is connected. Generic receivers without a supported
connection-status protocol cannot report live mouse presence through this fallback.

After detecting a mouse, Mushak saves its name in `%APPDATA%\Mushak\last-device.json`.
Disconnection and app restart retain that name with a `Disconnected` status.
Saved identity never restores connection state, battery readings, DPI, or other
live settings. Without a previously detected mouse, the header and status show
`No device`. The old `status.json` is not imported as history because it may
contain the incorrect touchpad identity. Startup replaces that stale status.

The I2C regression failed before the fix. All 41 tests now pass, including
receiver exclusion, external USB and Bluetooth identity, disconnect and restart
history, replacement by a different mouse, and invalid history recovery.
The release resident also passed two isolated local checks with the mouse absent.
A fresh data directory reported no name and `connected: false`. A separate data
directory containing a test identity retained that name with `connected: false`
through discovery retries. These checks did not modify the user's device history.

Behavior testing still needs a tester result from this branch. Earlier 2S testing
applies to earlier releases and does not validate this implementation.

| Mouse | Connections to check | Result |
| --- | --- | --- |
| Original MX Master | Bluetooth, compatible receiver | Pending |
| MX Master 2S | Bluetooth, Unifying | Bluetooth discovery passed; behavior and Unifying pending |
| MX Master 3 and variants | Bluetooth, compatible receiver | Pending |
| MX Master 3S and variants | Bluetooth, Bolt | Pending |
| MX Master 4 and variants | Bluetooth, Bolt | Pending |
| MX Vertical | Bluetooth, Unifying, wired USB | Pending |
| Other HID++ mice/receivers | Available connection paths | Pending |

Capture `--diagnose-device` output and logs with the mouse name, firmware,
Windows version, connection, and the acceptance checklist above. Settings that
require a protocol Mushak does not implement remain unavailable. This work does
not add Flow, hardware gesture feature `0x6501`, haptic intensity controls, or a
replacement for Actions Ring.

## Protocol references

Consulted these primary project sources for wire layouts and device evidence.
Mushak's implementation and protocol tests were written independently.

- [Solaar feature definitions](https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/hidpp20_constants.py)
- [Solaar protocol handling](https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/hidpp20.py)
- [Solaar setting protocols](https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/settings_templates.py)
- [Original MX Master device report](https://github.com/pwr-Solaar/Solaar/blob/master/docs/devices/Wireless%20Mouse%20MX%20Master%20B012.txt)
- [MX Master 4 business device report](https://github.com/pwr-Solaar/Solaar/issues/3264)
- [Logitech MX Vertical specifications](https://www.logitech.com/content/dam/logitech/en/business/pdf/mx-vertical-advanced-ergo-mouse-data-sheet.pdf)
