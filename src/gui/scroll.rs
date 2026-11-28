//! Wheel editor: SmartShift (0x2110) + hi-res wheel (0x2121) settings.

use super::widgets::{self, Ui2};
use super::{fonts, App};
use crate::config::SmartShiftMode;
use ply_engine::prelude::*;

pub(super) fn available(device: &crate::hidpp::device::DeviceStatus) -> bool {
    device.connected && device.capabilities.has_scroll_settings()
}

/// Only the form scrolls; the wheel tabs and its shared Save button stay fixed.
pub(super) fn form(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let caps = &app.device_view.device.capabilities;
    let Some(draft) = app.wheel_draft.as_mut() else {
        return;
    };
    let side_by_side = screen_height() < 430.0
        && ui
            .bounding_box("picker_panel")
            .is_some_and(|r| r.width >= 590.0);
    ui.element()
        .id("wheel_form")
        .width(grow!())
        .height(grow!(0.0))
        .overflow(|o| {
            o.scroll_y()
                .scrollbar(|s| s.width(5.0).thumb_color(pal.track))
        })
        .layout(|l| {
            l.direction(if side_by_side {
                LeftToRight
            } else {
                TopToBottom
            })
            .gap(if side_by_side { 20 } else { 6 })
            .padding((10, 16, 12, 16))
        })
        .children(|ui| {
            if caps.smartshift {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(TopToBottom).gap(6))
                    .children(|ui| {
                        widgets::eyebrow(ui, &pal, "Wheel mode");
                        ui.element()
                            .width(grow!())
                            .height(fit!())
                            .layout(|l| l.direction(LeftToRight).gap(4))
                            .children(|ui| {
                                for (i, (mode, label)) in [
                                    (SmartShiftMode::AlwaysRatchet, "Ratchet"),
                                    (SmartShiftMode::AlwaysFreespin, "Free-spin"),
                                    (SmartShiftMode::SmartShift, "SmartShift"),
                                ]
                                .into_iter()
                                .enumerate()
                                {
                                    if choice(
                                        ui,
                                        &pal,
                                        ("mode", i as u32).into(),
                                        label,
                                        draft.smartshift == mode,
                                    ) {
                                        draft.smartshift = mode;
                                    }
                                }
                            });
                        let enabled = draft.smartshift == SmartShiftMode::SmartShift;
                        let max = caps.smartshift_max_threshold.max(1);
                        let label = format!("{} / {max}", draft.smartshift_threshold.min(max));
                        if let Some(v) = range(
                            ui,
                            &pal,
                            "flick",
                            ("Flick sensitivity", &label),
                            1..=i64::from(max),
                            draft.smartshift_threshold,
                            enabled,
                        ) {
                            draft.smartshift_threshold = v;
                        }
                        let hint = if enabled {
                            "Lower for a gentler flick."
                        } else {
                            "Select SmartShift to adjust sensitivity."
                        };
                        ui.text(hint, |t| {
                            t.font(&fonts::SANS).font_size(12).color(pal.muted)
                        });
                    });
            }
            if caps.hires_scroll || caps.invert_scroll {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(TopToBottom).gap(6))
                    .children(|ui| {
                        for (supported, id, title, value) in [
                            (
                                caps.invert_scroll,
                                "sw_invert",
                                "Invert scroll direction",
                                &mut draft.invert_scroll,
                            ),
                            (
                                caps.hires_scroll,
                                "sw_hires",
                                "High-resolution scrolling",
                                &mut draft.hires_scroll,
                            ),
                        ] {
                            if supported {
                                ui.element()
                                    .width(grow!())
                                    .height(fixed!(34.0))
                                    .layout(|l| {
                                        l.direction(LeftToRight).align(Left, CenterY).gap(8)
                                    })
                                    .children(|ui| {
                                        ui.element().width(grow!()).height(fit!()).children(|ui| {
                                            ui.text(title, |t| {
                                                t.font(&fonts::SANS_SEMIBOLD)
                                                    .font_size(14)
                                                    .color(pal.text)
                                            });
                                        });
                                        if widgets::toggle(ui, &pal, id, *value) {
                                            *value = !*value
                                        }
                                    });
                            }
                        }
                        if caps.hires_scroll {
                            let max = caps.wheel_multiplier.unwrap_or(1).max(1);
                            let value = draft.scroll_deadzone.min(max);
                            let label = if value == 0 {
                                "Off".into()
                            } else if caps.wheel_multiplier.is_some_and(|m| m > 0) {
                                format!(
                                    "{:.0}% of a notch",
                                    f32::from(value) * 100.0 / f32::from(max)
                                )
                            } else {
                                format!("{value} step")
                            };
                            if let Some(v) = range(
                                ui,
                                &pal,
                                "deadzone",
                                ("Ignore accidental nudges", &label),
                                0..=i64::from(max),
                                value,
                                draft.hires_scroll,
                            ) {
                                draft.scroll_deadzone = v;
                            }
                            let hint = if draft.hires_scroll {
                                "Ignore wheel movement below this threshold."
                            } else {
                                "Enable high-resolution scrolling to adjust."
                            };
                            ui.text(hint, |t| {
                                t.font(&fonts::SANS).font_size(12).color(pal.muted)
                            });
                        }
                    });
            }
        });
}

