# MX Master family button preview

The Buttons page displays a fixed-angle product image with clickable markers
and action labels for the original MX Master, MX Master 2S, MX Master 3,
MX Master 3S, and MX Master 4. Their Mac and Business variants use the same
layout as their generation. These are rendered images, not rotatable 3D meshes.

Click a marker or label, search for a preset, or build a custom shortcut.
Choosing an action saves it immediately. Close the popover with its close
button, Escape, or a click outside. The profile selector controls button
mappings. The gesture button edits the global tap action, and Configure
gestures opens the swipe and enable settings.

The reported device name selects one of three shared body diagrams. The
original MX Master and 2S use the 2S artwork. The 3 and 3S use the 3S artwork.
The 4 has its own artwork and marker positions for its separate gesture
button and Haptic Sense Panel. The diagrams describe button positions;
color and surface details can differ from the connected mouse.

Connected-device controls appear only when the device reports them. A thumb
button that supports ordinary mapping instead of gestures opens its reported
button mapping. The Haptic Sense Panel opens either its button mapping or
the global tap action according to the reported gesture control. This change
does not add haptic feedback effects or Actions Ring support. Controls without
an artwork marker remain editable in a list below the diagram.

The top button behind the scroll wheel is labeled Wheel mode button on Master
diagrams. It uses the per-profile action picker, including while disconnected.
Dedicated DPI Change and DPI Switch controls use named rows in the generic list
or below the diagram when the connected mouse reports them as configurable.
Choose Pass through to restore the native wheel-mode or DPI action. Pausing
Mushak or quitting also restores native behavior.

Disconnected devices retain offline editing of saved extra-button mappings,
Back, Forward, Middle, and all five saved gesture actions. The gesture marker
remains visible on every Master
diagram while offline, and Configure gestures opens the saved tap and swipe
mappings. The fallback button list also includes Gesture button tap for every
connected mouse reporting gesture support, including unnamed future models.
Tap and swipe mappings and the threshold remain editable while gesture input
is disabled. Saving these settings does not enable gesture recognition.
Other mice, unknown future Master generations, and narrow windows retain the
button list. Live hardware settings always require a connected device.

## Verification

Run `cargo test --locked` for model recognition, capability gating, gesture
and haptic target selection, extra-control coverage, and popover placement.

Run `cargo run --locked -- --ui-smoke` for GPU-rendered interaction checks.
This debug-only command writes screenshots, its result, and isolated test
configuration under `target/ui-smoke`. It starts no mouse hook or HID worker
and does not write the user's settings. Release builds omit this command.

The smoke check clicks every generation's markers in both themes, including
Mac and Business names. It checks the panel's saved mapping and profile
isolation, additional reported controls, presets and custom shortcuts,
search reset, disconnected editing, narrow windows, other-mouse fallback,
gesture-only devices, all five gesture actions while offline and disabled,
DPI Change and DPI Switch mappings, the offline top button, saved extra controls,
and restoring Pass through without changing another profile.
Device status comes from fixtures. Physical button behavior, real device
reconnection, and Windows UI Automation remain separate manual checks.

See `assets/devices/README.md` for artwork provenance.
