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

/// CID of the MX Master 2S thumb "gesture" button.
pub const CID_GESTURE: u16 = 0x00C3;

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

impl ControlInfo {
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
    fn set_divert(&self, ci: &ControlInfo, divert: bool) -> Result<()> {
        if !ci.divertable() {
            return Ok(());
        }
        let mut flags = RC_CHANGE_TEMP_DIVERT;
        if divert {
            flags |= RC_TEMP_DIVERT;
        }
        if ci.raw_xy() {
            flags |= RC_CHANGE_RAWXY_DIVERT;
            if divert {
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

    /// Assert the divert state of *every* control: the thumb button diverted
    /// when gestures are enabled, everything else reporting natively.
    ///
    /// Restoring the others is not merely tidy — it is required. Logitech
    /// Options+ diverts the side buttons so it can remap them in software, and
    /// those diverts live in the mouse until it power-cycles; quitting Options+
    /// does not undo them. While a button is diverted the mouse emits an HID++
    /// event instead of a normal mouse button, so the low-level hook never sees
    /// it and the button appears dead.
    pub(crate) fn apply_control_diverts(&self, gestures_enabled: bool) {
        let mut diverted = 0usize;
        let mut restored = 0usize;
        for ci in &self.controls {
            if ci.virtual_control() || !ci.divertable() {
                continue;
            }
            let want = ci.cid == CID_GESTURE && gestures_enabled;
            match self.set_divert(ci, want) {
                Ok(()) => {
                    if want {
                        diverted += 1;
                    } else {
                        restored += 1;
                    }
                }
                Err(e) => tracing::warn!(
                    "set divert={want} on CID {:#06x} failed: {e:#}",
                    ci.cid
                ),
            }
        }
        tracing::info!("control diverts applied: {diverted} diverted, {restored} native");
    }
}
