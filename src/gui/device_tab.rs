//! Device section: connection status, battery, firmware, DPI, HID++ features.
//! Live values come from the status file the resident process publishes.

use super::widgets::{self, Ui2};
use super::{fonts, icons, App};
use ply_engine::prelude::*;

/// Names for the HID++ features Mushak knows about. `status.json` only carries
/// (id, index), so the design's readable names live here.
fn feature_name(id: u16) -> &'static str {
    match id {
        0x0000 => "Root",
        0x0001 => "Feature set",
        0x0003 => "Device information",
        0x0005 => "Device name",
        0x1000 => "Battery status",
        0x1001 => "Battery voltage",
        0x1004 => "Unified battery",
        0x1814 => "Change host",
        0x1b04 => "Reprogrammable controls v4",
        0x1982 => "Backlight",
        0x2100 => "Vertical scrolling",
        0x2110 => "SmartShift",
        0x2111 => "Enhanced SmartShift",
        0x2121 => "Hi-res wheel",
        0x2201 => "Adjustable DPI",
        0x2202 => "Extended adjustable DPI",
        0x2205 => "Pointer motion",
        _ => "Unidentified feature",
    }
}

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    if app.hidpp_expanded && app.mouse_awake() && !app.device_view.device.features.is_empty() {
        feature_details(ui, app);
        return;
    }
    let mut refresh = false;

    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).align(Left, Top).gap(16))
        .children(|ui| {
            ui.element().width(grow!()).height(fit!()).children(|ui| {
                widgets::heading(
                    ui,
                    &pal,
                    "Device",
                    if app.mouse_awake() {
                        app.device_view.device.display_name()
                    } else {
                        ""
                    },
                );
            });
            if !app.mouse_awake() || app.device_view.device.settings_unavailable_reason.is_some() {
                return;
            }
            let hovered = ui.pointer_over("btn_reapply");
            ui.element()
                .id("btn_reapply")
                .width(fit!())
                .height(fixed!(38.0))
                .corner_radius(10.0)
                .background_color(pal.surface)
                .border(|b| {
                    b.all(1)
                        .color(if hovered { pal.accent } else { pal.line_strong })
                })
                .layout(|l| {
                    l.direction(LeftToRight)
                        .align(Left, CenterY)
                        .gap(7)
                        .padding((0, 13, 0, 13))
                })
                .children(|ui| {
                    if ui.just_pressed() {
                        refresh = true;
                    }
                    widgets::icon(ui, &icons::REFRESH, 15.0, pal.text);
                    ui.text("Reapply mouse settings", |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                    });
                });
        });

    ui.element().width(grow!()).height(fixed!(22.0)).empty();

    if !app.mouse_awake() {
        widgets::asleep_card(ui, &pal, &app.device_view.device);
        ui.element().height(fixed!(12.0)).empty();
        ui.element()
            .id("offline_mappings")
            .height(fixed!(36.0))
            .corner_radius(7.0)
            .background_color(pal.surface)
            .border(|b| b.all(1).color(pal.line_strong))
            .accessibility(|a| a.button("Edit button mappings"))
            .layout(|l| {
                l.direction(LeftToRight)
                    .align(Left, CenterY)
                    .gap(8)
                    .padding((0, 12, 0, 12))
            })
            .children(|ui| {
                widgets::icon(ui, &icons::NAV_BUTTONS, 15.0, pal.accent);
                ui.text("Edit button mappings", |t| {
                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                });
                if ui.just_pressed() {
                    app.section = super::Section::Buttons;
                }
            });
        return;
    }

    let d = app.device_view.device.clone();
    if !super::Section::Buttons.available(&d) {
        widgets::card(ui, &pal)
            .width(grow!())
            .height(fit!())
            .layout(|l| l.direction(TopToBottom).gap(5).padding((10, 14, 10, 14)))
            .children(|ui| {
                if let Some(reason) = &d.settings_unavailable_reason {
                    ui.text(reason, |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(15).color(pal.text)
                    });
                }
                ui.text(
                    "Connect a mouse with remappable buttons to edit its mappings.",
                    |t| t.font(&fonts::SANS_MEDIUM).font_size(14).color(pal.text),
                );
            });
        ui.element().height(fixed!(10.0)).empty();
    } else if let Some(reason) = &d.settings_unavailable_reason {
        ui.text(reason, |text| {
            text.font(&fonts::SANS).font_size(13).color(pal.muted)
        });
        ui.element().width(grow!()).height(fixed!(10.0)).empty();
    }
    let battery = d.battery_percent;
    let batt_color = match battery {
        Some(p) if p <= 20 => pal.danger,
        Some(p) if p <= 45 => pal.warn,
        _ => pal.good,
    };

    // Four stat tiles, two per row.
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).gap(10))
        .children(|ui| {
            stat_tile(ui, app, "Connection", |ui| {
                let icon = if d.connection_kind
                    == Some(crate::hidpp::transport::ConnectionKind::Bluetooth)
                {
                    &icons::BLUETOOTH
                } else {
                    &icons::NAV_DEVICE
                };
                widgets::icon(ui, icon, 16.0, pal.accent);
                ui.text(d.connection.as_deref().unwrap_or("—"), |t| {
                    t.font(&fonts::SANS_SEMIBOLD).font_size(15).color(pal.text)
                });
            });
            if d.firmware.is_some() {
                stat_tile(ui, app, "Firmware", |ui| {
                    ui.text(d.firmware.as_deref().unwrap_or("—"), |t| {
                        t.font(&fonts::MONO_SEMIBOLD).font_size(15).color(pal.text)
                    });
                });
            }
        });
    ui.element().width(grow!()).height(fixed!(10.0)).empty();
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).gap(10))
        .children(|ui| {
            if d.capabilities.battery {
                stat_tile(ui, app, "Battery", |ui| {
                    ui.element()
                        .width(grow!())
                        .height(fixed!(8.0))
                        .corner_radius(99.0)
                        .background_color(pal.s2)
                        .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                        .children(|ui| {
                            if battery.is_none() {
                                return;
                            }
                            let frac = battery.unwrap_or(0) as f32 / 100.0;
                            ui.element()
                                .width(ply_engine::layout::Sizing::Percent(frac.max(0.01)))
                                .height(fixed!(8.0))
                                .corner_radius(99.0)
                                .background_color(batt_color)
                                .empty();
                        });
                    ui.text(&d.battery_label(), |t| {
                        t.font(&fonts::MONO_BOLD).font_size(15).color(pal.text)
                    });
                    if d.charging {
                        ui.text("charging", |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(11).color(pal.good)
                        });
                    }
                });
            }
            if d.capabilities.dpi
                && (d.dpi_list.is_empty()
                    || d.dpi_current.is_some_and(|v| v != app.draft.device.dpi))
            {
                stat_tile(ui, app, "Reported DPI", |ui| {
                    ui.text(
                        &d.dpi_current.map(|v| v.to_string()).unwrap_or("—".into()),
                        |t| t.font(&fonts::MONO_BOLD).font_size(15).color(pal.accent),
                    );
                });
            }
        });

    ui.element().width(grow!()).height(fixed!(10.0)).empty();

    let mut new_dpi: Option<u16> = None;
    if d.capabilities.dpi && !d.dpi_list.is_empty() {
        let choices = &d.dpi_list;
        let min = choices[0];
        let max = choices[choices.len() - 1];
        let selected = choices
            .iter()
            .enumerate()
            .min_by_key(|(_, value)| {
                (i32::from(**value) - i32::from(app.draft.device.dpi)).unsigned_abs()
            })
            .map(|(index, _)| index)
            .unwrap_or(0);
        let dpi = choices[selected];

        widgets::card(ui, &pal)
            .layout(|l| l.direction(TopToBottom).gap(12).padding((16, 17, 16, 17)))
            .children(|ui| {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                    .children(|ui| {
                        ui.element().width(grow!()).height(fit!()).children(|ui| {
                            ui.text("Pointer speed in DPI", |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                            });
                        });
                        ui.text(&dpi.to_string(), |t| {
                            t.font(&fonts::MONO_BOLD).font_size(15).color(pal.accent)
                        });
                    });
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(12))
                    .children(|ui| {
                        ui.text(&min.to_string(), |t| {
                            t.font(&fonts::MONO_MEDIUM).font_size(12).color(pal.muted)
                        });
                        if choices.len() > 1 {
                            if let Some(v) = widgets::slider(
                                ui,
                                &pal,
                                "dpi",
                                selected as i64,
                                0,
                                (choices.len() - 1) as i64,
                                1,
                            ) {
                                new_dpi = choices.get(v as usize).copied();
                            }
                        }
                        ui.text(&max.to_string(), |t| {
                            t.font(&fonts::MONO_MEDIUM).font_size(12).color(pal.muted)
                        });
                    });
            });

        ui.element().width(grow!()).height(fixed!(10.0)).empty();
    }

    // HID++ feature list (diagnostic).
    let features = d.features.clone();
    let mut toggle = false;
    if !features.is_empty() {
        widgets::card(ui, &pal)
            .layout(|l| l.direction(TopToBottom).padding(0))
            .children(|ui| {
                ui.element()
                    .id("hidpp_head")
                    .width(grow!())
                    .height(fixed!(46.0))
                    .layout(|l| {
                        l.direction(LeftToRight)
                            .align(Left, CenterY)
                            .gap(10)
                            .padding((0, 17, 0, 17))
                    })
                    .children(|ui| {
                        if ui.just_pressed() {
                            toggle = true;
                        }
                        widgets::icon(ui, &icons::CHEVRON_RIGHT, 14.0, pal.muted);
                        ui.element().width(grow!()).height(fit!()).children(|ui| {
                            ui.text("Reported mouse features", |t| {
                                t.font(&fonts::SANS_SEMIBOLD)
                                    .font_size(widgets::px(13.5))
                                    .color(pal.text)
                            });
                        });
                        ui.text(&format!("{} detected", features.len()), |t| {
                            t.font(&fonts::MONO_MEDIUM)
                                .font_size(widgets::px(11.5))
                                .color(pal.faint)
                        });
                    });
            });
    }
    if toggle {
        app.hidpp_expanded = true;
    }
    if let Some(v) = new_dpi {
        app.draft.device.dpi = v;
        app.commit_config();
    }
    if refresh {
        app.device_view = crate::statusfile::read();
        app.commit_config();
    }
}

