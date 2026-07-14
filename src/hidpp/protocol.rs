//! HID++ 1.0 / 2.0 wire constants and report (de)serialization.
//!
//! Packet layout (matching Solaar / logiops):
//!   byte0  report id      0x10 short (7 bytes) / 0x11 long (20 bytes)
//!   byte1  device index   1..=6 via receiver, 0xFF for direct Bluetooth
//!   byte2  feature index  (or 0xFF/0x8F on error reports)
//!   byte3  address        (function << 4) | software id
//!   byte4+ parameters
//!
//! Everything above is used over long reports for HID++ 2.0 traffic; the 2S
//! accepts long reports for all 2.0 features, which keeps the transport simple.

pub const VENDOR_LOGITECH: u16 = 0x046D;
pub const PID_UNIFYING_RECEIVER: u16 = 0xC52B;
pub const PID_MX_MASTER_2S_BT: u16 = 0xB019;

/// Vendor-defined HID usage page carrying HID++ collections.
pub const USAGE_PAGE_HIDPP: u16 = 0xFF00;
/// Long-report (0x11) collection usage.
pub const USAGE_HIDPP_LONG: u16 = 0x0002;

pub const REPORT_ID_SHORT: u8 = 0x10;
pub const REPORT_ID_LONG: u8 = 0x11;
pub const SHORT_LEN: usize = 7;
pub const LONG_LEN: usize = 20;

/// Our software id (low nibble of the address byte, 1..=15). Responses echo it,
/// letting us distinguish command replies from device-initiated events (which
/// carry software id 0).
pub const SW_ID: u8 = 0x0A;

/// IRoot is always reachable at feature index 0.
pub const ROOT_INDEX: u8 = 0x00;

// Feature IDs (looked up to feature *indices* at runtime via IRoot).
pub const FEAT_FEATURE_SET: u16 = 0x0001;
pub const FEAT_DEVICE_INFO: u16 = 0x0003;
pub const FEAT_BATTERY_STATUS: u16 = 0x1000;
pub const FEAT_REPROG_CONTROLS_V4: u16 = 0x1B04;
pub const FEAT_SMART_SHIFT: u16 = 0x2110;
pub const FEAT_HIRES_WHEEL: u16 = 0x2121;
pub const FEAT_ADJUSTABLE_DPI: u16 = 0x2201;

/// Marker in the feature-index byte for a HID++ 2.0 error reply.
pub const ERR20_MARKER: u8 = 0xFF;
/// Sub-id marker for a HID++ 1.0 error reply.
pub const ERR10_MARKER: u8 = 0x8F;

/// Compose the address byte from a function id and our software id.
pub fn address(function: u8) -> u8 {
    (function << 4) | (SW_ID & 0x0F)
}

/// Build a 20-byte long request. `params` is zero-padded / truncated to fit.
pub fn long_request(device_index: u8, feature_index: u8, function: u8, params: &[u8]) -> [u8; LONG_LEN] {
    let mut r = [0u8; LONG_LEN];
    r[0] = REPORT_ID_LONG;
    r[1] = device_index;
    r[2] = feature_index;
    r[3] = address(function);
    let n = params.len().min(LONG_LEN - 4);
    r[4..4 + n].copy_from_slice(&params[..n]);
    r
}

/// Build a 7-byte short request (HID++ 1.0 receiver registers). Kept as a
/// documented reference; the 2S path uses long reports exclusively.
#[allow(dead_code)]
pub fn short_request(device_index: u8, sub_id: u8, address_byte: u8, params: &[u8]) -> [u8; SHORT_LEN] {
    let mut r = [0u8; SHORT_LEN];
    r[0] = REPORT_ID_SHORT;
    r[1] = device_index;
    r[2] = sub_id;
    r[3] = address_byte;
    let n = params.len().min(SHORT_LEN - 4);
    r[4..4 + n].copy_from_slice(&params[..n]);
    r
}

/// A parsed HID++ report (any length). `raw` keeps the exact bytes read so the
/// error layouts can be decoded precisely.
#[derive(Clone)]
pub struct Report {
    pub raw: Vec<u8>,
}

impl Report {
    pub fn new(bytes: &[u8]) -> Report {
        Report {
            raw: bytes.to_vec(),
        }
    }

    fn at(&self, i: usize) -> u8 {
        self.raw.get(i).copied().unwrap_or(0)
    }

    pub fn report_id(&self) -> u8 {
        self.at(0)
    }
    pub fn device_index(&self) -> u8 {
        self.at(1)
    }
    pub fn feature_index(&self) -> u8 {
        self.at(2)
    }
    pub fn address_byte(&self) -> u8 {
        self.at(3)
    }
    pub fn function(&self) -> u8 {
        self.address_byte() >> 4
    }
    pub fn sw_id(&self) -> u8 {
        self.address_byte() & 0x0F
    }
    /// Parameter byte `n` (0-based after the 4-byte header).
    pub fn param(&self, n: usize) -> u8 {
        self.at(4 + n)
    }

    pub fn is_error_20(&self) -> bool {
        self.feature_index() == ERR20_MARKER
    }
    pub fn is_error_10(&self) -> bool {
        self.feature_index() == ERR10_MARKER
    }

    /// For a 2.0 error reply: (failed feature index, failed address, code).
    pub fn error_20(&self) -> (u8, u8, u8) {
        (self.at(3), self.at(4), self.at(5))
    }

    /// True when this looks like a device-initiated event (software id 0) as
    /// opposed to a reply to one of our commands.
    pub fn is_event(&self) -> bool {
        !self.is_error_20() && !self.is_error_10() && self.sw_id() == 0
    }
}

/// Human-readable name for a HID++ 2.0 error code, for logs.
pub fn error_20_name(code: u8) -> &'static str {
    match code {
        0 => "no error",
        1 => "unknown",
        2 => "invalid argument",
        3 => "out of range",
        4 => "hardware error",
        5 => "logitech internal",
        6 => "invalid feature index",
        7 => "invalid function id",
        8 => "busy",
        9 => "unsupported",
        _ => "reserved",
    }
}