pub(super) fn choice(
    ui: &mut Ui2,
    pal: &super::theme::Palette,
    id: Id,
    label: &str,
    selected: bool,
) -> bool {
    let mut pressed = false;
    ui.element()
        .id(id)
        .width(grow!())
        .height(fixed!(30.0))
        .corner_radius(6.0)
        .accessibility(|a| a.button(label))
        .background_color(if selected { pal.s2 } else { pal.win })
        .border(|b| {
            b.bottom(2)
                .color(if selected { pal.accent } else { pal.line })
        })
        .layout(|l| l.align(CenterX, CenterY))
        .children(|ui| {
            pressed = ui.just_pressed();
            ui.text(label, |t| {
                t.font(&fonts::SANS_SEMIBOLD)
                    .font_size(13)
                    .color(if selected { pal.text } else { pal.muted })
            });
        });
    pressed
}

fn range(
    ui: &mut Ui2,
    pal: &super::theme::Palette,
    id: &'static str,
    (title, value_label): (&str, &str),
    bounds: std::ops::RangeInclusive<i64>,
    value: u8,
    enabled: bool,
) -> Option<u8> {
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).gap(6))
        .children(|ui| {
            ui.element().width(grow!()).height(fit!()).children(|ui| {
                ui.text(title, |t| {
                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                });
            });
            ui.text(value_label, |t| {
                t.font(&fonts::MONO_MEDIUM).font_size(12).color(if enabled {
                    pal.accent
                } else {
                    pal.muted
                })
            });
        });
    let value = i64::from(value).clamp(*bounds.start(), *bounds.end());
    widgets::slider_enabled(ui, pal, id, value, bounds, 1, enabled).map(|v| v as u8)
}

/// A card with a title, description and a toggle on the right.
pub fn switch_row(
    ui: &mut Ui2,
    app: &App,
    id: &'static str,
    title: &str,
    desc: &str,
    on: bool,
) -> bool {
    let pal = app.pal;
    let mut clicked = false;
    widgets::card(ui, &pal)
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(16)
                .padding(if screen_width() < 840.0 {
                    (8, 12, 8, 12)
                } else {
                    (15, 17, 15, 17)
                })
        })
        .children(|ui| {
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(TopToBottom).gap(1))
                .children(|ui| {
                    ui.text(title, |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                    });
                    ui.text(desc, |t| {
                        t.font(&fonts::SANS)
                            .font_size(widgets::px(12.5))
                            .color(pal.muted)
                    });
                });
            if widgets::toggle(ui, &pal, id, on) {
                clicked = true;
            }
        });
    clicked
}
