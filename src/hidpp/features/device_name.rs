//! Feature 0x0005 DEVICE_NAME.

use crate::hidpp::device::Device;
use crate::hidpp::protocol::FEAT_DEVICE_NAME;
use anyhow::{anyhow, Result};

impl Device {
    pub(crate) fn device_name(&self) -> Result<String> {
        self.device_name_on(self.index)
    }

    pub(crate) fn device_name_on(&self, device_index: u8) -> Result<String> {
        let index = self
            .get_feature_on(device_index, FEAT_DEVICE_NAME)?
            .map(|feature| feature.0)
            .ok_or_else(|| anyhow!("no DEVICE_NAME feature"))?;

        let length = self.request_on(device_index, index, 0, &[])?.param(0) as usize;
        if length == 0 {
            return Err(anyhow!("device returned an empty name"));
        }

        let mut bytes = Vec::with_capacity(length);
        for offset in (0..length).step_by(16) {
            let reply = self.request_on(device_index, index, 1, &[offset as u8])?;
            for position in 0..16 {
                bytes.push(reply.param(position));
            }
        }
        bytes.truncate(length);
        String::from_utf8(bytes).map_err(|error| anyhow!("device name is not UTF-8: {error}"))
    }
}
