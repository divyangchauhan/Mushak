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

Requirements recorded before implementation. Implementation and validation
results will be appended in the commits that perform the work.
