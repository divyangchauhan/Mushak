//! Feature 0x2201 ADJUSTABLE_DPI.
//!
//! function 0 getSensorCount
//! function 1 getSensorDpiList(sensor) -> big-endian u16 list; a value > 0xE000
//!            encodes a step of `(value - 0xE000)` between neighbours (range).
//! function 2 getSensorDpi(sensor)     -> current + default (big-endian u16)
//! function 3 setSensorDpi(sensor, dpi)

use crate::hidpp::device::Device;
use anyhow::{anyhow, Result};

const SENSOR: u8 = 0;

impl Device {
    fn dpi_index(&self) -> Result<u8> {
        self.features
            .dpi
            .ok_or_else(|| anyhow!("no ADJUSTABLE_DPI feature"))
    }

    /// Expand the device's DPI list (including range encoding) into discrete,
    /// ascending values.
    pub(crate) fn get_dpi_list(&self) -> Result<Vec<u16>> {
        let idx = self.dpi_index()?;
        let rep = self.request(idx, 1, &[SENSOR])?;

        // Payload: param(0) echoes the sensor index; values start at param(1).
        let mut raw: Vec<u16> = Vec::new();
        let mut i = 1;
        while i + 1 <= 15 {
            let v = ((rep.param(i) as u16) << 8) | rep.param(i + 1) as u16;
            if v == 0 {
                break;
            }
            raw.push(v);
            i += 2;
        }

        // Decode range markers (0xE000 | step) into discrete values.
        let mut out: Vec<u16> = Vec::new();
        let mut j = 0;
        while j < raw.len() {
            let v = raw[j];
            if v > 0xE000 {
                // step between the previous value (min) and the next (max)
                let step = (v - 0xE000).max(1);
                let min = *out.last().unwrap_or(&200);
                let max = raw.get(j + 1).copied().unwrap_or(min);
                let mut d = min + step;
                while d <= max {
                    out.push(d);
                    d += step;
                }
                j += 2; // consume step + max
            } else {
                out.push(v);
                j += 1;
            }
        }
        out.sort_unstable();
        out.dedup();
        tracing::info!("DPI list: {:?}", out);
        Ok(out)
    }

    /// `getSensorDpi` -> `(current, default)`.
    pub(crate) fn get_dpi(&self) -> Result<(u16, u16)> {
        let idx = self.dpi_index()?;
        let rep = self.request(idx, 2, &[SENSOR])?;
        let current = ((rep.param(1) as u16) << 8) | rep.param(2) as u16;
        let default = ((rep.param(3) as u16) << 8) | rep.param(4) as u16;
        Ok((current, default))
    }

    fn set_dpi(&self, dpi: u16) -> Result<()> {
        let idx = self.dpi_index()?;
        self.request(idx, 3, &[SENSOR, (dpi >> 8) as u8, (dpi & 0xFF) as u8])?;
        Ok(())
    }

    /// Set DPI, snapping to the nearest device-supported value.
    pub(crate) fn apply_dpi(&self, desired: u16) -> Result<()> {
        if self.features.dpi.is_none() {
            return Ok(());
        }
        let list = self.get_dpi_list().unwrap_or_default();
        let dpi = nearest(&list, desired);
        tracing::info!("dpi -> {dpi} (requested {desired})");
        self.set_dpi(dpi)
    }
}

/// Nearest value in `list` to `target`, or `target` if the list is empty.
fn nearest(list: &[u16], target: u16) -> u16 {
    list.iter()
        .copied()
        .min_by_key(|&v| (v as i32 - target as i32).unsigned_abs())
        .unwrap_or(target)
}
