//! Battery status, voltage, and unified battery use different wire layouts.

use crate::hidpp::{device::Device, protocol::*};
use crate::state;
use anyhow::{bail, Result};

#[derive(Default, Debug)]
pub struct BatteryReading {
    pub percent: Option<u8>,
    pub voltage_mv: Option<u16>,
    pub level: Option<String>,
    pub charging: bool,
}

impl Device {
    pub(crate) fn read_battery(&self) -> Result<BatteryReading> {
        for (feature, index, function) in [
            (FEAT_UNIFIED_BATTERY, self.features.unified_battery, 1),
            (FEAT_BATTERY_STATUS, self.features.battery, 0),
            (FEAT_BATTERY_VOLTAGE, self.features.battery_voltage, 0),
        ] {
            if let Some(index) = index {
                if let Ok(reply) = self.request(index, function, &[]) {
                    if let Ok(reading) = decode_battery(feature, reply.raw.get(4..).unwrap_or(&[]))
                    {
                        return Ok(reading);
                    }
                }
            }
        }
        bail!("no readable battery feature")
    }

    pub(crate) fn on_battery_event(&self, report: &Report) -> bool {
        if report.function() != 0 {
            return false;
        }
        for (feature, index) in [
            (FEAT_UNIFIED_BATTERY, self.features.unified_battery),
            (FEAT_BATTERY_STATUS, self.features.battery),
            (FEAT_BATTERY_VOLTAGE, self.features.battery_voltage),
        ] {
            if index == Some(report.feature_index()) {
                if let Ok(battery) = decode_battery(feature, report.raw.get(4..).unwrap_or(&[])) {
                    state::update_device_status(|s| {
                        s.battery_percent = battery.percent;
                        s.battery_voltage_mv = battery.voltage_mv;
                        s.battery_level = battery.level;
                        s.charging = battery.charging;
                    });
                }
                return true;
            }
        }
        false
    }
}

fn decode_battery(feature: u16, data: &[u8]) -> Result<BatteryReading> {
    if data.len() < 3 {
        bail!("truncated battery report");
    }
    let mut reading = BatteryReading::default();
    match feature {
        FEAT_BATTERY_STATUS | FEAT_UNIFIED_BATTERY => {
            if data[0] > 100 {
                bail!("invalid battery percentage");
            }
            reading.percent = (data[0] > 0).then_some(data[0]);
            reading.charging = matches!(data[2], 1 | 2);
            if feature == FEAT_UNIFIED_BATTERY && reading.percent.is_none() {
                reading.level = Some(
                    match data[1] {
                        8 => "Full",
                        4 => "Good",
                        2 => "Low",
                        1 => "Critical",
                        0 => "Empty",
                        _ => {
                            bail!("invalid unified battery level");
                        }
                    }
                    .to_owned(),
                );
            }
        }
        FEAT_BATTERY_VOLTAGE => {
            let voltage = u16::from_be_bytes([data[0], data[1]]);
            if voltage == 0 {
                bail!("invalid battery voltage");
            }
            reading.voltage_mv = Some(voltage);
            reading.charging = data[2] & 0x80 != 0;
        }
        _ => bail!("unsupported battery feature"),
    }
    Ok(reading)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_each_battery_layout_without_inventing_a_percentage() {
        let battery = decode_battery(FEAT_BATTERY_STATUS, &[73, 50, 1]).unwrap();
        assert_eq!(battery.percent, Some(73));
        assert!(battery.charging);
        assert!(
            !decode_battery(FEAT_UNIFIED_BATTERY, &[100, 8, 3, 0])
                .unwrap()
                .charging
        );
        let coarse = decode_battery(FEAT_UNIFIED_BATTERY, &[0, 2, 0, 0]).unwrap();
        assert_eq!(coarse.percent, None);
        assert_eq!(coarse.level.as_deref(), Some("Low"));
        let voltage = decode_battery(FEAT_BATTERY_VOLTAGE, &[0x0F, 0xA0, 0x80]).unwrap();
        assert_eq!(voltage.voltage_mv, Some(4000));
        assert_eq!(voltage.percent, None);
        assert!(decode_battery(FEAT_BATTERY_STATUS, &[101, 0, 0]).is_err());
        assert!(decode_battery(FEAT_UNIFIED_BATTERY, &[80]).is_err());
    }
}
