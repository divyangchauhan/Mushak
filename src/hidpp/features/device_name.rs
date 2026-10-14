//! HID++ 0x0005 reports the peripheral name and kind, including receiver slots.

use crate::hidpp::{device::Device, protocol::FEAT_DEVICE_NAME};
use anyhow::{bail, Context, Result};

impl Device {
    pub(crate) fn device_kind_on(&self, device_index: u8) -> Result<Option<u8>> {
        let Some((idx, _, _)) = self.get_feature_on(device_index, FEAT_DEVICE_NAME)? else {
            return Ok(None);
        };
        Ok(Some(self.request_on(device_index, idx, 2, &[])?.param(0)))
    }
    pub(crate) fn device_name_on(&self, device_index: u8) -> Result<String> {
        let (idx, _, _) = self
            .get_feature_on(device_index, FEAT_DEVICE_NAME)?
            .context("no DEVICE_NAME feature")?;
        let length = self.request_on(device_index, idx, 0, &[])?.param(0) as usize;
        if length == 0 {
            bail!("device name is empty");
        }
        let mut bytes = Vec::with_capacity(length);
        while bytes.len() < length {
            let reply = self.request_on(device_index, idx, 1, &[bytes.len() as u8])?;
            let fragment = reply.raw.get(4..).context("truncated device name reply")?;
            if fragment.is_empty() {
                bail!("empty device name fragment");
            }
            bytes.extend_from_slice(&fragment[..fragment.len().min(length - bytes.len())]);
        }
        decode_name(&bytes)
    }
}

fn decode_name(bytes: &[u8]) -> Result<String> {
    let name = std::str::from_utf8(bytes)
        .context("device name is not UTF-8")?
        .trim_matches('\0')
        .trim();
    if name.is_empty() {
        bail!("device name is empty");
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_reported_model_and_rejects_bad_names() {
        assert_eq!(
            decode_name(b"MX Master 4 for Mac\0").unwrap(),
            "MX Master 4 for Mac"
        );
        assert!(decode_name(b"\0").is_err());
        assert!(decode_name(&[0xFF]).is_err());
    }
}
