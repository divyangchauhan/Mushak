//! Discover HID++ interfaces by protocol collection and bus, not mouse model.

use super::protocol::*;
use crate::logging::hex;
use anyhow::{anyhow, Context, Result};
use hidapi::{BusType, HidApi, HidDevice};
use std::collections::HashSet;
use std::ffi::CString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConnectionKind {
    UnifyingReceiver,
    BoltReceiver,
    OtherReceiver,
    Bluetooth,
    Usb,
    Other,
}

impl ConnectionKind {
    pub fn is_receiver(self) -> bool {
        matches!(
            self,
            Self::UnifyingReceiver | Self::BoltReceiver | Self::OtherReceiver
        )
    }

    pub fn label(self, device_index: u8) -> String {
        let label = match self {
            Self::UnifyingReceiver => "Unifying receiver",
            Self::BoltReceiver => "Bolt receiver",
            Self::OtherReceiver => "USB receiver",
            Self::Bluetooth => "Bluetooth",
            Self::Usb => "Wired USB",
            Self::Other => "HID connection",
        };
        if self.is_receiver() && device_index != 0xFF {
            format!("{label}, device {device_index}")
        } else {
            label.to_owned()
        }
    }
}

#[derive(Clone)]
pub struct Candidate {
    pub path: CString,
    pub kind: ConnectionKind,
    pub product: Option<String>,
    pub product_id: u16,
    usage: u16,
}

pub struct HidppTransport {
    device: TransportIo,
    pub kind: ConnectionKind,
    pub product: Option<String>,
}

enum TransportIo {
    Hid(HidDevice),
    #[cfg(test)]
    Test(std::sync::Mutex<TestIo>),
}

#[cfg(test)]
type TestResponder = dyn FnMut(&[u8]) -> Result<Report> + Send;

#[cfg(test)]
struct TestIo {
    respond: Box<TestResponder>,
    pending: Option<Report>,
}

fn classify(bus: BusType, pid: u16, product: &str) -> ConnectionKind {
    if matches!(bus, BusType::Bluetooth) {
        return ConnectionKind::Bluetooth;
    }
    let name = product.to_ascii_lowercase();
    if pid == 0xC548 || name.contains("bolt receiver") {
        ConnectionKind::BoltReceiver
    } else if matches!(pid, 0xC52B | 0xC532) || name.contains("unifying") {
        ConnectionKind::UnifyingReceiver
    } else if (0xC500..=0xC5FF).contains(&pid) || name.contains("receiver") {
        ConnectionKind::OtherReceiver
    } else if matches!(bus, BusType::Usb) {
        ConnectionKind::Usb
    } else {
        ConnectionKind::Other
    }
}

fn eligible(vendor: u16, usage_page: u16) -> bool {
    vendor == VENDOR_LOGITECH && usage_page >= USAGE_PAGE_HIDPP
}

fn ordered_candidates(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
    candidates.sort_by_key(|c| {
        (
            if c.kind.is_receiver() { 0 } else { 1 },
            !matches!(c.usage, USAGE_HIDPP_LONG | 0x0202),
        )
    });
    let mut paths = HashSet::new();
    candidates.retain(|c| paths.insert(c.path.clone()));
    candidates
}

fn standard_mouse_status(
    vendor: u16,
    pid: u16,
    bus: BusType,
    product: Option<&str>,
) -> super::device::DeviceStatus {
    let kind = if vendor == VENDOR_LOGITECH {
        classify(bus, pid, product.unwrap_or(""))
    } else {
        match bus {
            BusType::Bluetooth => ConnectionKind::Bluetooth,
            BusType::Usb
                if product
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains("receiver") =>
            {
                ConnectionKind::OtherReceiver
            }
            BusType::Usb => ConnectionKind::Usb,
            _ => ConnectionKind::Other,
        }
    };
    let name = product.map(|name| {
        if kind.is_receiver() {
            format!("Mouse via {name}")
        } else {
            name.to_owned()
        }
    });
    super::device::DeviceStatus {
        connected: true,
        name,
        connection_kind: Some(kind),
        connection: Some(kind.label(0xFF)),
        settings_unavailable_reason: Some(
            "Advanced device settings are unavailable for this connection.".to_owned(),
        ),
        ..Default::default()
    }
}

impl HidppTransport {
    /// Standard HID mouse identity remains available without vendor controls.
    pub fn other_mouse_status() -> Result<Option<super::device::DeviceStatus>> {
        let api = HidApi::new().context("initializing hidapi")?;
        for info in api.device_list() {
            if info.usage_page() != 1 || info.usage() != 2 {
                continue;
            }
            let product = info.product_string().filter(|name| !name.trim().is_empty());
            return Ok(Some(standard_mouse_status(
                info.vendor_id(),
                info.product_id(),
                info.bus_type(),
                product,
            )));
        }
        Ok(None)
    }
    #[cfg(test)]
    pub(crate) fn test(
        kind: ConnectionKind,
        respond: impl FnMut(&[u8]) -> Result<Report> + Send + 'static,
    ) -> Self {
        Self {
            device: TransportIo::Test(std::sync::Mutex::new(TestIo {
                respond: Box::new(respond),
                pending: None,
            })),
            kind,
            product: None,
        }
    }
    pub fn candidates() -> Result<Vec<Candidate>> {
        let api = HidApi::new().context("initializing hidapi")?;
        let mut candidates = Vec::new();
        for info in api.device_list() {
            if !eligible(info.vendor_id(), info.usage_page()) {
                continue;
            }
            let product = info.product_string().map(str::to_owned);
            let kind = classify(
                info.bus_type(),
                info.product_id(),
                product.as_deref().unwrap_or(""),
            );
            tracing::debug!(
                "HID candidate pid={:04x} page={:04x} usage={:04x} bus={:?} product={product:?}",
                info.product_id(),
                info.usage_page(),
                info.usage(),
                info.bus_type()
            );
            candidates.push(Candidate {
                path: info.path().to_owned(),
                kind,
                product,
                product_id: info.product_id(),
                usage: info.usage(),
            });
        }
        let candidates = ordered_candidates(candidates);
        if candidates.is_empty() {
            return Err(anyhow!("no Logitech HID++ interface found"));
        }
        Ok(candidates)
    }

