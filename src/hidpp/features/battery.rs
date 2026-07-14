//! Feature 0x1000 BATTERY_STATUS (the 2S predates 0x1004 UNIFIED_BATTERY).

use crate::hidpp::device::Device;
use anyhow::{anyhow, Result};

impl Device {
    /// `getBatteryLevelStatus` (function 0) -> `(percent, charging)`.
    pub(crate) fn read_battery(&self) -> Result<(u8, bool)> {
        let b = self
            .features
            .battery
            .ok_or_else(|| anyhow!("no BATTERY_STATUS feature"))?;
        let rep = self.request(b, 0, &[])?;
        Ok(interpret(rep.param(0), rep.param(2)))
    }
}

/// Map a `getBatteryLevelStatus` reply / broadcast into `(percent, charging)`.
///
/// `status`: 0 discharging, 1 recharging, 2 almost-full (charging),
/// 3 charge-complete, 4 charging-error, 5 invalid battery, 6 thermal-error.
pub fn interpret(level: u8, status: u8) -> (u8, bool) {
    let charging = matches!(status, 1 | 2 | 3);
    (level, charging)
}
