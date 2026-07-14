//! Feature 0x2121 HIRES_WHEEL (smooth/high-resolution scrolling + invert).
//!
//! function 1 getMode -> mode byte
//! function 2 setMode <- mode byte
//!
//! mode bits: 0x01 target (divert to HID++), 0x02 high-resolution, 0x04 invert.

use crate::hidpp::device::Device;
use anyhow::{anyhow, Result};

const MODE_HIRES: u8 = 0x02;
const MODE_INVERT: u8 = 0x04;

impl Device {
    fn hires_index(&self) -> Result<u8> {
        self.features
            .hires_wheel
            .ok_or_else(|| anyhow!("no HIRES_WHEEL feature"))
    }

    pub(crate) fn get_wheel_mode(&self) -> Result<u8> {
        let idx = self.hires_index()?;
        let rep = self.request(idx, 1, &[])?;
        Ok(rep.param(0))
    }

    fn set_wheel_mode(&self, mode: u8) -> Result<()> {
        let idx = self.hires_index()?;
        self.request(idx, 2, &[mode])?;
        Ok(())
    }

    /// Apply high-resolution + invert flags, preserving the divert (target) bit.
    pub(crate) fn apply_hires(&self, hires: bool, invert: bool) -> Result<()> {
        if self.features.hires_wheel.is_none() {
            return Ok(());
        }
        let current = self.get_wheel_mode().unwrap_or(0);
        let mut mode = current & 0x01; // keep target bit as-is
        if hires {
            mode |= MODE_HIRES;
        }
        if invert {
            mode |= MODE_INVERT;
        }
        tracing::info!("hires wheel -> mode={mode:#04x} (hires={hires} invert={invert})");
        self.set_wheel_mode(mode)
    }
}