    pub fn open_candidate(c: &Candidate) -> Result<Self> {
        let api = HidApi::new().context("initializing hidapi")?;
        let device = api
            .open_path(&c.path)
            .with_context(|| format!("opening HID path {:?}", c.path))?;
        tracing::info!(
            "opened {:?} HID++ interface pid={:04x}",
            c.kind,
            c.product_id
        );
        Ok(Self {
            device: TransportIo::Hid(device),
            kind: c.kind,
            product: c.product.clone(),
        })
    }

    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        tracing::debug!("HID++ >>> {}", hex(bytes));
        let n = match &self.device {
            TransportIo::Hid(device) => device.write(bytes).context("HID write")?,
            #[cfg(test)]
            TransportIo::Test(io) => {
                let mut io = io.lock().unwrap();
                io.pending = Some((io.respond)(bytes)?);
                bytes.len()
            }
        };
        if n != bytes.len() {
            return Err(anyhow!("short HID write: {n}/{}", bytes.len()));
        }
        Ok(())
    }

    pub fn read(&self, timeout_ms: i32) -> Result<Option<Report>> {
        let mut buf = [0u8; LONG_LEN];
        let n = match &self.device {
            TransportIo::Hid(device) => device
                .read_timeout(&mut buf, timeout_ms)
                .context("HID read")?,
            #[cfg(test)]
            TransportIo::Test(io) => return Ok(io.lock().unwrap().pending.take()),
        };
        if n == 0 {
            return Ok(None);
        }
        let report = Report::new(&buf[..n]);
        tracing::debug!("HID++ <<< {}", hex(&report.raw));
        Ok(Some(report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_mice_keep_identity_without_unverified_vendor_settings() {
        let mouse = standard_mouse_status(0x1234, 0xC548, BusType::Usb, Some("USB mouse"));
        assert_eq!(mouse.display_name(), "USB mouse");
        assert_eq!(mouse.connection_kind, Some(ConnectionKind::Usb));
        assert!(!mouse.capabilities.dpi);
        assert!(!mouse.capabilities.has_scroll_settings());
        assert!(mouse.capabilities.buttons.is_empty());
        let receiver = standard_mouse_status(0x1234, 0, BusType::Usb, Some("Wireless Receiver"));
        assert_eq!(receiver.connection.as_deref(), Some("USB receiver"));
        let bluetooth =
            standard_mouse_status(0x1234, 0, BusType::Bluetooth, Some("Bluetooth mouse"));
        assert_eq!(bluetooth.connection_kind, Some(ConnectionKind::Bluetooth));
    }
    #[test]
    fn connections_do_not_depend_on_a_mouse_pid_whitelist() {
        assert_eq!(
            classify(BusType::Bluetooth, 0xBFFF, "New mouse"),
            ConnectionKind::Bluetooth
        );
        assert_eq!(
            classify(BusType::Usb, 0xC52B, ""),
            ConnectionKind::UnifyingReceiver
        );
        assert_eq!(
            classify(BusType::Usb, 0xC548, ""),
            ConnectionKind::BoltReceiver
        );
        assert_eq!(
            classify(BusType::Usb, 0xC549, "Logi Bolt Receiver"),
            ConnectionKind::BoltReceiver
        );
        assert_eq!(
            classify(BusType::Usb, 0xC541, ""),
            ConnectionKind::OtherReceiver
        );
        assert_eq!(
            classify(BusType::Usb, 0xC08A, "MX Vertical"),
            ConnectionKind::Usb
        );
        assert!(eligible(VENDOR_LOGITECH, 0xFF43));
        assert!(!eligible(VENDOR_LOGITECH, 1));
        assert!(!eligible(0x1234, 0xFF00));
    }
    #[test]
    fn retains_all_interfaces_and_prefers_long_receiver_reports() {
        let candidate = |path, kind, usage| Candidate {
            path: CString::new(path).unwrap(),
            kind,
            usage,
            product: None,
            product_id: 0,
        };
        let result = ordered_candidates(vec![
            candidate("bt1", ConnectionKind::Bluetooth, 2),
            candidate("rx-short", ConnectionKind::UnifyingReceiver, 1),
            candidate("rx-long", ConnectionKind::UnifyingReceiver, 2),
            candidate("bt2", ConnectionKind::Bluetooth, 2),
            candidate("rx-long", ConnectionKind::UnifyingReceiver, 2),
        ]);
        let paths: Vec<_> = result.iter().map(|c| c.path.to_str().unwrap()).collect();
        assert_eq!(paths, ["rx-long", "rx-short", "bt1", "bt2"]);
    }
}
