//! Simulated protocol exchanges. These do not establish physical compatibility.

use super::{
    device::Device,
    protocol::*,
    transport::{ConnectionKind, HidppTransport},
};
use crate::config::SmartShiftMode;
use anyhow::{bail, Result};
use std::sync::{Arc, Mutex};

struct Fixture {
    name: &'static str,
    connection: ConnectionKind,
    features: Vec<u16>,
    multiplier: u8,
    controls: Vec<(u16, bool)>,
    dpi_entries: Vec<u16>,
}

fn simulated_mouse(fixture: Fixture) -> (Device, Arc<Mutex<Vec<Vec<u8>>>>) {
    let writes = Arc::new(Mutex::new(Vec::new()));
    let trace = writes.clone();
    let io = HidppTransport::test(fixture.connection, move |request| -> Result<Report> {
        trace.lock().unwrap().push(request.to_vec());
        let index = request[1];
        let mouse_index = if fixture.connection.is_receiver() {
            2
        } else {
            0xFF
        };
        if index != mouse_index && !(fixture.connection.is_receiver() && index == 1) {
            bail!("empty slot");
        }
        let function = request[3] >> 4;
        let mut payload = Vec::new();
        let feature = if request[2] == 0 {
            0
        } else {
            *fixture
                .features
                .get(request[2] as usize - 1)
                .ok_or_else(|| anyhow::anyhow!("bad feature index"))?
        };
        match (feature, function) {
            (0, 1) => payload.extend_from_slice(&[4, 5, request[6]]),
            (0, 0) => {
                let id = u16::from_be_bytes([request[4], request[5]]);
                payload.extend_from_slice(&[
                    fixture
                        .features
                        .iter()
                        .position(|feature| *feature == id)
                        .map(|i| i as u8 + 1)
                        .unwrap_or(0),
                    0,
                    0,
                ]);
            }
            (FEAT_DEVICE_NAME, 2) => payload.push(if index == mouse_index { 3 } else { 0 }),
            (FEAT_DEVICE_NAME, 0) => payload.push(fixture.name.len() as u8),
            (FEAT_DEVICE_NAME, 1) => payload.extend_from_slice(
                &fixture.name.as_bytes()[request[4] as usize..]
                    [..(fixture.name.len() - request[4] as usize).min(16)],
            ),
            (FEAT_FEATURE_SET, 0) => payload.push(fixture.features.len() as u8),
            (FEAT_FEATURE_SET, 1) => {
                let id = if request[4] == 0 {
                    0
                } else {
                    fixture.features[request[4] as usize - 1]
                };
                payload.extend_from_slice(&id.to_be_bytes());
                payload.push(0);
            }
            (FEAT_REPROG_CONTROLS_V4, 0) => payload.push(fixture.controls.len() as u8),
            (FEAT_REPROG_CONTROLS_V4, 1) => {
                let (cid, raw) = fixture.controls[request[4] as usize];
                payload.extend_from_slice(&cid.to_be_bytes());
                payload.extend_from_slice(&[0, 0, 0x20, 0, 0, 0, u8::from(raw)]);
            }
            (FEAT_REPROG_CONTROLS_V4, 3) => {}
            (FEAT_BATTERY_STATUS, 0) => payload.extend_from_slice(&[80, 50, 0]),
            (FEAT_UNIFIED_BATTERY, 1) => payload.extend_from_slice(&[85, 4, 0, 0]),
            (FEAT_BATTERY_VOLTAGE, 0) => payload.extend_from_slice(&[0x0F, 0xA0, 0]),
            (FEAT_SMART_SHIFT, 0) => payload.extend_from_slice(&[2, 10, 20]),
            (FEAT_SMART_SHIFT_ENHANCED, 1) => payload.extend_from_slice(&[2, 10, 60]),
            (FEAT_SMART_SHIFT, 1) | (FEAT_SMART_SHIFT_ENHANCED, 2) => {}
            (FEAT_HIRES_WHEEL, 0) => payload.extend_from_slice(&[fixture.multiplier, 8]),
            (FEAT_HIRES_WHEEL, 1) => payload.push(0),
            (FEAT_HIRES_WHEEL, 2) => {}
            (FEAT_ADJUSTABLE_DPI, 0) | (FEAT_EXTENDED_ADJUSTABLE_DPI, 0) => payload.push(1),
            (FEAT_ADJUSTABLE_DPI, 1) | (FEAT_EXTENDED_ADJUSTABLE_DPI, 2) => {
                payload.push(0);
                if feature == FEAT_EXTENDED_ADJUSTABLE_DPI {
                    payload.extend_from_slice(&[0, request[6]]);
                }
                let per_page = if feature == FEAT_EXTENDED_ADJUSTABLE_DPI {
                    6
                } else {
                    7
                };
                for value in fixture
                    .dpi_entries
                    .iter()
                    .skip(request[6] as usize * per_page)
                    .take(per_page)
                {
                    payload.extend_from_slice(&value.to_be_bytes());
                }
            }
            (FEAT_EXTENDED_ADJUSTABLE_DPI, 1) => payload.extend_from_slice(&[0, 0, 3]),
            (FEAT_ADJUSTABLE_DPI, 2) | (FEAT_EXTENDED_ADJUSTABLE_DPI, 5) => {
                payload.push(0);
                for value in [1000u16, 1000, 1200, 1200] {
                    payload.extend_from_slice(&value.to_be_bytes());
                }
                payload.push(1);
            }
            (FEAT_ADJUSTABLE_DPI, 3) | (FEAT_EXTENDED_ADJUSTABLE_DPI, 6) => {}
            _ => bail!("unexpected feature {feature:04x} function {function}"),
        }
        let mut reply = vec![REPORT_ID_LONG, index, request[2], request[3]];
        reply.extend_from_slice(&payload);
        reply.resize(LONG_LEN, 0);
        Ok(Report { raw: reply })
    });
    (Device::from_transport(io), writes)
}

