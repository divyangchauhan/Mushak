//! HID++ 0x2201, with choices validated against the sensor's own DPI list.

use crate::hidpp::device::Device;
use anyhow::{anyhow, bail, Context, Result};

const SENSOR: u8 = 0;

impl Device {
    fn dpi_index(&self) -> Result<u8> {
        self.features
            .dpi
            .ok_or_else(|| anyhow!("no ADJUSTABLE_DPI feature"))
    }

    pub(crate) fn get_dpi_list(&self) -> Result<Vec<u16>> {
        let idx = self.dpi_index()?;
        if self.request(idx, 0, &[])?.param(0) == 0 {
            bail!("no DPI sensor");
        }
        let rep = self.request(idx, 1, &[SENSOR])?;
        if rep.param(0) != SENSOR {
            bail!("unexpected DPI sensor");
        }
        let data = rep.raw.get(5..).context("truncated DPI list")?;
        let raw: Vec<_> = data
            .chunks_exact(2)
            .map(|v| u16::from_be_bytes([v[0], v[1]]))
            .take_while(|v| *v != 0)
            .collect();
        decode_dpi_list(&raw)
    }

    pub(crate) fn get_dpi(&self) -> Result<(u16, u16)> {
        let idx = self.dpi_index()?;
        let rep = self.request(idx, 2, &[SENSOR])?;
        if rep.raw.len() < 9 || rep.param(0) != SENSOR {
            bail!("truncated DPI state");
        }
        let default = u16::from_be_bytes([rep.param(3), rep.param(4)]);
        let current = u16::from_be_bytes([rep.param(1), rep.param(2)]);
        Ok((if current == 0 { default } else { current }, default))
    }

    pub(crate) fn apply_dpi(&self, desired: u16) -> Result<()> {
        if !self.capabilities.dpi {
            return Ok(());
        }
        let dpi = nearest(&self.dpi_values, desired).context("no validated DPI choices")?;
        let idx = self.dpi_index()?;
        self.request(idx, 3, &[SENSOR, (dpi >> 8) as u8, dpi as u8])?;
        Ok(())
    }
}

pub(crate) fn decode_dpi_list(raw: &[u16]) -> Result<Vec<u16>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        let value = raw[i];
        if value >= 0xE000 {
            let step = u32::from(value - 0xE000);
            let min = u32::from(*out.last().context("DPI range missing minimum")?);
            let max = u32::from(*raw.get(i + 1).context("DPI range missing maximum")?);
            if step == 0 || max <= min || max >= 0xE000 {
                bail!("invalid DPI range");
            }
            let mut next = min + step;
            while next <= max {
                out.push(next as u16);
                next += step;
            }
            i += 2;
        } else {
            if value == 0 {
                bail!("invalid zero DPI");
            }
            out.push(value);
            i += 1;
        }
    }
    out.sort_unstable();
    out.dedup();
    if out.is_empty() {
        bail!("empty DPI list");
    }
    Ok(out)
}

pub(crate) fn nearest(list: &[u16], target: u16) -> Option<u16> {
    list.iter()
        .copied()
        .min_by_key(|&v| (i32::from(v) - i32::from(target)).unsigned_abs())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discrete_choices_and_ranges_are_reported_values_only() {
        assert_eq!(
            decode_dpi_list(&[1600, 400, 800]).unwrap(),
            [400, 800, 1600]
        );
        assert_eq!(
            decode_dpi_list(&[400, 0xE064, 800]).unwrap(),
            [400, 500, 600, 700, 800]
        );
        assert_eq!(nearest(&[400, 800, 1600], 1000), Some(800));
        assert_eq!(nearest(&[], 1000), None);
        assert!(decode_dpi_list(&[]).is_err());
        assert!(decode_dpi_list(&[0xE064, 800]).is_err());
        assert!(decode_dpi_list(&[400, 0xE064]).is_err());
        assert!(decode_dpi_list(&[400, 0xE000, 800]).is_err());
        assert!(decode_dpi_list(&[800, 0xE064, 400]).is_err());
    }
}
