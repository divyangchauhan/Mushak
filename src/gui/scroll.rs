//! Scroll section: SmartShift (0x2110) + hi-res wheel (0x2121) settings.

use super::widgets::{self, Ui2};
use super::{fonts, App};
use crate::config::SmartShiftMode;
use ply_engine::prelude::*;

const MODES: [(SmartShiftMode, &str, &str); 3] = [
    (
        SmartShiftMode::AlwaysRatchet,
        "Always ratchet",
        "Clicky, stepped notches on every scroll.",
    ),
    (
        SmartShiftMode::AlwaysFreespin,
        "Always freespin",
        "Smooth and frictionless, no steps.",
    ),
    (
        SmartShiftMode::SmartShift,
        "SmartShift",
        "Ratchets normally, spins free when you flick.",
    ),
];

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    widgets::heading(
        ui,
        &pal,
        "Scroll",
        "Tune the wheel's feel. SmartShift switches between clicky and free-spinning when \
         you flick.",
    );
    ui.element().width(grow!()).height(fixed!(22.0)).empty();

    if !app.mouse_awake() {
        widgets::asleep_card(ui, &pal);
        return;
    }

    let mode = app.draft.device.smartshift;
    let flick = app.draft.device.smartshift_threshold as i64;
    let hires = app.draft.device.hires_scroll;
    let invert = app.draft.device.invert_scroll;
    let flick_on = mode == SmartShiftMode::SmartShift;

    let mut new_mode: Option<SmartShiftMode> = None;
    let mut new_flick: Option<i64> = None;
    let mut toggle_hires = false;
    let mut toggle_invert = false;

    widgets::eyebrow(ui, &pal, "Wheel mode");
    ui.element().width(grow!()).height(fixed!(10.0)).empty();

    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(9))
        .children(|ui| {
            for (i, (m, title, desc)) in MODES.into_iter().enumerate() {
                let id: Id = ("mode", i as u32).into();
                let sel = m == mode;
                ui.element()
                    .id(id)
                    .width(grow!())
                    .height(fit!())
                    .corner_radius(13.0)
                    .background_color(if sel {
                        Palette2::mix_accent(&pal)
                    } else {
                        pal.surface
                    })
                    .border(|b| b.all(1).color(if sel { pal.accent } else { pal.line }))
                    .layout(|l| {
                        l.direction(LeftToRight)
                            .align(Left, CenterY)
                            .gap(13)
                            .padding((13, 16, 13, 16))
                    })
                    .children(|ui| {
                        if ui.just_pressed() {
                            new_mode = Some(m);
                        }
                        // Radio.
                        ui.element()
                            .width(fixed!(18.0))
                            .height(fixed!(18.0))
                            .corner_radius(99.0)
                            .border(|b| {
                                b.all(2)
                                    .color(if sel { pal.accent } else { pal.line_strong })
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
                                ui.text(desc, |t| {
                                    t.font(&fonts::SANS)
                                        .font_size(widgets::px(12.5))
                                        .color(pal.muted)
                                });
                            });
                    });
            }
        });

    ui.element().width(grow!()).height(fixed!(22.0)).empty();

    // Flick sensitivity. Dimmed unless SmartShift is the active mode.
    let dim = if flick_on { pal.text } else { pal.muted };
    widgets::card(ui, &pal)
        .layout(|l| l.direction(TopToBottom).gap(3).padding((16, 17, 16, 17)))
        .children(|ui| {
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                .children(|ui| {
                    ui.element().width(grow!()).height(fit!()).children(|ui| {
                        ui.text("Flick sensitivity", |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(dim)
                        });
                    });
                    ui.text(&flick.to_string(), |t| {
                        t.font(&fonts::MONO_BOLD).font_size(13).color(pal.accent)
                    });
                });
            ui.text(
                "Lower = a gentler flick frees the wheel. Only used in SmartShift mode.",
                |t| {
                    t.font(&fonts::SANS)
                        .font_size(widgets::px(12.5))
                        .color(pal.muted)
                },
            );
            ui.element().width(grow!()).height(fixed!(12.0)).empty();
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(12))
                .children(|ui| {
                    ui.text("1", |t| {
                        t.font(&fonts::MONO_MEDIUM).font_size(11).color(pal.faint)
                    });
                    if flick_on {
                        if let Some(v) = widgets::slider(ui, &pal, "flick", flick, 1, 30, 1) {
                            new_flick = Some(v);
                        }
                    } else {
                        // Same footprint, inert.
                        ui.element()
                            .width(grow!())
                            .height(fixed!(20.0))
                            .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                            .children(|ui| {
                                ui.element()
                                    .width(grow!())
                                    .height(fixed!(4.0))
                                    .corner_radius(2.0)
                                    .background_color(pal.s2)
                                    .empty();
                            });
                    }
                    ui.text("30", |t| {
                        t.font(&fonts::MONO_MEDIUM).font_size(11).color(pal.faint)
                    });
                });
        });

    ui.element().width(grow!()).height(fixed!(18.0)).empty();

    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(10))
        .children(|ui| {
            if switch_row(
                ui,
                app,
                "sw_hires",
                "High-resolution scrolling",
                "Smooth, pixel-level scroll instead of chunky lines.",
                hires,
            ) {
                toggle_hires = true;
            }
            if switch_row(
                ui,
                app,
                "sw_invert",
                "Invert scroll direction",
                "Push the wheel up to scroll the page up.",
                invert,
            ) {
                toggle_invert = true;
            }
        });

    if let Some(m) = new_mode {
        app.draft.device.smartshift = m;
        app.commit_config();
    }
    if let Some(v) = new_flick {
        app.draft.device.smartshift_threshold = v as u8;
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
                .padding((15, 17, 15, 17))
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