fn fixture(
    name: &'static str,
    connection: ConnectionKind,
    battery: u16,
    enhanced: bool,
    vertical: bool,
) -> Fixture {
    let mut features = vec![
        FEAT_DEVICE_NAME,
        FEAT_FEATURE_SET,
        battery,
        FEAT_REPROG_CONTROLS_V4,
        FEAT_ADJUSTABLE_DPI,
    ];
    let mut controls = vec![(0x0052, false), (0x0053, false), (0x0056, false)];
    if vertical {
        controls.push((0x00ED, true));
    } else {
        features.extend_from_slice(&[
            if enhanced {
                FEAT_SMART_SHIFT_ENHANCED
            } else {
                FEAT_SMART_SHIFT
            },
            FEAT_HIRES_WHEEL,
        ]);
        controls.extend_from_slice(&[(0x00C3, true), (0x00C4, false)]);
    }
    Fixture {
        name,
        connection,
        features,
        multiplier: 15,
        controls,
        dpi_entries: vec![400, 0xE064, 8000, 0],
    }
}

#[test]
fn mx_family_discovers_by_capability_across_connections_and_skips_receiver_keyboards() {
    for (name, connection, battery, enhanced, vertical) in [
        (
            "Wireless Mouse MX Master",
            ConnectionKind::Bluetooth,
            FEAT_BATTERY_STATUS,
            false,
            false,
        ),
        (
            "Wireless Mouse MX Master 2S",
            ConnectionKind::UnifyingReceiver,
            FEAT_BATTERY_STATUS,
            false,
            false,
        ),
        (
            "MX Master 3 for Mac",
            ConnectionKind::Bluetooth,
            FEAT_BATTERY_STATUS,
            false,
            false,
        ),
        (
            "MX Master 3S for Business",
            ConnectionKind::BoltReceiver,
            FEAT_UNIFIED_BATTERY,
            false,
            false,
        ),
        (
            "MX Master 4 for Business",
            ConnectionKind::BoltReceiver,
            FEAT_UNIFIED_BATTERY,
            true,
            false,
        ),
        (
            "MX Vertical",
            ConnectionKind::Usb,
            FEAT_BATTERY_STATUS,
            false,
            true,
        ),
        (
            "Future HID++ Mouse",
            ConnectionKind::OtherReceiver,
            FEAT_BATTERY_STATUS,
            false,
            true,
        ),
    ] {
        let mut input = fixture(name, connection, battery, enhanced, vertical);
        let original = name == "Wireless Mouse MX Master";
        if original || name == "Wireless Mouse MX Master 2S" {
            input.multiplier = 8;
        }
        if original {
            input
                .controls
                .iter_mut()
                .find(|(cid, _)| *cid == 0x00C3)
                .unwrap()
                .1 = false;
        }
        let expected_multiplier = input.multiplier;
        let (mut mouse, writes) = simulated_mouse(input);
        mouse.discover().unwrap();
        assert_eq!(mouse.device_name_on(mouse.index).unwrap(), name);
        assert_eq!(mouse.capabilities.gestures, !vertical && !original);
        assert_eq!(mouse.capabilities.smartshift, !vertical);
        assert_eq!(mouse.capabilities.hires_scroll, !vertical);
        assert_eq!(
            mouse.capabilities.wheel_multiplier,
            if vertical {
                None
            } else {
                Some(expected_multiplier)
            }
        );
        assert!(mouse.capabilities.dpi && mouse.capabilities.battery);
        assert_eq!(mouse.dpi_values.first(), Some(&400));
        assert_eq!(mouse.dpi_values.last(), Some(&8000));
        assert_eq!(mouse.index, if connection.is_receiver() { 2 } else { 0xFF });
        mouse.apply_dpi(1550).unwrap();
        if !vertical {
            mouse
                .apply_smartshift(SmartShiftMode::AlwaysRatchet, 10)
                .unwrap();
        }
        let trace = writes.lock().unwrap();
        let dpi_index = mouse.features.dpi.unwrap();
        assert!(trace
            .iter()
            .any(|r| r[2] == dpi_index && r[3] >> 4 == 3 && r[4..7] == [0, 0x05, 0xDC]));
        if enhanced {
            let index = mouse.features.smartshift_enhanced.unwrap();
            assert!(trace
                .iter()
                .any(|r| r[2] == index && r[3] >> 4 == 2 && r[4..7] == [2, 255, 60]));
        }
    }
}