fn feature_details(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let d = &app.device_view.device;
    ui.element()
        .id("diagnostics_back")
        .height(fixed!(30.0))
        .corner_radius(7.0)
        .background_color(pal.surface)
        .accessibility(|a| a.button("Back to device"))
        .layout(|l| {
            l.direction(LeftToRight)
                .gap(6)
                .align(Left, CenterY)
                .padding((0, 10, 0, 10))
        })
        .children(|ui| {
            if ui.just_pressed() {
                app.hidpp_expanded = false;
            }
            widgets::icon(ui, &icons::BTN_BACK, 14.0, pal.accent);
            ui.text("Device", |t| {
                t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
            });
        });
    ui.element().height(fixed!(12.0)).empty();
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(14))
        .children(|ui| {
            ui.element().width(grow!()).height(fit!()).children(|ui| {
                widgets::heading(
                    ui,
                    &pal,
                    "HID++ features",
                    &format!(
                        "{} hardware feature IDs reported by {}.",
                        d.features.len(),
                        d.display_name()
                    ),
                );
            });
            ui.element()
                .id("copy_device_report")
                .height(fixed!(34.0))
                .corner_radius(7.0)
                .background_color(pal.surface)
                .border(|b| b.all(1).color(pal.line_strong))
                .accessibility(|a| a.button("Copy device report"))
                .layout(|l| {
                    l.direction(LeftToRight)
                        .align(CenterX, CenterY)
                        .gap(6)
                        .padding((0, 12, 0, 12))
                })
                .children(|ui| {
                    widgets::icon(ui, &icons::EDIT_COPY, 15.0, pal.muted);
                    ui.text("Copy device report", |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                    });
                    if ui.just_pressed() {
                        miniquad::window::clipboard_set(&device_report(d));
                    }
                });
        });
    ui.element().height(fixed!(16.0)).empty();
    let mut named = Vec::new();
    let mut other = Vec::new();
    for (id, _) in &d.features {
        let name = feature_name(*id);
        if name == "Unidentified feature" {
            other.push(format!("{id:#06x}"));
        } else {
            named.push((*id, name));
        }
    }
    if !named.is_empty() {
        let columns = if screen_width() >= 840.0 { 2 } else { 1 };
        widgets::card(ui, &pal)
            .layout(|l| l.direction(TopToBottom).padding((10, 16, 10, 16)))
            .children(|ui| {
                for row in named.chunks(columns) {
                    ui.element()
                        .width(grow!())
                        .height(fixed!(32.0))
                        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(16))
                        .children(|ui| {
                            for (id, name) in row {
                                ui.element()
                                    .width(grow!(0.0))
                                    .height(fit!())
                                    .layout(|l| {
                                        l.direction(LeftToRight).align(Left, CenterY).gap(10)
                                    })
                                    .children(|ui| {
                                        ui.text(&format!("{id:#06x}"), |t| {
                                            t.font(&fonts::MONO_MEDIUM)
                                                .font_size(14)
                                                .color(pal.accent)
                                        });
                                        ui.text(name, |t| {
                                            t.font(&fonts::SANS_MEDIUM)
                                                .font_size(15)
                                                .color(pal.text)
                                        });
                                    });
                            }
                        });
                }
            });
    }
    if !other.is_empty() {
        ui.element().height(fixed!(16.0)).empty();
        ui.element()
            .width(grow!())
            .height(fit!())
            .layout(|l| l.align(Left, CenterY).gap(12))
            .children(|ui| {
                ui.element().width(grow!()).height(fit!()).children(|ui| {
                    ui.text(&format!("{} unnamed features", other.len()), |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(16).color(pal.text)
                    });
                });
                ui.element()
                    .id("copy_feature_ids")
                    .height(fixed!(32.0))
                    .corner_radius(7.0)
                    .background_color(pal.s2)
                    .accessibility(|a| a.button("Copy unnamed IDs"))
                    .layout(|l| l.align(CenterX, CenterY).gap(6).padding((0, 12, 0, 12)))
                    .children(|ui| {
                        widgets::icon(ui, &icons::EDIT_COPY, 15.0, pal.accent);
                        ui.text("Copy unnamed IDs", |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                        });
                        if ui.just_pressed() {
                            miniquad::window::clipboard_set(&other.join(", "));
                        }
                    });
            });
        ui.element().height(fixed!(6.0)).empty();
        ui.text(
            "Use these IDs when reporting a device compatibility issue.",
            |t| t.font(&fonts::SANS).font_size(13).color(pal.muted),
        );
        let per_line = ((screen_width() - 260.0).clamp(70.0, 700.0) / 70.0) as usize;
        for row in other.chunks(per_line) {
            ui.element().height(fixed!(6.0)).empty();
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).gap(6))
                .children(|ui| {
                    for id in row {
                        ui.element()
                            .height(fixed!(26.0))
                            .corner_radius(5.0)
                            .background_color(pal.s2)
                            .layout(|l| l.align(CenterX, CenterY).padding((0, 8, 0, 8)))
                            .children(|ui| {
                                ui.text(id, |t| {
                                    t.font(&fonts::MONO_MEDIUM).font_size(13).color(pal.text)
                                });
                            });
                    }
                });
        }
    }
}

