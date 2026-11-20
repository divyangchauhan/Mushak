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
        if app.draft.gestures.enabled {
            "Tap the gesture button, or hold it and move the mouse to swipe."
        } else {
            "Gesture recognition is off."
        },
    );
    ui.element().width(grow!()).height(fixed!(22.0)).empty();

    if !app.mouse_awake() {
        widgets::asleep_card(ui, &pal, &app.device_view.device);
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
        "Button gestures",
        "Gesture actions apply to every profile.",
        enabled,
    ) {
        toggle_enabled = true;
    }

    if !enabled {
        if toggle_enabled {
            app.draft.gestures.enabled = true;
            app.commit_config();
        }
        ui.element().height(fixed!(16.0)).empty();
        ui.text(
            "Enable gestures to edit your saved tap and swipe actions.",
            |t| t.font(&fonts::SANS).font_size(14).color(pal.muted),
        );
        return;
    }
    ui.element().width(grow!()).height(fixed!(10.0)).empty();

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
                            t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                        });
                    });
                    ui.text(&format!("{threshold} counts"), |t| {
                        t.font(&fonts::MONO_BOLD).font_size(13).color(pal.accent)
                    });
                });
            ui.text(
                "A swipe starts when horizontal or vertical movement reaches this many sensor counts.",
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
                    if let Some(v) = widgets::slider(ui, &pal, "threshold", threshold, 10, 400, 5) {
                        new_threshold = Some(v);
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
        .layout(|l| l.direction(TopToBottom).gap(4))
        .children(|ui| {
            for (slot, name, disp) in &rows {
                widgets::card(ui, &pal)
                    .layout(|l| {
                        l.direction(LeftToRight)
                            .align(Left, CenterY)
                            .gap(16)
                            .padding((6, 16, 6, 16))
                    })
                    .children(|ui| {
                        widgets::icon(ui, row_icon(slot), 24.0, pal.accent);
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
