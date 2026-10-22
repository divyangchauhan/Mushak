# MX Master 2S button preview

The Buttons page displays a fixed-angle product image with clickable markers
and action labels. This is a rendered image, not a rotatable 3D mesh.

Click a marker or label, search for a preset, or build a custom shortcut.
Choosing an action saves it immediately. Close the popover with its close
button, Escape, or a click outside. The profile selector controls button
mappings. The gesture button edits the global tap action, and Configure
gestures opens the swipe and enable settings.

The view uses the reported device name to select the 2S artwork. It shows
connected-device controls only when the device reports them. Disconnected
devices retain the existing offline editing of Back, Forward, and Middle.
Other models and narrow windows retain the button list.

## Verification

Run `cargo test --locked` for capability and popover placement checks.

Run `cargo run --locked -- --ui-smoke` for GPU-rendered interaction checks.
This debug-only command writes screenshots, its result, and isolated test
configuration under `target/ui-smoke`. It starts no mouse hook or HID worker
and does not write the user's settings. Release builds omit this command.

The smoke check clicks markers and labels through Ply's pointer handling,
selects presets and custom shortcuts, and checks profile isolation and saved
configuration. It also renders both themes, disconnected editing, narrow
windows, and the list for another model. Its device status is a fixture.
Physical button behavior, real device reconnection, and Windows UI Automation
remain separate manual checks.

See `assets/devices/README.md` for artwork provenance.
