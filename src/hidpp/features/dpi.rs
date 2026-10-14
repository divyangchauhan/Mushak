//! HID++ 0x2201, with choices validated against the sensor's own DPI list.

use crate::hidpp::device::Device;
use anyhow::{anyhow, bail, Context, Result};

impl Device {
    fn dpi_feature(&self) -> Result<(u8, bool)> {
        if let Some(index) = self.features.extended_dpi {
            Ok((index, true))
        } else if let Some(index) = self.features.dpi {
            Ok((index, false))
        } else {
            Err(anyhow!("no adjustable DPI feature"))
        }
    }

    pub(crate) fn find_dpi_sensor(&self) -> Result<u8> {
        let (index, extended) = self.dpi_feature()?;
        let count = self.request(index, 0, &[])?.param(0);
        for sensor in 0..count {
            if self.dpi_list_on(index, extended, sensor).is_ok() {
                return Ok(sensor);
            }
        }
        bail!("no readable DPI sensor")
    }

    pub(crate) fn get_dpi_list(&self) -> Result<Vec<u16>> {
        let (index, extended) = self.dpi_feature()?;
        let sensor = self.dpi_sensor.context("no DPI sensor")?;
        self.dpi_list_on(index, extended, sensor)
    }

    fn dpi_list_on(&self, index: u8, extended: bool, sensor: u8) -> Result<Vec<u16>> {
        let mut values = Vec::new();
        let mut previous = Vec::new();
        for page in 0..=u8::MAX {
            let reply = self.request(index, if extended { 2 } else { 1 }, &[sensor, 0, page])?;
            if reply.param(0) != sensor {
                bail!("unexpected DPI sensor");
            }
            let header = if extended { 3 } else { 1 };
            let data = reply.raw.get(4 + header..).context("truncated DPI list")?;
            let items: Vec<_> = data
                .chunks_exact(2)
                .map(|v| u16::from_be_bytes([v[0], v[1]]))
                .collect();
            if items.is_empty() || (page > 0 && items == previous) {
                bail!("incomplete DPI list");
            }
            for value in &items {
                if *value == 0 {
                    return decode_dpi_list(&values);
                }
                values.push(*value);
            }
            previous = items;
        }
        bail!("DPI list exceeds protocol page limit")
    }

    pub(crate) fn get_dpi(&self) -> Result<(u16, u16)> {
        let (idx, extended) = self.dpi_feature()?;
        let sensor = self.dpi_sensor.context("no DPI sensor")?;
        let rep = self.request(idx, if extended { 5 } else { 2 }, &[sensor])?;
        if rep.raw.len() < 9 || rep.param(0) != sensor {
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
        let (idx, extended) = self.dpi_feature()?;
        let sensor = self.dpi_sensor.context("no DPI sensor")?;
        let mut params = vec![sensor, (dpi >> 8) as u8, dpi as u8];
        if extended {
            // Keep independent Y resolution and lift-off distance unchanged.
            let caps = self.request(idx, 1, &[sensor])?;
            let current = self.request(idx, 5, &[sensor])?;
            if caps.raw.len() < 7 || current.raw.len() < 14 {
                bail!("truncated extended DPI state");
            }
            let y = if caps.param(2) & 1 != 0 {
                let reported = u16::from_be_bytes([current.param(5), current.param(6)]);
                if reported == 0 {
                    u16::from_be_bytes([current.param(7), current.param(8)])
                } else {
                    reported
                }
            } else {
                0
            };
            params.extend_from_slice(&[
                (y >> 8) as u8,
                y as u8,
                if caps.param(2) & 2 != 0 {
                    current.param(9)
                } else {
                    0
                },
            ]);
        }
        self.request(idx, if extended { 6 } else { 3 }, &params)?;
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
