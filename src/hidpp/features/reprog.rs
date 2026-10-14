//! Feature 0x1B04 REPROG_CONTROLS_V4: control enumeration and divert.
//!
//! function 0 getControlCount
//! function 1 getControlInfo(index) -> cid, tid, flags, pos, group, gmask, addl
//! function 3 setControlReporting(cid, flags, remap)
//!
//! setControlReporting flags (logiops semantics): each divert has a value bit
//! and a "change" bit that must be set for the value to take effect.

use crate::hidpp::device::Device;
use anyhow::{anyhow, Result};

// getControlInfo `flags` bits.
const KEY_DIVERTABLE: u8 = 0x20;
const KEY_PERSISTENTLY_DIVERTABLE: u8 = 0x40;
const KEY_VIRTUAL: u8 = 0x80;
// getControlInfo `additional_flags` bits.
const KEY_RAW_XY: u8 = 0x01;

// setControlReporting flags. Bit 0x04 (persistently diverted) is deliberately
// never set: Mushak only ever takes a temporary divert.
const RC_TEMP_DIVERT: u8 = 0x01;
const RC_CHANGE_TEMP_DIVERT: u8 = 0x02;
const RC_CHANGE_PERSIST_DIVERT: u8 = 0x08;
const RC_RAWXY_DIVERT: u8 = 0x10;
const RC_CHANGE_RAWXY_DIVERT: u8 = 0x20;

/// One control as reported by getControlInfo.
#[derive(Debug, Clone, Copy)]
pub struct ControlInfo {
    pub cid: u16,
    pub tid: u16,
    pub flags: u8,
    pub additional_flags: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ButtonInfo {
    pub cid: u16,
    pub slot: String,
    pub name: String,
}

pub(crate) fn gesture_control(controls: &[ControlInfo]) -> Option<u16> {
    // A DPI button's raw XY adjusts sensitivity. Do not make it a gesture pad.
    controls
        .iter()
        .filter(|c| c.supports_gestures() && matches!(c.cid, 0x00C3 | 0x00D0 | 0x01A0))
        .min_by_key(|c| if c.cid == 0x00C3 { 0 } else { 1 })
        .map(|c| c.cid)
}

pub(crate) fn button_choices(controls: &[ControlInfo], gesture: Option<u16>) -> Vec<ButtonInfo> {
    controls
        .iter()
        .filter_map(|c| {
            if c.virtual_control() || Some(c.cid) == gesture {
                return None;
            }
            let (slot, name) = match c.cid {
                0x0052 => ("middle".to_owned(), "Middle click".to_owned()),
                0x0053 | 0x0054 => ("back".to_owned(), "Back button".to_owned()),
                0x0055 | 0x0056 => ("forward".to_owned(), "Forward button".to_owned()),
                _ if c.divertable() && !matches!(c.cid, 0x0050 | 0x0051) => {
                    let name = match c.cid {
                        0x00C3 | 0x00D0 => "Gesture button".to_owned(),
                        0x00C4 => "Wheel mode button".to_owned(),
                        0x00ED => "DPI button".to_owned(),
                        0x01A0 => "Haptic panel".to_owned(),
                        _ => format!("Button {:#06x}", c.cid),
                    };
                    (format!("control_{:04x}", c.cid), name)
                }
                _ => return None,
            };
            Some(ButtonInfo {
                cid: c.cid,
                slot,
                name,
            })
        })
        .collect()
}

impl ControlInfo {
    pub(crate) fn supports_gestures(&self) -> bool {
        self.divertable() && self.raw_xy() && !self.virtual_control()
    }
    /// The control can be temporarily diverted to HID++ reporting.
    fn divertable(&self) -> bool {
        self.flags & KEY_DIVERTABLE != 0
    }

    /// The control supports a divert that survives a power cycle.
    fn persistently_divertable(&self) -> bool {
        self.flags & KEY_PERSISTENTLY_DIVERTABLE != 0
    }

    /// The control can report raw sensor movement while held.
    fn raw_xy(&self) -> bool {
        self.additional_flags & KEY_RAW_XY != 0
    }

    /// Not a physical button — nothing to restore, so we leave these alone.
    fn virtual_control(&self) -> bool {
        self.flags & KEY_VIRTUAL != 0
    }
}

impl Device {
    fn reprog_index(&self) -> Result<u8> {
        self.features
            .reprog
            .ok_or_else(|| anyhow!("no REPROG_CONTROLS_V4 feature"))
    }

    pub(crate) fn control_count(&self) -> Result<u8> {
        let idx = self.reprog_index()?;
        let rep = self.request(idx, 0, &[])?;
        Ok(rep.param(0))
    }

    pub(crate) fn control_info(&self, control_index: u8) -> Result<ControlInfo> {
        let idx = self.reprog_index()?;
        let rep = self.request(idx, 1, &[control_index])?;
        if rep.raw.len() < 13 {
            return Err(anyhow!("truncated control information"));
        }
        Ok(ControlInfo {
            cid: ((rep.param(0) as u16) << 8) | rep.param(1) as u16,
            tid: ((rep.param(2) as u16) << 8) | rep.param(3) as u16,
            flags: rep.param(4),
            additional_flags: rep.param(8),
        })
    }

