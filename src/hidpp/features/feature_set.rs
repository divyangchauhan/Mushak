//! Feature 0x0001 IFeatureSet: enumerate the full feature table (great for
//! logging against unknown firmware).

use crate::hidpp::device::Device;
use crate::hidpp::protocol::*;
use anyhow::Result;

impl Device {
    /// Build `features.all` — the authoritative (id, index) list via IFeatureSet
    /// when available, else the subset resolved through IRoot.
    pub(crate) fn enumerate_features(&mut self) {
        let mut all: Vec<(u16, u8)> = Vec::new();

        if let Some(fs) = self.features.feature_set {
            match self.feature_count(fs) {
                Ok(count) => {
                    tracing::info!("device exposes {count} features (+ IRoot)");
                    for i in 0..=count {
                        match self.feature_id_at(fs, i) {
                            Ok((id, ftype)) => {
                                tracing::info!(
                                    "  feature[{i:02}] id={id:#06x} type={ftype:#04x}"
                                );
                                all.push((id, i));
                            }
                            Err(e) => tracing::debug!("  feature[{i:02}] read failed: {e:#}"),
                        }
                    }
                }
                Err(e) => tracing::warn!("IFeatureSet.getCount failed: {e:#}"),
            }
        }

        if all.is_empty() {
            all = self.resolved_pairs();
        }
        self.features.all = all;
    }

    fn feature_count(&self, fs_index: u8) -> Result<u8> {
        // getCount (function 0)
        let rep = self.request(fs_index, 0, &[])?;
        Ok(rep.param(0))
    }

    fn feature_id_at(&self, fs_index: u8, feature_index: u8) -> Result<(u16, u8)> {
        // getFeatureId (function 1)
        let rep = self.request(fs_index, 1, &[feature_index])?;
        let id = ((rep.param(0) as u16) << 8) | rep.param(1) as u16;
        Ok((id, rep.param(2)))
    }

    fn resolved_pairs(&self) -> Vec<(u16, u8)> {
        let f = &self.features;
        let mut v = Vec::new();
        for (id, idx) in [
            (FEAT_FEATURE_SET, f.feature_set),
            (FEAT_DEVICE_INFO, f.device_info),
            (FEAT_BATTERY_STATUS, f.battery),
            (FEAT_REPROG_CONTROLS_V4, f.reprog),
            (FEAT_SMART_SHIFT, f.smartshift),
            (FEAT_HIRES_WHEEL, f.hires_wheel),
            (FEAT_ADJUSTABLE_DPI, f.dpi),
        ] {
            if let Some(i) = idx {
                v.push((id, i));
            }
        }
        v
    }
}
