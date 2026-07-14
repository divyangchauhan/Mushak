//! Feature 0x2110 SMART_SHIFT (original variant used by the MX Master 2S).
//!
//! function 0 getStatus  -> { wheelMode, autoDisengage, autoDisengageDefault }
//! function 1 setStatus  <- { wheelMode, autoDisengage, autoDisengageDefault }
//!
//! wheelMode: 1 = freespin, 2 = ratchet.
//! autoDisengage: 1..=30 threshold, or 255 = never auto-disengage.

use crate::config::SmartShiftMode;
use crate::hidpp::device::Device;
use anyhow::{anyhow, Result};

const WHEEL_FREESPIN: u8 = 1;
const WHEEL_RATCHET: u8 = 2;

impl Device {
    /// `getStatus` -> `(wheelMode, autoDisengage, autoDisengageDefault)`.
    pub(crate) fn get_smartshift(&self) -> Result<(u8, u8, u8)> {
        let idx = self
            .features
            .smartshift
            .ok_or_else(|| anyhow!("no SMART_SHIFT feature"))?;
        let rep = self.request(idx, 0, &[])?;
        Ok((rep.param(0), rep.param(1), rep.param(2)))
    }

    fn set_smartshift(&self, wheel_mode: u8, auto_disengage: u8, default: u8) -> Result<()> {
        let idx = self
            .features
            .smartshift
            .ok_or_else(|| anyhow!("no SMART_SHIFT feature"))?;
        self.request(idx, 1, &[wheel_mode, auto_disengage, default])?;
        Ok(())
    }

    /// Apply a [`SmartShiftMode`] + threshold from config.
    pub(crate) fn apply_smartshift(&self, mode: SmartShiftMode, threshold: u8) -> Result<()> {
        if self.features.smartshift.is_none() {
            return Ok(());
        }
        // Preserve the firmware's default auto-disengage value.
        let default = self.get_smartshift().map(|s| s.2).unwrap_or(0);

        let (wheel_mode, auto_disengage) = match mode {
            SmartShiftMode::AlwaysRatchet => (WHEEL_RATCHET, 255),
            SmartShiftMode::AlwaysFreespin => (WHEEL_FREESPIN, 1),
            SmartShiftMode::SmartShift => (WHEEL_RATCHET, threshold.clamp(1, 30)),
        };
        tracing::info!(
            "smartshift -> mode={mode:?} wheelMode={wheel_mode} autoDisengage={auto_disengage}"
        );
        self.set_smartshift(wheel_mode, auto_disengage, default)
    }
}
