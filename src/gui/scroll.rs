//! Scroll section: SmartShift (0x2110) + hi-res wheel (0x2121) settings.

use super::widgets::{self, Ui2};
use super::{fonts, App};
use crate::config::SmartShiftMode;
use ply_engine::prelude::*;

const MODES: [(SmartShiftMode, &str, &str); 3] = [
    (
        SmartShiftMode::AlwaysRatchet,
        "Always ratchet",
        "One notch at a time.",
    ),
    (
        SmartShiftMode::AlwaysFreespin,
        "Always freespin",
        "Spin without notches.",
    ),
    (
        SmartShiftMode::SmartShift,
        "SmartShift",
        "Flick to spin freely.",
    ),
];

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let compact = screen_width() < 840.0;
    widgets::heading(
        ui,
        &pal,
        "Scroll",
        "Choose how the wheel feels and when a turn counts.",
    );
    ui.element()
        .width(grow!())
        .height(fixed!(if compact { 10.0 } else { 22.0 }))
        .empty();

    if !app.mouse_awake() {
        widgets::asleep_card(ui, &pal, &app.device_view.device);
        return;
    }

    let caps = app.device_view.device.capabilities.clone();
    let mode = app.draft.device.smartshift;
    let flick = app.draft.device.smartshift_threshold as i64;
    let flick_max = i64::from(caps.smartshift_max_threshold.max(1));
    let hires = app.draft.device.hires_scroll;
    let invert = app.draft.device.invert_scroll;
    let flick_on = mode == SmartShiftMode::SmartShift;

    let mut new_mode: Option<SmartShiftMode> = None;
    let mut new_flick: Option<i64> = None;
    let mut toggle_hires = false;
    let mut toggle_invert = false;

    if caps.invert_scroll {
        toggle_invert = switch_row(
            ui,
            app,
            "sw_invert",
            "Invert scroll direction",
            "Push the wheel up to scroll the page up.",
            invert,
        );
        ui.element().width(grow!()).height(fixed!(14.0)).empty();
    }

    if caps.smartshift {
        widgets::eyebrow(ui, &pal, "Wheel mode");
        ui.element().width(grow!()).height(fixed!(10.0)).empty();

        ui.element()
            .width(grow!())
            .height(fit!())
            .layout(|l| l.direction(LeftToRight).gap(6))
            .children(|ui| {
                for (i, (m, title, desc)) in MODES.into_iter().enumerate() {
                    let id: Id = ("mode", i as u32).into();
                    let sel = m == mode;
                    ui.element()
                        .id(id)
                        .width(grow!())
                        .height(fixed!(if screen_width() >= 840.0 { 86.0 } else { 64.0 }))
                        .corner_radius(10.0)
                        .background_color(if sel {
                            Palette2::mix_accent(&pal)
                        } else {
                            pal.surface
                        })
                        .border(|b| b.all(1).color(if sel { pal.accent } else { pal.line }))
                        .layout(|l| {
                            l.direction(LeftToRight)
                                .align(Left, CenterY)
                                .gap(if screen_width() >= 840.0 { 8 } else { 5 })
                                .padding(if screen_width() >= 840.0 { 12 } else { 7 })
                        })
                        .children(|ui| {
                            if ui.just_pressed() {
                                new_mode = Some(m);
                            }
                            // Radio.
                            ui.element()
                                .width(fixed!(if compact { 14.0 } else { 18.0 }))
                                .height(fixed!(if compact { 14.0 } else { 18.0 }))
                                .corner_radius(99.0)
                                .border(|b| {
                                    b.all(2).color(if sel { pal.accent } else { pal.faint })
                                })
                                .layout(|l| l.align(CenterX, CenterY))
                                .children(|ui| {
                                    ui.element()
                                        .width(fixed!(9.0))
                                        .height(fixed!(9.0))
                                        .corner_radius(99.0)
                                        .background_color(if sel {
                                            pal.accent
                                        } else {
                                            super::theme::TRANSPARENT
                                        })
                                        .empty();
                                });
                            ui.element()
                                .width(grow!())
                                .height(fit!())
                                .layout(|l| l.direction(TopToBottom).gap(1))
                                .children(|ui| {
                                    ui.text(title, |t| {
                                        t.font(&fonts::SANS_SEMIBOLD)
                                            .font_size(widgets::px(13.5))
                                            .color(pal.text)
                                    });
                                    {
                                        let description = if compact {
                                            match m {
                                                SmartShiftMode::AlwaysRatchet => {
                                                    "One notch at a time"
                                                }
                                                SmartShiftMode::AlwaysFreespin => "No notches",
                                                SmartShiftMode::SmartShift => "Flick for free-spin",
                                            }
                                        } else {
                                            desc
                                        };
                                        ui.text(description, |t| {
                                            t.font(&fonts::SANS)
                                                .font_size(widgets::px(12.5))
                                                .color(pal.muted)
                                        });
                                    }
                                });
                        });
                }
            });

        ui.element()
            .width(grow!())
            .height(fixed!(if compact { 10.0 } else { 22.0 }))
            .empty();
    }
    if caps.smartshift {
        // Flick sensitivity applies only in SmartShift mode.
        widgets::card(ui, &pal)
            .layout(|l| {
                l.direction(TopToBottom).gap(3).padding(if compact {
                    (8, 12, 8, 12)
                } else {
                    (16, 17, 16, 17)
                })
            })
            .children(|ui| {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                    .children(|ui| {
                        ui.element().width(grow!()).height(fit!()).children(|ui| {
                            ui.text("SmartShift flick sensitivity", |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                            });
                        });
                        ui.text(
                            &format!("Level {} of {flick_max}", flick.min(flick_max)),
                            |t| {
                                t.font(&fonts::MONO_BOLD).font_size(15).color(if flick_on {
                                    pal.accent
                                } else {
                                    pal.muted
                                })
                            },
                        );
                    });
                ui.text(
                    if flick_on {
                        "Lower for a gentler flick. Higher for a firmer flick."
                    } else {
                        "Select SmartShift to adjust the saved sensitivity."
                    },
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
                        ui.text("1", |t| {
                            t.font(&fonts::MONO_MEDIUM).font_size(12).color(pal.muted)
                        });

                        if let Some(v) = widgets::slider_enabled(
                            ui,
                            &pal,
                            "flick",
                            flick.min(flick_max),
                            1..=flick_max,
                            1,
                            flick_on,
                        ) {
                            new_flick = Some(v);
                        }
                        ui.text(&flick_max.to_string(), |t| {
                            t.font(&fonts::MONO_MEDIUM).font_size(12).color(pal.muted)
                        });
                    });
            });

        ui.element().width(grow!()).height(fixed!(14.0)).empty();
    }
    let deadzone = i64::from(app.draft.device.scroll_deadzone);
    let threshold = if deadzone == 0 {
        "off".to_owned()
    } else if let Some(m) = caps.wheel_multiplier.filter(|m| *m > 0) {
        format!("{:.0}% of a notch", deadzone as f32 * 100.0 / f32::from(m))
    } else {
        format!("{deadzone} steps")
    };
    if caps.hires_scroll {
        let detail = if hires {
            "Use finer steps for each wheel turn.".to_owned()
        } else {
            "Use standard wheel steps.".to_owned()
        };
        toggle_hires = switch_row(
            ui,
            app,
            "sw_hires",
            "High-resolution scrolling",
            &detail,
            hires,
        );
        ui.element().width(grow!()).height(fixed!(10.0)).empty();
    }
    // Wheel deadzone. Only meaningful with hi-res on: that is the only mode
    // where the wheel reports to us rather than straight to Windows, so it is
    // the only mode we can filter.
    let multiplier = caps.wheel_multiplier.unwrap_or(1) as i64;
    let mut new_deadzone: Option<i64> = None;
    if caps.hires_scroll {
        widgets::card(ui, &pal)
            .layout(|l| {
                l.direction(TopToBottom).gap(3).padding(if compact {
                    (8, 12, 8, 12)
                } else {
                    (16, 17, 16, 17)
                })
            })
            .children(|ui| {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                    .children(|ui| {
                        ui.element().width(grow!()).height(fit!()).children(|ui| {
                            ui.text("Ignore accidental nudges", |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                            });
                        });
                        ui.text(&threshold, |t| {
                            t.font(&fonts::MONO_BOLD).font_size(15).color(if hires {
                                pal.accent
                            } else {
                                pal.muted
                            })
                        });
                    });
                let hint = if !hires {
                    "Turn on high-resolution scrolling to adjust the saved threshold."
                } else if deadzone == 0 {
                    "Every movement scrolls."
                } else {
                    "After a pause or reversal, turn the wheel this far to start scrolling."
                };
                ui.text(hint, |t| {
                    t.font(&fonts::SANS)
                        .font_size(widgets::px(12.5))
                        .color(pal.muted)
                });
                ui.element()
                    .width(grow!())
                    .height(fixed!(if compact { 6.0 } else { 12.0 }))
                    .empty();
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(12))
                    .children(|ui| {
                        ui.text("off", |t| {
                            t.font(&fonts::MONO_MEDIUM).font_size(12).color(pal.muted)
                        });
                        if let Some(v) = widgets::slider_enabled(
                            ui,
                            &pal,
                            "deadzone",
                            deadzone.min(multiplier),
                            0..=multiplier,
                            1,
                            hires,
                        ) {
                            new_deadzone = Some(v);
                        }
                        ui.text(
                            if caps.wheel_multiplier.is_some_and(|m| m > 0) {
                                "100%"
                            } else {
                                "1 step"
                            },
                            |t| t.font(&fonts::MONO_MEDIUM).font_size(12).color(pal.muted),
                        );
                    });
            });

        ui.element()
            .width(grow!())
            .height(fixed!(if compact { 14.0 } else { 18.0 }))
            .empty();
    }
    if let Some(m) = new_mode {
        app.draft.device.smartshift = m;
        app.commit_config();
    }
    if let Some(v) = new_flick {
        app.draft.device.smartshift_threshold = v as u8;
        app.commit_config();
    }
    if let Some(v) = new_deadzone {
        app.draft.device.scroll_deadzone = v as u8;
        app.commit_config();
    }
    if toggle_hires {
        app.draft.device.hires_scroll = !hires;
        app.commit_config();
    }
    if toggle_invert {
        app.draft.device.invert_scroll = !invert;
        app.commit_config();
    }
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

/// The design tints a selected radio card with 12% accent over the surface.
struct Palette2;
impl Palette2 {
    fn mix_accent(pal: &super::theme::Palette) -> Color {
        super::theme::Palette::mix(pal.accent, 12.0, pal.surface)
    }
}
