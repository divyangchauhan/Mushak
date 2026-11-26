//! Gestures section: thumb-button gesture actions + tap/swipe threshold.

use super::widgets::{self, Ui2};
use super::{actions, fonts, icons, App, PickerTarget};
use ply_engine::prelude::*;

const ROWS: [(&str, &str); 5] = [
    ("tap", "Tap"),
    ("up", "Swipe up"),
    ("down", "Swipe down"),
    ("left", "Swipe left"),
    ("right", "Swipe right"),
];

fn row_icon(slot: &str) -> &'static GraphicAsset {
    match slot {
        "tap" => &icons::G_TAP,
        "up" => &icons::G_UP,
        "down" => &icons::G_DOWN,
        "left" => &icons::G_LEFT,
        _ => &icons::G_RIGHT,
    }
}

/// Ply's `Id` is built from (&'static str, u32), so slots need a stable index.
fn slot_index(slot: &str) -> u32 {
    match slot {
        "tap" => 0,
        "up" => 1,
        "down" => 2,
        "left" => 3,
        _ => 4,
    }
}

fn swipe_distance_hint(device: &crate::hidpp::device::DeviceStatus, threshold: i64) -> String {
    if let Some(dpi) = device
        .dpi_current
        .filter(|dpi| device.connected && device.capabilities.dpi && *dpi > 0)
    {
        let millimetres = threshold as f64 * 25.4 / f64::from(dpi);
        format!("About {millimetres:.1} mm along either axis at {dpi} DPI. Shorter moves count as taps.")
    } else {
        "Each count is a unit of mouse movement. Shorter moves count as taps.".to_owned()
    }
}

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let compact = screen_width() < 840.0 && screen_height() <= 560.0;
    let fixed_trigger = app
        .device_view
        .device
        .capabilities
        .gesture_buttons
        .is_empty()
        && app.selected_gestures().trigger.is_none();
    let trigger_number = super::mouse_view::target_number(
        &app.device_view.device,
        &PickerTarget::Gesture("tap".into()),
    );
    let hint = if !app.selected_gestures().enabled {
        "Gesture recognition is off.".to_owned()
    } else if compact && fixed_trigger {
        trigger_number.map_or_else(
            || "Tap the gesture button, or hold it and move to swipe.".to_owned(),
            |n| format!("Tap gesture button {n}, or hold it and move to swipe."),
        )
    } else {
        "Tap the gesture button, or hold it and move the mouse to swipe.".to_owned()
    };
    super::buttons::profile_heading(ui, app, "Gestures", &hint);
    ui.element()
        .width(grow!())
        .height(fixed!(if compact { 6.0 } else { 12.0 }))
        .empty();

    if !app.mouse_awake() {
        ui.text(
            "Mouse disconnected. These saved actions will apply when a supported mouse connects.",
            |t| t.font(&fonts::SANS).font_size(13).color(pal.muted),
        );
        ui.element().height(fixed!(14.0)).empty();
    }

    if app.selected_profile > 0
        && app.draft.profiles[app.selected_profile - 1]
            .gestures
            .is_some()
    {
        ui.element()
            .id("gesture_inherit")
            .width(grow!())
            .height(fixed!(30.0))
            .children(|ui| {
                ui.text("Use default profile gestures", |t| {
                    t.font(&fonts::SANS).font_size(12).color(pal.accent)
                });
                if ui.just_pressed() {
                    app.draft.profiles[app.selected_profile - 1].gestures = None;
                    app.commit_config();
                }
            });
    }
    if fixed_trigger {
        if !compact {
            let trigger = trigger_number.map_or_else(
                || "Default gesture button".to_owned(),
                |n| format!("Gesture button {n} on the mouse diagram"),
            );
            ui.text(&format!("Trigger: {trigger}"), |t| {
                t.font(&fonts::SANS_MEDIUM).font_size(14).color(pal.muted)
            });
        }
    } else {
        widgets::eyebrow(ui, &pal, "Gesture trigger");
        ui.element().height(fixed!(5.0)).empty();
        let mut triggers = vec![(None, "Default gesture button".to_string())];
        triggers.extend(
            app.device_view
                .device
                .capabilities
                .gesture_buttons
                .iter()
                .map(|b| (Some(b.cid), b.name.clone())),
        );
        for (index, (cid, name)) in triggers.iter().enumerate() {
            ui.element()
                .id(("gesture_trigger", index as u32))
                .corner_radius(7.0)
                .width(grow!())
                .height(fixed!(30.0))
                .background_color(if app.selected_gestures().trigger == *cid {
                    pal.s2
                } else {
                    pal.win
                })
                .layout(|l| l.align(Left, CenterY).gap(8).padding((0, 10, 0, 10)))
                .children(|ui| {
                    if app.selected_gestures().trigger == *cid {
                        widgets::icon(ui, &icons::CHECK, 14.0, pal.accent);
                    }
                    ui.text(name, |t| {
                        t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.text)
                    });
                    if ui.just_pressed() {
                        app.selected_gestures_mut().trigger = *cid;
                        app.commit_config();
                    }
                });
        }
    }
    ui.element()
        .height(fixed!(if compact { 4.0 } else { 12.0 }))
        .empty();
    let enabled = app.selected_gestures().enabled;
    let threshold = app.selected_gestures().tap_threshold as i64;
    let mut toggle_enabled = false;
    let mut new_threshold: Option<i64> = None;
    let mut open_picker: Option<PickerTarget> = None;

    if super::scroll::switch_row(
        ui,
        app,
        "sw_gestures",
        "Button gestures",
        if compact {
            "Use the tap and swipe actions below."
        } else {
            "Use tap and swipe actions in place of the trigger button's normal action."
        },
        enabled,
    ) {
        toggle_enabled = true;
    }

    if !enabled {
        ui.element().height(fixed!(10.0)).empty();
        ui.text(
            "You can edit these actions now. Turn gestures on to use them.",
            |t| t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text),
        );
    }

    ui.element().width(grow!()).height(fixed!(10.0)).empty();

    if !enabled {
        ui.element()
            .width(fit!())
            .height(fixed!(24.0))
            .corner_radius(6.0)
            .background_color(pal.s2)
            .layout(|l| {
                l.direction(LeftToRight)
                    .align(Left, CenterY)
                    .gap(6)
                    .padding((0, 8, 0, 8))
            })
            .children(|ui| {
                widgets::icon(ui, &icons::NAV_GESTURES, 14.0, pal.muted);
                ui.text("Gesture recognition off", |t| {
                    t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                });
            });
        ui.element().height(fixed!(6.0)).empty();
    }
    let g = app.selected_gestures();
    let row_inset = if compact { 2 } else { 6 };
    let rows: Vec<(String, String, actions::Display)> = ROWS
        .iter()
        .map(|(slot, name)| {
            let a = match *slot {
                "tap" => &g.tap,
                "up" => &g.up,
                "down" => &g.down,
                "left" => &g.left,
                _ => &g.right,
            };
            (slot.to_string(), name.to_string(), actions::display(a))
        })
        .collect();

    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(4))
        .children(|ui| {
            for (slot, name, disp) in &rows {
                widgets::card(ui, &pal)
                    .layout(|l| {
                        l.direction(LeftToRight)
                            .align(Left, CenterY)
                            .gap(16)
                            .padding((row_inset, 16, row_inset, 16))
                    })
                    .children(|ui| {
                        widgets::icon(
                            ui,
                            row_icon(slot),
                            24.0,
                            if enabled { pal.accent } else { pal.muted },
                        );
                        ui.element().width(grow!()).height(fit!()).children(|ui| {
                            ui.text(name, |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                            });
                        });
                        let id: Id = ("g_chip", slot_index(slot)).into();
                        if widgets::action_chip(ui, &pal, id, disp) {
                            open_picker = Some(PickerTarget::Gesture(slot.clone()));
                        }
                    });
            }
        });

    // Put the five assignments before the optional tuning control at every width.
    ui.element()
        .height(fixed!(if compact { 7.0 } else { 12.0 }))
        .empty();
    if enabled {
        let inset = if compact { 8 } else { 16 };
        widgets::card(ui, &pal)
            .layout(|l| {
                l.direction(TopToBottom)
                    .gap(3)
                    .padding((inset, 17, inset, 17))
            })
            .children(|ui| {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                    .children(|ui| {
                        ui.element().width(grow!()).height(fit!()).children(|ui| {
                            ui.text("Minimum swipe distance", |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                            });
                        });
                        ui.text(&format!("{threshold} counts"), |t| {
                            t.font(&fonts::MONO_BOLD).font_size(13).color(pal.accent)
                        });
                    });
                ui.text(
                    &swipe_distance_hint(&app.device_view.device, threshold),
                    |t| {
                        t.font(&fonts::SANS)
                            .font_size(widgets::px(12.5))
                            .color(pal.muted)
                    },
                );
                ui.element()
                    .width(grow!())
                    .height(fixed!(if compact { 6.0 } else { 12.0 }))
                    .empty();
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(12))
                    .children(|ui| {
                        ui.text("Shorter", |t| {
                            t.font(&fonts::MONO_MEDIUM).font_size(11).color(pal.faint)
                        });
                        if let Some(v) =
                            widgets::slider(ui, &pal, "threshold", threshold, 10, 400, 5)
                        {
                            new_threshold = Some(v);
                        }
                        ui.text("Longer", |t| {
                            t.font(&fonts::MONO_MEDIUM).font_size(11).color(pal.faint)
                        });
                    });
            });
    }
    if toggle_enabled {
        app.selected_gestures_mut().enabled = !enabled;
        app.commit_config();
    }
    if let Some(v) = new_threshold {
        app.selected_gestures_mut().tap_threshold = v as i32;
        app.commit_config();
    }
    if let Some(t) = open_picker {
        app.open_picker(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hidpp::device::{DeviceCapabilities, DeviceStatus};

    #[test]
    fn physical_swipe_estimate_requires_a_live_reported_resolution() {
        let mut device = DeviceStatus {
            connected: true,
            capabilities: DeviceCapabilities {
                dpi: true,
                ..Default::default()
            },
            dpi_current: Some(1000),
            ..Default::default()
        };
        assert!(swipe_distance_hint(&device, 60).contains("1.5 mm"));
        device.capabilities.dpi = false;
        assert!(!swipe_distance_hint(&device, 60).contains("mm"));
        device.capabilities.dpi = true;
        device.connected = false;
        assert!(!swipe_distance_hint(&device, 60).contains("mm"));
        device.connected = true;
        device.dpi_current = Some(0);
        assert!(!swipe_distance_hint(&device, 60).contains("mm"));
    }
}
