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
        0x1814 => "Change host",
        0x1b04 => "Reprogrammable controls v4",
        0x1982 => "Backlight",
        0x2100 => "Vertical scrolling",
        0x2110 => "SmartShift",
        0x2121 => "Hi-res wheel",
        0x2201 => "Adjustable DPI",
        0x2205 => "Pointer motion",
        _ => "Unknown",
    }
}

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let mut refresh = false;

    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).align(Left, Top).gap(16))
        .children(|ui| {
            ui.element().width(grow!()).height(fit!()).children(|ui| {
                widgets::heading(ui, &pal, "Device", "Live status straight from the mouse.");
            });
            let hovered = ui.pointer_over("btn_reapply");
            ui.element()
                .id("btn_reapply")
                .width(fit!())
                .height(fixed!(38.0))
                .corner_radius(10.0)
                .background_color(pal.surface)
                .border(|b| b.all(1).color(if hovered { pal.accent } else { pal.line_strong }))
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
                    ui.text("Re-apply", |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                    });
                });
        });

    ui.element().width(grow!()).height(fixed!(22.0)).empty();

    if !app.mouse_awake() {
        widgets::asleep_card(ui, &pal);
        return;
    }

    let d = app.device_view.device.clone();
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
                widgets::icon(ui, &icons::BLUETOOTH, 16.0, pal.accent);
                ui.text(d.connection.as_deref().unwrap_or("—"), |t| {
                    t.font(&fonts::SANS_SEMIBOLD).font_size(15).color(pal.text)
                });
            });
            stat_tile(ui, app, "Firmware", |ui| {
                ui.text(d.firmware.as_deref().unwrap_or("—"), |t| {
                    t.font(&fonts::MONO_SEMIBOLD).font_size(15).color(pal.text)
                });
            });
        });
    ui.element().width(grow!()).height(fixed!(10.0)).empty();
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).gap(10))
        .children(|ui| {
            stat_tile(ui, app, "Battery", |ui| {
                ui.element()
                    .width(grow!())
                    .height(fixed!(8.0))
                    .corner_radius(99.0)
                    .background_color(pal.s2)
                    .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                    .children(|ui| {
                        let frac = battery.unwrap_or(0) as f32 / 100.0;
                        ui.element()
                            .width(ply_engine::layout::Sizing::Percent(frac.max(0.01)))
                            .height(fixed!(8.0))
                            .corner_radius(99.0)
                            .background_color(batt_color)
                            .empty();
                    });
                ui.text(
                    &battery.map(|p| format!("{p}%")).unwrap_or("—".into()),
                    |t| t.font(&fonts::MONO_BOLD).font_size(15).color(pal.text),
                );
                if d.charging {
                    ui.text("charging", |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(11).color(pal.good)
                    });
                }
            });
            stat_tile(ui, app, "Current DPI", |ui| {
                ui.text(
                    &d.dpi_current.map(|v| v.to_string()).unwrap_or("—".into()),
                    |t| t.font(&fonts::MONO_BOLD).font_size(15).color(pal.accent),
                );
            });
        });

    ui.element().width(grow!()).height(fixed!(10.0)).empty();

    // DPI slider, bounded by the device's reported list.
    let (min, max) = match (d.dpi_list.first(), d.dpi_list.last()) {
        (Some(&f), Some(&l)) if l > f => (f as i64, l as i64),
        _ => (200, 4000),
    };
    let dpi = (app.draft.device.dpi as i64).clamp(min, max);
    let mut new_dpi: Option<i64> = None;

    widgets::card(ui, &pal)
        .layout(|l| l.direction(TopToBottom).gap(12).padding((16, 17, 16, 17)))
        .children(|ui| {
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                .children(|ui| {
                    ui.element().width(grow!()).height(fit!()).children(|ui| {
                        ui.text("Pointer speed (DPI)", |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                        });
                    });
                    ui.text(&dpi.to_string(), |t| {
                        t.font(&fonts::MONO_BOLD).font_size(13).color(pal.accent)
                    });
                });
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(12))
                .children(|ui| {
                    ui.text(&min.to_string(), |t| {
                        t.font(&fonts::MONO_MEDIUM).font_size(11).color(pal.faint)
                    });
                    if let Some(v) = widgets::slider(ui, &pal, "dpi", dpi, min, max, 50) {
                        new_dpi = Some(v);
                    }
                    ui.text(&max.to_string(), |t| {
                        t.font(&fonts::MONO_MEDIUM).font_size(11).color(pal.faint)
                    });
                });
        });

    ui.element().width(grow!()).height(fixed!(10.0)).empty();

    // HID++ feature list (diagnostic).
    let features = d.features.clone();
    let expanded = app.hidpp_expanded;
    let mut toggle = false;
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
                    widgets::icon(
                        ui,
                        if expanded {
                            &icons::CHEVRON_DOWN
                        } else {
                            &icons::CHEVRON_RIGHT
                        },
                        14.0,
                        pal.muted,
                    );
                    ui.element().width(grow!()).height(fit!()).children(|ui| {
                        ui.text("HID++ features", |t| {
                            t.font(&fonts::SANS_SEMIBOLD)
                                .font_size(widgets::px(13.5))
                                .color(pal.text)
                        });
                    });
                    ui.text(&format!("{} · diagnostic", features.len()), |t| {
                        t.font(&fonts::MONO_MEDIUM)
                            .font_size(widgets::px(11.5))
                            .color(pal.faint)
                    });
                });
            if expanded {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(TopToBottom).padding((2, 8, 10, 8)))
                    .children(|ui| {
                        for (id, _idx) in &features {
                            ui.element()
                                .width(grow!())
                                .height(fixed!(32.0))
                                .corner_radius(8.0)
                                .layout(|l| {
                                    l.direction(LeftToRight)
                                        .align(Left, CenterY)
                                        .gap(12)
                                        .padding((0, 10, 0, 10))
                                })
                                .children(|ui| {
                                    ui.element()
                                        .width(fixed!(64.0))
                                        .height(fit!())
                                        .children(|ui| {
                                            ui.text(&format!("{id:#06x}"), |t| {
                                                t.font(&fonts::MONO_MEDIUM)
                                                    .font_size(12)
                                                    .color(pal.accent)
                                            });
                                        });
                                    ui.element().width(grow!()).height(fit!()).children(|ui| {
                                        ui.text(feature_name(*id), |t| {
                                            t.font(&fonts::SANS)
                                                .font_size(widgets::px(12.5))
                                                .color(pal.text)
                                        });
                                    });
                                    ui.element()
                                        .width(fixed!(6.0))
                                        .height(fixed!(6.0))
                                        .corner_radius(99.0)
                                        .background_color(pal.good)
                                        .empty();
                                });
                        }
                    });
            }
        });

    if toggle {
        app.hidpp_expanded = !expanded;
    }
    if let Some(v) = new_dpi {
        app.draft.device.dpi = v as u16;
        app.commit_config();
    }
    if refresh {
        app.device_view = crate::statusfile::read();
        app.commit_config();
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
            ui.text(&label.to_uppercase(), |t| {
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