#[test]
fn extended_dpi_writes_preserve_the_other_axis_and_lift_off_distance() {
    let mut config = fixture(
        "Extended DPI Mouse",
        ConnectionKind::Usb,
        FEAT_UNIFIED_BATTERY,
        false,
        true,
    );
    *config
        .features
        .iter_mut()
        .find(|feature| **feature == FEAT_ADJUSTABLE_DPI)
        .unwrap() = FEAT_EXTENDED_ADJUSTABLE_DPI;
    let (mut mouse, writes) = simulated_mouse(config);
    mouse.discover().unwrap();
    assert!(mouse.capabilities.dpi);
    mouse.apply_dpi(1550).unwrap();
    assert!(writes
        .lock()
        .unwrap()
        .iter()
        .any(|r| r[2] == mouse.features.extended_dpi.unwrap()
            && r[3] >> 4 == 6
            && r[4..10] == [0, 0x05, 0xDC, 0x04, 0xB0, 1]));
}

#[test]
fn extra_button_diversion_is_temporary_and_restored_on_pause_or_exit() {
    let (mut mouse, writes) = simulated_mouse(fixture(
        "MX Vertical",
        ConnectionKind::Usb,
        FEAT_BATTERY_STATUS,
        false,
        true,
    ));
    mouse.discover().unwrap();
    let mut map = crate::config::ButtonMap::default();
    map.controls
        .insert("00ed".into(), crate::config::Action::Disabled);
    mouse.apply_control_diverts_with_map(false, true, &map);
    let index = mouse.features.reprog.unwrap();
    let is_dpi_button = |r: &Vec<u8>| r[2] == index && r[3] >> 4 == 3 && r[4..6] == [0, 0xED];
    assert_eq!(
        writes
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|r| is_dpi_button(r))
            .unwrap()[6],
        0x23
    );
    mouse.apply_control_diverts_with_map(false, false, &map);
    assert_eq!(
        writes
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|r| is_dpi_button(r))
            .unwrap()[6],
        0x22
    );
    mouse.restore_diverts();
    assert_eq!(
        writes
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|r| is_dpi_button(r))
            .unwrap()[6],
        0x22
    );
}

#[test]
fn invalid_dpi_choices_hide_the_setting_and_prevent_writes() {
    let mut input = fixture(
        "Unavailable DPI Mouse",
        ConnectionKind::Usb,
        FEAT_BATTERY_STATUS,
        false,
        true,
    );
    input.dpi_entries = vec![0];
    let (mut mouse, writes) = simulated_mouse(input);
    mouse.discover().unwrap();
    assert!(!mouse.capabilities.dpi);
    assert!(!mouse.capabilities.has_scroll_settings());
    assert!(!mouse.capabilities.gestures);
    mouse.apply_dpi(1000).unwrap();
    assert!(!writes
        .lock()
        .unwrap()
        .iter()
        .any(|r| r[2] == mouse.features.dpi.unwrap() && r[3] >> 4 == 3));
}

#[test]
fn extended_dpi_reads_every_page_of_discrete_choices() {
    let mut input = fixture(
        "Paged DPI Mouse",
        ConnectionKind::Usb,
        FEAT_BATTERY_STATUS,
        false,
        true,
    );
    *input
        .features
        .iter_mut()
        .find(|feature| **feature == FEAT_ADJUSTABLE_DPI)
        .unwrap() = FEAT_EXTENDED_ADJUSTABLE_DPI;
    input.dpi_entries = vec![400, 500, 600, 700, 800, 900, 1000, 1200, 1400, 1600, 0];
    let (mut mouse, _) = simulated_mouse(input);
    mouse.discover().unwrap();
    assert_eq!(
        mouse.dpi_values,
        [400, 500, 600, 700, 800, 900, 1000, 1200, 1400, 1600]
    );
    assert!(mouse.capabilities.dpi);
}
