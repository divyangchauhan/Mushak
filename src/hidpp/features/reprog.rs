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

const RC_TEMP_DIVERT: u8 = 0x01;
const RC_CHANGE_TEMP_DIVERT: u8 = 0x02;
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

    /// Enumerate + log all controls; return true if the gesture CID is present.
    pub(crate) fn enumerate_controls(&self) -> bool {
        let count = match self.control_count() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("getControlCount failed: {e:#}");
                return false;
            }
        };
        tracing::info!("REPROG_CONTROLS_V4: {count} controls");
        let mut has_gesture = false;
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
                    if ci.cid == CID_GESTURE {
                        has_gesture = true;
                    }
                }
                Err(e) => tracing::debug!("  control[{i:02}] read failed: {e:#}"),
            }
        }
        has_gesture
    }

    fn set_control_reporting(&self, cid: u16, flags: u8) -> Result<()> {
        let idx = self.reprog_index()?;
        // params: cid(be16), flags, remap(be16 = 0)
        self.request(idx, 3, &[(cid >> 8) as u8, (cid & 0xFF) as u8, flags, 0, 0])?;
        Ok(())
    }

    /// Divert or restore the gesture button (button + raw XY).
    pub(crate) fn divert_gesture(&self, enable: bool) -> Result<()> {
        let flags = if enable {
            RC_TEMP_DIVERT | RC_CHANGE_TEMP_DIVERT | RC_RAWXY_DIVERT | RC_CHANGE_RAWXY_DIVERT
        } else {
            // Clear both divert bits (set change bits, leave value bits 0).
            RC_CHANGE_TEMP_DIVERT | RC_CHANGE_RAWXY_DIVERT
        };
        tracing::info!(
            "{} gesture CID {:#06x} (flags={flags:#04x})",
            if enable { "diverting" } else { "restoring" },
            CID_GESTURE
        );
        self.set_control_reporting(CID_GESTURE, flags)
    }
}
