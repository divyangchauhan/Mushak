//! The HID++ device engine: owns the transport, runs synchronous
//! request/response over long reports, routes device-initiated events, and
//! survives sleep/reconnect. Everything runs on one dedicated thread so the
//! non-`Sync` HID handle is never touched concurrently.

use super::features::reprog::ControlInfo;
use super::features::{battery, reprog};
use super::protocol::{self, Report, *};
use super::transport::{ConnectionKind, HidppTransport};
use crate::{gestures, state};
use anyhow::{bail, Result};
use crossbeam_channel::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Resolved feature id -> feature index map (indices vary by firmware).
#[derive(Default, Clone)]
pub struct FeatureMap {
    pub feature_set: Option<u8>,
    pub device_info: Option<u8>,
    pub battery: Option<u8>,
    pub reprog: Option<u8>,
    pub smartshift: Option<u8>,
    pub hires_wheel: Option<u8>,
    pub dpi: Option<u8>,
    /// Full (id, index) list for the Device tab / logs.
    pub all: Vec<(u16, u8)>,
}

/// Snapshot of device state published to the GUI/tray (via `state`) and to the
/// settings subprocess (via the status file).
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DeviceStatus {
    pub connected: bool,
    pub connection: Option<String>,
    pub battery_percent: Option<u8>,
    pub charging: bool,
    pub firmware: Option<String>,
    pub features: Vec<(u16, u8)>,
    // Populated from phase 3 on.
    pub dpi_current: Option<u16>,
    pub dpi_list: Vec<u16>,
    pub smartshift_freespin: Option<bool>,
    pub smartshift_threshold: Option<u8>,
    pub hires_on: Option<bool>,
}

/// Commands sent to the device thread.
pub enum DeviceCommand {
    /// Apply the current `config.device` (SmartShift, DPI, hi-res).
    ApplyDeviceConfig,
    /// Re-assert every control's divert state per `config.gestures`.
    ApplyControlDiverts,
    /// Stop the thread.
    Shutdown,
}

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn spawn() -> Sender<DeviceCommand> {
    let (tx, rx) = crossbeam_channel::unbounded::<DeviceCommand>();
    std::thread::Builder::new()
        .name("hid-device".into())
        .spawn(move || run(rx))
        .expect("spawn hid-device thread");
    tx
}

