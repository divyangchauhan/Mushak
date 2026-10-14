//! Original SmartShift 0x2110 and enhanced SmartShift 0x2111.
//!
//! function 0 getStatus  -> { wheelMode, autoDisengage, autoDisengageDefault }
//! function 1 setStatus  <- { wheelMode, autoDisengage, autoDisengageDefault }
//!
//! wheelMode: 1 = freespin, 2 = ratchet.
//! autoDisengage: 1..=50 threshold, or 255 = never auto-disengage.

use crate::config::SmartShiftMode;
use crate::hidpp::device::Device;
use anyhow::{anyhow, Result};

const WHEEL_FREESPIN: u8 = 1;
const WHEEL_RATCHET: u8 = 2;

impl Device {
    fn smartshift_feature(&self) -> Result<(u8, bool)> {
        if let Some(index) = self.features.smartshift_enhanced {
            Ok((index, true))
        } else if let Some(index) = self.features.smartshift {
            Ok((index, false))
        } else {
            Err(anyhow!("no SmartShift feature"))
        }
    }
    /// `getStatus` -> `(wheelMode, autoDisengage, autoDisengageDefault)`.
    pub(crate) fn get_smartshift(&self) -> Result<(u8, u8, u8)> {
        let (idx, enhanced) = self.smartshift_feature()?;
        let rep = self.request(idx, if enhanced { 1 } else { 0 }, &[])?;
        if rep.raw.len() < 7 || !matches!(rep.param(0), WHEEL_FREESPIN | WHEEL_RATCHET) {
            return Err(anyhow!("invalid SmartShift state"));
        }
        Ok((rep.param(0), rep.param(1), rep.param(2)))
    }

    fn set_smartshift(&self, wheel_mode: u8, auto_disengage: u8, default: u8) -> Result<()> {
        let (idx, enhanced) = self.smartshift_feature()?;
        self.request(
            idx,
            if enhanced { 2 } else { 1 },
            &[wheel_mode, auto_disengage, default],
        )?;
        Ok(())
    }

    /// Apply a [`SmartShiftMode`] + threshold from config.
    pub(crate) fn apply_smartshift(&self, mode: SmartShiftMode, threshold: u8) -> Result<()> {
        if !self.capabilities.smartshift {
            return Ok(());
        }
        // The third byte is the original default threshold or enhanced torque.
        // Preserve it and refuse to write when the read failed.
        let default = self.get_smartshift()?.2;

        let (wheel_mode, auto_disengage) = match mode {
            SmartShiftMode::AlwaysRatchet => (WHEEL_RATCHET, 255),
            SmartShiftMode::AlwaysFreespin => (WHEEL_FREESPIN, 1),
            SmartShiftMode::SmartShift => (
                WHEEL_RATCHET,
                threshold.clamp(1, self.capabilities.smartshift_max_threshold.max(1)),
            ),
        };
        tracing::info!(
            "smartshift -> mode={mode:?} wheelMode={wheel_mode} autoDisengage={auto_disengage}"
        );
        self.set_smartshift(wheel_mode, auto_disengage, default)
    }
}
