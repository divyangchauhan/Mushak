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

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    widgets::heading(
        ui,
        &pal,
        "Gestures",
        "Hold the thumb button and move the mouse to swipe, or just tap it.",
    );
    ui.element().width(grow!()).height(fixed!(22.0)).empty();

    if !app.mouse_awake() {
        widgets::asleep_card(ui, &pal);
        return;
    }

    let enabled = app.draft.gestures.enabled;
    let threshold = app.draft.gestures.tap_threshold as i64;
    let mut toggle_enabled = false;
    let mut new_threshold: Option<i64> = None;
    let mut open_picker: Option<PickerTarget> = None;

    if super::scroll::switch_row(
        ui,
        app,
        "sw_gestures",
        "Thumb-button gestures",
        "Turn the whole gesture pad on or off.",
        enabled,
    ) {
        toggle_enabled = true;
    }

    ui.element().width(grow!()).height(fixed!(10.0)).empty();

    // Everything below is inert while gestures are off; the design dims it.
    let body_text = if enabled { pal.text } else { pal.muted };

    widgets::card(ui, &pal)
        .layout(|l| l.direction(TopToBottom).gap(3).padding((16, 17, 16, 17)))
        .children(|ui| {
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                .children(|ui| {
                    ui.element().width(grow!()).height(fit!()).children(|ui| {
                        ui.text("Tap vs. swipe threshold", |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(body_text)
                        });
                    });
                    ui.text(&format!("{threshold}px"), |t| {
                        t.font(&fonts::MONO_BOLD).font_size(13).color(pal.accent)
                    });
                });
            ui.text(
                "Move less than this and it counts as a tap; more, and it's a swipe.",
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
                    ui.text("10", |t| {
                        t.font(&fonts::MONO_MEDIUM).font_size(11).color(pal.faint)
                    });
                    if enabled {
                        if let Some(v) =
                            widgets::slider(ui, &pal, "threshold", threshold, 10, 400, 5)
                        {
                            new_threshold = Some(v);
                        }
                    } else {
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
                    ui.text("400", |t| {
                        t.font(&fonts::MONO_MEDIUM).font_size(11).color(pal.faint)
                    });
                });
        });

    ui.element().width(grow!()).height(fixed!(16.0)).empty();

    let g = &app.draft.gestures;
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
        .layout(|l| l.direction(TopToBottom).gap(10))
        .children(|ui| {
            for (slot, name, disp) in &rows {
                widgets::card(ui, &pal)
                    .layout(|l| {
                        l.direction(LeftToRight)
                            .align(Left, CenterY)
                            .gap(16)
                            .padding((13, 17, 13, 17))
                    })
                    .children(|ui| {
                        widgets::icon(
                            ui,
                            row_icon(slot),
                            24.0,
                            if enabled { pal.accent } else { pal.muted },
                        );
                        ui.element()
                            .width(grow!())
                            .height(fit!())
                            .children(|ui| {
                                ui.text(name, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(body_text)
                                });
                            });
                        let id: Id = ("g_chip", slot_index(slot)).into();
                        if widgets::action_chip(ui, &pal, id, disp) && enabled {
                            open_picker = Some(PickerTarget::Gesture(slot.clone()));
                        }
                    });
            }
        });

    if toggle_enabled {
        app.draft.gestures.enabled = !enabled;
        app.commit_config();
    }
    if let Some(v) = new_threshold {
        app.draft.gestures.tap_threshold = v as i32;
        app.commit_config();
    }
    if let Some(t) = open_picker {
        app.open_picker(t);
    }
}
