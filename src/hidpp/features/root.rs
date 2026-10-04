//! Feature 0x0000 IRoot: feature discovery and ping.

use crate::hidpp::device::Device;
use crate::hidpp::protocol::ROOT_INDEX;
use anyhow::Result;
use std::time::Duration;

impl Device {
    /// Resolve a feature ID through `IRoot.getFeature`.
    /// `(featureIndex, featureType, featureVersion)`, or `None` if absent.
    pub(crate) fn get_feature(&self, feature_id: u16) -> Result<Option<(u8, u8, u8)>> {
        self.get_feature_on(self.index, feature_id)
    }

    pub(crate) fn get_feature_on(
        &self,
        device_index: u8,
        feature_id: u16,
    ) -> Result<Option<(u8, u8, u8)>> {
        let params = [(feature_id >> 8) as u8, (feature_id & 0xFF) as u8, 0];
        let rep = self.request_on(device_index, ROOT_INDEX, 0, &params)?;
        let idx = rep.param(0);
        if idx == 0 {
            Ok(None)
        } else {
            Ok(Some((idx, rep.param(1), rep.param(2))))
        }
    }

    /// `IRoot.getProtocolVersion` — ping a device index; returns
    /// `(major, minor)`. Fails quickly if the slot has no connected device
    /// (kept short so probing all 6 receiver slots stays snappy).
    pub(crate) fn ping(&self, device_index: u8) -> Result<(u8, u8)> {
        let rep = self.request_timeout(
            device_index,
            ROOT_INDEX,
            1,
            &[0, 0, 0x5A],
            Duration::from_millis(300),
        )?;
        Ok((rep.param(0), rep.param(1)))
    }
}