    /// Enumerate + log every control the firmware exposes.
    pub(crate) fn enumerate_controls(&self) -> Vec<ControlInfo> {
        let count = match self.control_count() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("getControlCount failed: {e:#}");
                return Vec::new();
            }
        };
        tracing::info!("REPROG_CONTROLS_V4: {count} controls");
        let mut controls = Vec::new();
        for i in 0..count {
            match self.control_info(i) {
                Ok(ci) => {
                    tracing::info!(
                        "  control[{i:02}] cid={:#06x} tid={:#06x} flags={:#04x} addl={:#04x}",
                        ci.cid,
                        ci.tid,
                        ci.flags,
                        ci.additional_flags
                    );
                    controls.push(ci);
                }
                Err(e) => tracing::debug!("  control[{i:02}] read failed: {e:#}"),
            }
        }
        controls
    }

    fn set_control_reporting(&self, cid: u16, flags: u8) -> Result<()> {
        let idx = self.reprog_index()?;
        // params: cid(be16), flags, remap(be16 = 0)
        self.request(idx, 3, &[(cid >> 8) as u8, (cid & 0xFF) as u8, flags, 0, 0])?;
        Ok(())
    }

    /// Divert one control to HID++ reporting, or restore its native behavior.
    ///
    /// Only the change bits a control actually supports are sent — a change bit
    /// for an unsupported divert is rejected by the firmware.
    fn set_divert(&self, ci: &ControlInfo, divert: bool, raw_xy: bool) -> Result<()> {
        if !ci.divertable() {
            return Ok(());
        }
        let mut flags = RC_CHANGE_TEMP_DIVERT;
        if divert {
            flags |= RC_TEMP_DIVERT;
        }
        if ci.raw_xy() {
            flags |= RC_CHANGE_RAWXY_DIVERT;
            if divert && raw_xy {
                flags |= RC_RAWXY_DIVERT;
            }
        }
        if ci.persistently_divertable() {
            // Clear any persistent divert; never set one.
            flags |= RC_CHANGE_PERSIST_DIVERT;
        }
        tracing::debug!(
            "{} CID {:#06x} (flags={flags:#04x})",
            if divert { "divert" } else { "restore" },
            ci.cid
        );
        self.set_control_reporting(ci.cid, flags)
    }

    /// Divert gesture and configured extra buttons. Restore other controls.
    ///
    /// Restoring the others is not merely tidy — it is required. Logitech
    /// Options+ diverts the side buttons so it can remap them in software, and
    /// those diverts live in the mouse until it power-cycles; quitting Options+
    /// does not undo them. While a button is diverted the mouse emits an HID++
    /// event instead of a normal mouse button, so the low-level hook never sees
    /// it and the button appears dead.
    pub(crate) fn apply_control_diverts(&self, gestures_enabled: bool, mappings_enabled: bool) {
        self.apply_control_diverts_with_map(
            gestures_enabled,
            mappings_enabled,
            &crate::state::active_map(),
        );
    }

    pub(crate) fn apply_control_diverts_with_map(
        &self,
        gestures_enabled: bool,
        mappings_enabled: bool,
        buttons: &crate::config::ButtonMap,
    ) {
        let mut diverted = 0usize;
        let mut restored = 0usize;
        for ci in &self.controls {
            if ci.virtual_control() || !ci.divertable() {
                continue;
            }
            let gesture = Some(ci.cid) == self.capabilities.gesture_control && gestures_enabled;
            let mapped = self
                .capabilities
                .buttons
                .iter()
                .any(|button| button.cid == ci.cid && button.slot.starts_with("control_"))
                && mappings_enabled
                && buttons.control_action(ci.cid).intercepts();
            let want = gesture || mapped;
            match self.set_divert(ci, want, gesture) {
                Ok(()) => {
                    if want {
                        diverted += 1;
                    } else {
                        restored += 1;
                    }
                }
                Err(e) => tracing::warn!("set divert={want} on CID {:#06x} failed: {e:#}", ci.cid),
            }
        }
        tracing::info!("control diverts applied: {diverted} diverted, {restored} native");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn control(cid: u16, raw_xy: bool) -> ControlInfo {
        ControlInfo {
            cid,
            tid: 0,
            flags: KEY_DIVERTABLE,
            additional_flags: if raw_xy { KEY_RAW_XY } else { 0 },
        }
    }
    #[test]
    fn vertical_has_dpi_button_without_a_gesture_pad() {
        let controls = [
            control(0x0052, false),
            control(0x0053, false),
            control(0x0056, false),
            control(0x00ED, true),
        ];
        assert_eq!(gesture_control(&controls), None);
        let choices = button_choices(&controls, None);
        assert_eq!(choices.len(), 4);
        assert_eq!(choices[3].slot, "control_00ed");
        assert_eq!(choices[3].name, "DPI button");
    }
    #[test]
    fn raw_xy_capabilities_choose_gestures_and_leave_extra_buttons_remappable() {
        let controls = [
            control(0x00C4, true),
            control(0x00C3, true),
            control(0x01A1, false),
        ];
        assert_eq!(gesture_control(&controls), Some(0x00C3));
        let choices = button_choices(&controls, Some(0x00C3));
        assert_eq!(choices.len(), 2);
        let haptic = [control(0x01A0, true)];
        assert_eq!(gesture_control(&haptic), Some(0x01A0));
    }
}