/// One labelled stat tile.
fn stat_tile(ui: &mut Ui2, app: &App, label: &str, body: impl FnOnce(&mut Ui2)) {
    let pal = app.pal;
    ui.element()
        .width(grow!())
        .height(fit!())
        .corner_radius(13.0)
        .background_color(pal.surface)
        .border(|b| b.all(1).color(pal.line))
        .layout(|l| l.direction(TopToBottom).gap(7).padding((15, 17, 15, 17)))
        .children(|ui| {
            ui.text(label, |t| {
                t.font(&fonts::SANS_SEMIBOLD)
                    .font_size(widgets::px(11.5))
                    .color(pal.faint)
            });
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(8))
                .children(body);
        });
}

fn device_report(d: &crate::hidpp::device::DeviceStatus) -> String {
    use std::fmt::Write;
    let mut report = format!(
        "{}\nHID++ features ({})\n",
        d.display_name(),
        d.features.len()
    );
    for &(id, _) in &d.features {
        writeln!(report, "{id:#06x}  {}", feature_name(id)).unwrap();
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_contains_the_model_and_all_reported_features() {
        let device = crate::hidpp::device::DeviceStatus {
            name: Some("Test mouse".into()),
            features: vec![(0x2201, 1), (0x9abc, 5)],
            ..Default::default()
        };
        assert_eq!(device_report(&device), "Test mouse\nHID++ features (2)\n0x2201  Adjustable DPI\n0x9abc  Unidentified feature\n");
    }
}
