//! Raw HID transport: locate and open the MX Master 2S HID++ interface (via a
//! Unifying receiver or directly over Bluetooth) and move report bytes.

use super::protocol::*;
use crate::logging::hex;
use anyhow::{anyhow, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::ffi::{CStr, CString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    Receiver,
    Bluetooth,
}

impl ConnectionKind {
    pub fn label(&self, device_index: u8) -> String {
        match self {
            ConnectionKind::Receiver => format!("Unifying receiver (device {device_index})"),
            ConnectionKind::Bluetooth => "Bluetooth".to_string(),
        }
    }
}

/// A HID++ interface worth trying to talk to.
#[derive(Clone)]
pub struct Candidate {
    pub path: CString,
    pub kind: ConnectionKind,
}

pub struct HidppTransport {
    device: HidDevice,
    pub kind: ConnectionKind,
}

impl HidppTransport {
    /// Enumerate Logitech HID++ control interfaces, ordered receiver-first then
    /// Bluetooth. The caller tries each until one yields a live device (a
    /// receiver may be plugged in with no paired mouse while the 2S is actually
    /// on Bluetooth).
    pub fn candidates() -> Result<Vec<Candidate>> {
        let api = HidApi::new().context("initializing hidapi")?;

        let mut receiver: Option<(CString, u16)> = None;
        let mut bluetooth: Option<(CString, u16)> = None;

        for info in api.device_list() {
            if info.vendor_id() != VENDOR_LOGITECH {
                continue;
            }
            tracing::debug!(
                "HID candidate pid={:04x} usage_page={:04x} usage={:04x} iface={} product={:?}",
                info.product_id(),
                info.usage_page(),
                info.usage(),
                info.interface_number(),
                info.product_string()
            );

            let pid = info.product_id();
            let path = info.path().to_owned();

            if pid == PID_UNIFYING_RECEIVER && info.usage_page() == USAGE_PAGE_HIDPP {
                // Prefer the long-report (usage 0x0002) collection.
                let prefer = info.usage() == USAGE_HIDPP_LONG;
                if receiver.is_none() || prefer {
                    receiver = Some((path, info.usage()));
                }
            } else if pid == PID_MX_MASTER_2S_BT && info.usage_page() >= 0xFF00 {
                // Bluetooth HID++ lives on a vendor-defined usage page (e.g.
                // 0xFF43); the standard mouse collection (0x0001) is skipped.
                // The long collection may use usage 0x0002 or 0x0202.
                let prefer = matches!(info.usage(), USAGE_HIDPP_LONG | 0x0202);
                if bluetooth.is_none() || prefer {
                    bluetooth = Some((path, info.usage()));
                }
            }
        }

        let mut out = Vec::new();
        if let Some((path, _)) = receiver {
            out.push(Candidate {
                path,
                kind: ConnectionKind::Receiver,
            });
        }
        if let Some((path, _)) = bluetooth {
            out.push(Candidate {
                path,
                kind: ConnectionKind::Bluetooth,
            });
        }
        if out.is_empty() {
            return Err(anyhow!(
                "no MX Master 2S HID++ interface found (receiver {PID_UNIFYING_RECEIVER:04x} or BT {PID_MX_MASTER_2S_BT:04x})"
            ));
        }
        Ok(out)
    }

    pub fn open_candidate(c: &Candidate) -> Result<HidppTransport> {
        let api = HidApi::new().context("initializing hidapi")?;
        Self::open_path(&api, &c.path, c.kind)
    }

    fn open_path(api: &HidApi, path: &CStr, kind: ConnectionKind) -> Result<HidppTransport> {
        let device = api
            .open_path(path)
            .with_context(|| format!("opening HID path {path:?}"))?;
        tracing::info!("opened HID++ interface ({kind:?}) at {path:?}");
        Ok(HidppTransport { device, kind })
    }

    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        tracing::debug!("HID++ >>> {}", hex(bytes));
        let n = self.device.write(bytes).context("HID write")?;
        if n != bytes.len() {
            tracing::warn!("short HID write: {n}/{}", bytes.len());
        }
        Ok(())
    }

    /// Read one report with a timeout. Returns `None` on timeout, `Some(report)`
    /// otherwise. Errors indicate the device went away.
    pub fn read(&self, timeout_ms: i32) -> Result<Option<Report>> {
        let mut buf = [0u8; LONG_LEN];
        let n = self
            .device
            .read_timeout(&mut buf, timeout_ms)
            .context("HID read")?;
        if n == 0 {
            return Ok(None);
        }
        let report = Report::new(&buf[..n]);
        tracing::debug!("HID++ <<< {}", hex(&report.raw));
        Ok(Some(report))
    }
}