pub fn request_stop() {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

fn run(rx: Receiver<DeviceCommand>) {
    tracing::debug!("hid-device thread started");
    while !SHUTDOWN.load(Ordering::Relaxed) {
        match Device::connect() {
            Ok(dev) => {
                dev.apply_device_config();
                dev.apply_diverts();
                dev.event_loop(&rx);
                if SHUTDOWN.load(Ordering::Relaxed) {
                    dev.restore_diverts();
                }
                state::update_device_status(|s| s.connected = false);
            }
            Err(e) => {
                tracing::debug!("device not connected: {e:#}");
                state::set_device_status(DeviceStatus::default());
            }
        }
        if SHUTDOWN.load(Ordering::Relaxed) {
            break;
        }
        // Retry every ~3s, staying responsive to shutdown.
        for _ in 0..30 {
            if SHUTDOWN.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    tracing::info!("hid-device thread stopped");
}

pub struct Device {
    pub(crate) transport: HidppTransport,
    pub(crate) index: u8,
    pub(crate) features: FeatureMap,
    /// Reprogrammable controls (0x1B04), read once per connection.
    pub(crate) controls: Vec<ControlInfo>,
}

impl Device {
    /// Try each HID++ interface (receiver first, then Bluetooth) and return the
    /// first on which feature discovery succeeds.
    pub fn connect() -> Result<Device> {
        let candidates = HidppTransport::candidates()?;
        let mut last_err = None;
        for c in &candidates {
            match HidppTransport::open_candidate(c) {
                Ok(transport) => {
                    let mut dev = Device {
                        transport,
                        index: 0xFF,
                        features: FeatureMap::default(),
                        controls: Vec::new(),
                    };
                    match dev.discover() {
                        Ok(()) => return Ok(dev),
                        Err(e) => {
                            tracing::debug!("discovery failed on {:?}: {e:#}", c.kind);
                            last_err = Some(e);
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("open failed on {:?}: {e:#}", c.kind);
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no working HID++ interface")))
    }

    fn discover(&mut self) -> Result<()> {
        self.index = match self.transport.kind {
            ConnectionKind::Bluetooth => 0xFF,
            ConnectionKind::Receiver => self.probe_receiver_index()?,
        };

        let (maj, min) = self.ping(self.index)?;
        tracing::info!(
            "device index {} online, HID++ protocol {}.{}",
            self.index,
            maj,
            min
        );

        self.features.feature_set = self.lookup(FEAT_FEATURE_SET);
        self.features.device_info = self.lookup(FEAT_DEVICE_INFO);
        self.features.battery = self.lookup(FEAT_BATTERY_STATUS);
        self.features.reprog = self.lookup(FEAT_REPROG_CONTROLS_V4);
        self.features.smartshift = self.lookup(FEAT_SMART_SHIFT);
        self.features.hires_wheel = self.lookup(FEAT_HIRES_WHEEL);
        self.features.dpi = self.lookup(FEAT_ADJUSTABLE_DPI);

        self.enumerate_features();

        if self.features.reprog.is_some() {
            self.controls = self.enumerate_controls();
            if !self.controls.iter().any(|c| c.cid == reprog::CID_GESTURE) {
                tracing::warn!(
                    "gesture CID {:#06x} not found; gestures may be unavailable",
                    reprog::CID_GESTURE
                );
            }
        }

        let firmware = self.firmware_string().ok();
        let battery = self.read_battery().ok();

        state::set_device_status(DeviceStatus {
            connected: true,
            connection: Some(self.transport.kind.label(self.index)),
            battery_percent: battery.map(|b| b.0),
            charging: battery.map(|b| b.1).unwrap_or(false),
            firmware,
            features: self.features.all.clone(),
            ..Default::default()
        });
        self.read_device_settings();
        Ok(())
    }

    /// Probe receiver device indices 1..=6 and return the first that answers.
    fn probe_receiver_index(&self) -> Result<u8> {
        for i in 1..=6u8 {
            match self.ping(i) {
                Ok((maj, min)) => {
                    tracing::info!("receiver slot {i} responded (protocol {maj}.{min})");
                    return Ok(i);
                }
                Err(e) => tracing::debug!("receiver slot {i} silent: {e:#}"),
            }
        }
        bail!("no paired device answered on receiver slots 1..=6")
    }

    fn lookup(&self, feature_id: u16) -> Option<u8> {
        match self.get_feature(feature_id) {
            Ok(Some((idx, _t, ver))) => {
                tracing::info!("feature {feature_id:#06x} -> index {idx} (v{ver})");
                Some(idx)
            }
            Ok(None) => {
                tracing::warn!("feature {feature_id:#06x} not present on this firmware");
                None
            }
            Err(e) => {
                tracing::warn!("feature {feature_id:#06x} lookup failed: {e:#}");
                None
            }
        }
    }

    // ---- Core request/response ------------------------------------------------

    /// Send a command to `self.index` and await the matching reply.
    pub(crate) fn request(&self, feature_index: u8, function: u8, params: &[u8]) -> Result<Report> {
        self.request_on(self.index, feature_index, function, params)
    }

    /// Send a command to an explicit device index (used for receiver probing).
    pub(crate) fn request_on(
        &self,
        device_index: u8,
        feature_index: u8,
        function: u8,
        params: &[u8],
    ) -> Result<Report> {
        self.request_timeout(
            device_index,
            feature_index,
            function,
            params,
            Duration::from_millis(700),
        )
    }

    /// Core request/response with a caller-specified overall timeout.
    pub(crate) fn request_timeout(
        &self,
        device_index: u8,
        feature_index: u8,
        function: u8,
        params: &[u8],
        timeout: Duration,
    ) -> Result<Report> {
        let req = protocol::long_request(device_index, feature_index, function, params);
        let addr = protocol::address(function);
        self.transport.write(&req)?;

        let deadline = Instant::now() + timeout;
        loop {
            if Instant::now() >= deadline {
                bail!(
                    "HID++ timeout (feat_idx={feature_index:#04x} fn={function} idx={device_index})"
                );
            }
            let Some(rep) = self.transport.read(120)? else {
                continue;
            };

            if rep.device_index() != device_index {
                self.route_event(&rep);
                continue;
            }
            if rep.is_error_20() {
                let (fidx, faddr, code) = rep.error_20();
                if fidx == feature_index && faddr == addr {
                    bail!(
                        "HID++ 2.0 error {code:#04x} ({}) feat_idx={feature_index:#04x} fn={function}",
                        protocol::error_20_name(code)
                    );
                }
                continue;
            }
            if rep.is_error_10() {
                // A 1.0 error not for us (e.g. an empty receiver slot ping).
                continue;
            }
            if rep.feature_index() == feature_index && rep.address_byte() == addr {
                return Ok(rep);
            }
            // Anything else is a device-initiated event.
            self.route_event(&rep);
        }
    }

    // ---- Event routing --------------------------------------------------------

    fn route_event(&self, rep: &Report) {
        // Unifying receiver device-connection notification (short, sub-id 0x41):
        // the mouse woke / reconnected. Diverts and volatile settings reset on
        // sleep, so re-apply everything.
        if rep.report_id() == REPORT_ID_SHORT && rep.feature_index() == 0x41 {
            tracing::info!("receiver connection notification (0x41): re-applying settings");
            self.apply_device_config();
            self.apply_diverts();
            let _ = self.refresh();
            return;
        }

        if Some(rep.feature_index()) == self.features.battery && rep.is_event() {
            let (pct, charging) = battery::interpret(rep.param(0), rep.param(2));
            tracing::info!("battery event: {pct}% charging={charging}");
            state::update_device_status(|s| {
                s.battery_percent = Some(pct);
                s.charging = charging;
            });
            return;
        }

        if Some(rep.feature_index()) == self.features.reprog && rep.is_event() {
            self.on_reprog_event(rep);
            return;
        }

        // Wheel movement, diverted to us because high-resolution scrolling is on.
        if Some(rep.feature_index()) == self.features.hires_wheel && rep.is_event() {
            self.on_wheel_event(rep);
            return;
        }

        tracing::debug!("unhandled HID++ event: {}", crate::logging::hex(&rep.raw));
    }

    pub(crate) fn refresh(&self) -> Result<()> {
        if let Ok((pct, charging)) = self.read_battery() {
            state::update_device_status(|s| {
                s.battery_percent = Some(pct);
                s.charging = charging;
            });
        }
        Ok(())
    }

    // ---- Main loop ------------------------------------------------------------

    fn event_loop(&self, rx: &Receiver<DeviceCommand>) {
        tracing::info!("device event loop running");
        let reapply_every = Duration::from_secs(30);
        let mut last_reapply = Instant::now();

        loop {
            if SHUTDOWN.load(Ordering::Relaxed) {
                return;
            }

            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    DeviceCommand::Shutdown => return,
                    DeviceCommand::ApplyDeviceConfig => self.apply_device_config(),
                    DeviceCommand::ApplyControlDiverts => self.apply_diverts(),
                }
            }

            // Periodically re-assert diverts/settings so they survive a sleep we
            // did not otherwise observe (they reset when the 2S powers down), and
            // refresh the battery reading in case no broadcast arrived.
            if last_reapply.elapsed() >= reapply_every {
                self.apply_device_config();
                self.apply_diverts();
                let _ = self.refresh();
                last_reapply = Instant::now();
            }

            match self.transport.read(200) {
                Ok(Some(rep)) => self.route_event(&rep),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("HID read failed, treating as disconnect: {e:#}");
                    return;
                }
            }
        }
    }

    // ---- Orchestration hooks --------------------------------------------------

    /// Apply `config.device` (SmartShift, DPI, hi-res) and refresh the published
    /// settings snapshot.
    pub(crate) fn apply_device_config(&self) {
        let cfg = state::config();
        let d = &cfg.device;
        if let Err(e) = self.apply_smartshift(d.smartshift, d.smartshift_threshold) {
            tracing::warn!("apply smartshift failed: {e:#}");
        }
        if let Err(e) = self.apply_hires(d.hires_scroll, d.invert_scroll) {
            tracing::warn!("apply hires failed: {e:#}");
        }
        if let Err(e) = self.apply_dpi(d.dpi) {
            tracing::warn!("apply dpi failed: {e:#}");
        }
        self.read_device_settings();
    }

    /// Read current device settings (DPI list/current, SmartShift, hi-res) into
    /// the published status so the GUI can show live values.
    pub(crate) fn read_device_settings(&self) {
        let dpi_list = if self.features.dpi.is_some() {
            self.get_dpi_list().unwrap_or_default()
        } else {
            Vec::new()
        };
        let dpi_current = if self.features.dpi.is_some() {
            self.get_dpi().ok().map(|(cur, _)| cur)
        } else {
            None
        };
        let (ss_freespin, ss_threshold) = match self.get_smartshift() {
            Ok((wheel_mode, auto, _)) => (Some(wheel_mode == 1), Some(auto)),
            Err(_) => (None, None),
        };
        let hires_on = self.get_wheel_mode().ok().map(|m| m & 0x02 != 0);

        state::update_device_status(|s| {
            s.dpi_list = dpi_list;
            s.dpi_current = dpi_current;
            s.smartshift_freespin = ss_freespin;
            s.smartshift_threshold = ss_threshold;
            s.hires_on = hires_on;
        });
    }

    /// Re-assert the divert state of every control per config.
    pub(crate) fn apply_diverts(&self) {
        if self.features.reprog.is_none() {
            return;
        }
        self.apply_control_diverts(state::config().gestures.enabled);
    }

    /// Restore native behavior on every control (called on shutdown).
    ///
    /// The wheel is included: high-resolution scrolling diverts it to HID++, and
    /// a wheel left diverted after we exit reports to nobody — scrolling would
    /// simply stop until the mouse power-cycles.
    pub(crate) fn restore_diverts(&self) {
        if self.features.reprog.is_some() {
            self.apply_control_diverts(false);
        }
        self.restore_wheel();
    }

    /// Handle a diverted REPROG_CONTROLS_V4 event (button hold or raw XY).
    pub(crate) fn on_reprog_event(&self, rep: &Report) {
        if rep.function() == 0 {
            // divertedButtonsEvent: up to 4 currently-held control ids.
            let mut cids = Vec::new();
            let mut gesture_held = false;
            for i in 0..4 {
                let cid = ((rep.param(i * 2) as u16) << 8) | rep.param(i * 2 + 1) as u16;
                if cid != 0 {
                    cids.push(cid);
                    if cid == reprog::CID_GESTURE {
                        gesture_held = true;
                    }
                }
            }
            tracing::debug!("diverted buttons: {:04x?}", cids);
            gestures::handle_buttons(gesture_held);
        } else {
            // divertedRawMouseXYEvent: signed big-endian dx, dy.
            let dx = (((rep.param(0) as u16) << 8) | rep.param(1) as u16) as i16 as i32;
            let dy = (((rep.param(2) as u16) << 8) | rep.param(3) as u16) as i16 as i32;
            tracing::debug!("diverted rawXY (event {}): dx={dx} dy={dy}", rep.function());
            gestures::handle_raw_xy(dx, dy);
        }
    }
}
