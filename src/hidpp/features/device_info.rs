//! Feature 0x0003 DEVICE_INFO: firmware version string.

use crate::hidpp::device::Device;
use anyhow::{anyhow, Result};

impl Device {
    /// Best-effort main-application firmware version, e.g. `"MPM 12.02.B0009"`.
    pub(crate) fn firmware_string(&self) -> Result<String> {
        let di = self
            .features
            .device_info
            .ok_or_else(|| anyhow!("no DEVICE_INFO feature"))?;

        // getDeviceInfo (function 0) -> entityCount at param 0.
        let info = self.request(di, 0, &[])?;
        let entity_count = info.param(0);

        for i in 0..entity_count {
            // getFwInfo (function 1) for entity i.
            let fw = self.request(di, 1, &[i])?;
            let fw_type = fw.param(0) & 0x0F;
            // Type 0 == main application firmware.
            if fw_type == 0 {
                let name: String = (1..4)
                    .map(|k| fw.param(k))
                    .filter(|&c| c.is_ascii_graphic())
                    .map(|c| c as char)
                    .collect();
                let major = fw.param(4);
                let minor = fw.param(5);
                let build = ((fw.param(6) as u16) << 8) | fw.param(7) as u16;
                return Ok(format!("{name} {major:02x}.{minor:02x}.B{build:04x}"));
            }
        }
        Err(anyhow!("no main firmware entity found"))
    }
}
