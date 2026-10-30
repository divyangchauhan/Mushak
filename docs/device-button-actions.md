# Device button actions preview

The button picker offers Shift wheel mode when the mouse reports working
SmartShift support. It switches between fixed ratchet and free-spin modes.
Change pointer speed appears when the sensor reports at least two DPI values.
Choose two different speeds, then select Change pointer speed to save the binding.
Native action removes the remap and restores the physical button default.

Hardware actions can also be assigned to gesture outcomes. Native action on a
gesture outcome means no injected action, as with the previous Pass through option.

Bindings belong to the selected button profile. Gesture outcomes retain their
existing global scope. The actions use the live mouse state, so the first press
selects speed one if the current DPI matches neither configured speed.

Temporary hardware values survive profile switches and periodic device refresh.
Reconnect, app restart, or editing device settings restores saved device defaults.
The actions do not write those temporary values to config.toml. Sleep without a
detected disconnect reasserts the temporary values when the mouse wakes.

A mapping saved for unsupported hardware leaves the physical button native.
DPI pairs must both be reported by the connected sensor. Reopen the picker to
choose replacement values after changing mice. No model-name whitelist is used.

## Manual hardware check

1. Connect the mouse and open Buttons. Select the wheel-mode button.
2. Assign Shift wheel mode. Press it twice and confirm ratchet then free-spin.
3. Assign Change pointer speed with two noticeably different DPI values.
   Press twice and check pointer movement and the live Device tab DPI reading.
4. Switch application profiles, then wait at least 30 seconds. Confirm the chosen
   speed remains until the next press or a device settings edit.
5. Select Native action and confirm the physical button's default behavior returns.
6. Restart Mushak or reconnect the mouse and confirm saved device defaults return.

Simulated HID++ tests and rendered UI checks cannot establish physical operation
on every Logitech model. Each model and connection still needs a hardware check.
