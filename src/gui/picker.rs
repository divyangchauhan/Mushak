//! The action picker: a command palette over the whole window.
//!
//! Search filters the presets, and the footer builds a custom modifier+key
//! combo. Ply has no modal primitive, so this is a floating element pinned over
//! the window with a dimmed backdrop behind it.

use super::widgets::{self, Ui2};
use super::{actions, fonts, icons, App, PickerTarget};
use crate::config::{Action, Modifier};
use ply_engine::layout::Sizing;
use ply_engine::prelude::*;

impl PickerTarget {
    pub fn label(&self) -> &'static str {
        match self {
            PickerTarget::Button(s) => match s.as_str() {
                "back" => "Back button",
                "forward" => "Forward button",
                _ => "Middle click",
            },
            PickerTarget::Gesture(s) => match s.as_str() {
                "tap" => "Tap",
                "up" => "Swipe up",
                "down" => "Swipe down",
                "left" => "Swipe left",
                _ => "Swipe right",
            },
        }
    }
}

/// Draw the palette. Called last so it floats over the sections.
pub fn overlay(ui: &mut Ui2, app: &mut App) {
    let Some(target) = app.picker.clone() else {
        return;
    };
    let pal = app.pal;
    let query = app.picker_query.to_lowercase();
    let current = app.action_for(&target);
    let mut chosen: Option<Action> = None;
    let mut close = false;

    // Backdrop. Clicking it dismisses.
    //
    // Attached to the *root*, not the parent: a parent-attached float is placed
    // relative to wherever it sits in the tree, which pins it below the content
    // row instead of covering the window.
    ui.element()
        .id("picker_backdrop")
        .width(Sizing::Fixed(screen_width()))
        .height(Sizing::Fixed(screen_height()))
        .floating(|f| f.attach_root().offset((0.0, 0.0)).z_index(100))
        .background_color(Color::rgba(0.0, 0.0, 0.0, 128.0))
        .layout(|l| l.direction(TopToBottom).align(CenterX, Top).padding((62, 0, 0, 0)))
        .children(|ui| {
            // A press on any descendant also marks its ancestors pressed, so the
            // backdrop sees every click inside the panel too. Only a press that
            // misses the panel is a dismissal.
            if ui.just_pressed() && !ui.pointer_over("picker_panel") {
                close = true;
            }
            ui.element()
                .id("picker_panel")
                .width(fixed!(500.0))
                .height(fit!(0.0, 540.0))
                .corner_radius(16.0)
                .background_color(pal.pop)
                .border(|b| b.all(1).color(pal.line_strong))
                .layout(|l| l.direction(TopToBottom))
                .children(|ui| {
                    header(ui, app, &target);
                    results(ui, app, &query, &current, &mut chosen);
                    custom_footer(ui, app, &mut chosen);
                });
        });

    if let Some(a) = chosen {
        app.assign_action(&target, a);
        app.close_picker();
    } else if is_key_pressed(KeyCode::Escape) {
        // While capturing, Escape backs out of the capture rather than
        // discarding the whole palette.
        if app.capturing_key {
            app.capturing_key = false;
        } else {
            app.close_picker();
        }
    } else if close {
        app.close_picker();
    }
}

fn header(ui: &mut Ui2, app: &mut App, target: &PickerTarget) {
    let pal = app.pal;
    ui.element()
        .width(grow!())
        .height(fit!())
        .border(|b| b.bottom(1).color(pal.line))
        .layout(|l| l.direction(TopToBottom).gap(9).padding((13, 16, 11, 16)))
        .children(|ui| {
            ui.text(&format!("ACTION FOR {}", target.label().to_uppercase()), |t| {
                t.font(&fonts::SANS_SEMIBOLD)
                    .font_size(widgets::px(11.5))
                    .color(pal.faint)
            });
            ui.element()
                .width(grow!())
                .height(fixed!(38.0))
                .corner_radius(10.0)
                .background_color(pal.win)
                .border(|b| b.all(1).color(pal.line_strong))
                .layout(|l| {
                    l.direction(LeftToRight)
                        .align(Left, CenterY)
                        .gap(10)
                        .padding((0, 12, 0, 12))
                })
                .children(|ui| {
                    widgets::icon(ui, &icons::SEARCH, 16.0, pal.faint);
                    ui.element()
                        .id("picker_search")
                        .width(grow!())
                        .height(fixed!(24.0))
                        .text_input(|t| {
                            t.font(&fonts::SANS)
                                .font_size(14)
                                .text_color(pal.text)
                                .placeholder("Search actions…")
                        })
                        .empty();
                });
        });
    // Read whatever the input holds now.
    let live = ui.get_text_value("picker_search").to_string();
    if live != app.picker_query {
        app.picker_query = live;
    }
}

fn results(
    ui: &mut Ui2,
    app: &App,
    query: &str,
    current: &Action,
    chosen: &mut Option<Action>,
) {
    let pal = app.pal;
    let presets = actions::presets();

    // "Quick" group: pass-through and disabled.
    let quick: Vec<(&str, &str, Action, &'static GraphicAsset, &str)> = vec![
        (
            "Pass through",
            "Keep the button's normal behavior",
            Action::PassThrough,
            &icons::PASSTHROUGH,
            "pass through leave normal default",
        ),
        (
            "Disabled",
            "Swallow the press, do nothing",
            Action::Disabled,
            &icons::DISABLED,
            "disabled off nothing block",
        ),
    ];
    let quick: Vec<_> = quick
        .into_iter()
        .filter(|(l, _, _, _, s)| {
            query.is_empty() || l.to_lowercase().contains(query) || s.contains(query)
        })
        .collect();

    let mut groups: Vec<(String, Vec<(String, Option<String>, Vec<String>, Action, &'static GraphicAsset)>)> =
        Vec::new();
    if !quick.is_empty() {
        groups.push((
            "Quick".into(),
            quick
                .into_iter()
                .map(|(l, d, a, i, _)| (l.to_string(), Some(d.to_string()), vec![], a, i))
                .collect(),
        ));
    }
    for cat in actions::Category::ORDER {
        let hits: Vec<_> = presets
            .iter()
            .filter(|p| p.cat == cat)
            .filter(|p| {
                query.is_empty()
                    || p.label.to_lowercase().contains(query)
                    || p.keys.join(" ").to_lowercase().contains(query)
                    || cat.label().to_lowercase().contains(query)
            })
            .map(|p| {
                (
                    p.label.to_string(),
                    None,
                    p.keys.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                    (p.action)(),
                    cat.icon(),
                )
            })
            .collect();
        if !hits.is_empty() {
            groups.push((cat.label().to_string(), hits));
        }
    }

    let empty = groups.is_empty();
    ui.element()
        .width(grow!())
        .height(grow!(0.0, 360.0))
        .overflow(|o| {
            o.scroll_y().scrollbar(|s| {
                s.width(11.0)
                    .corner_radius(6.0)
                    .thumb_color(pal.line_strong)
                    .track_color(pal.pop)
            })
        })
        .layout(|l| l.direction(TopToBottom).padding(7))
        .children(|ui| {
            if empty {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.align(CenterX, CenterY).padding(22))
                    .children(|ui| {
                        ui.text(
                            &format!("No preset matches \u{201C}{query}\u{201D}. Build a custom shortcut below."),
                            |t| t.font(&fonts::SANS).font_size(13).color(pal.muted),
                        );
                    });
                return;
            }
            for (gi, (header, items)) in groups.iter().enumerate() {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.padding((8, 10, 4, 10)))
                    .children(|ui| {
                        ui.text(&header.to_uppercase(), |t| {
                            t.font(&fonts::SANS_BOLD)
                                .font_size(widgets::px(10.5))
                                .color(pal.faint)
                        });
                    });
                for (n, (label, desc, keys, action, glyph)) in items.iter().enumerate() {
                    let id: Id = ("pk", (gi * 100 + n) as u32).into();
                    let hov = ui.pointer_over(id.clone());
                    let is_current = action == current;
                    ui.element()
                        .id(id)
                        .width(grow!())
                        .height(fit!())
                        .corner_radius(9.0)
                        .background_color(if is_current {
                            super::theme::Palette::mix(pal.accent, 12.0, pal.pop)
                        } else if hov {
                            pal.s2
                        } else {
                            super::theme::TRANSPARENT
                        })
                        .layout(|l| {
                            l.direction(LeftToRight)
                                .align(Left, CenterY)
                                .gap(11)
                                .padding((9, 10, 9, 10))
                        })
                        .children(|ui| {
                            if ui.just_pressed() {
                                *chosen = Some(action.clone());
                            }
                            ui.element()
                                .width(fixed!(30.0))
                                .height(fixed!(30.0))
                                .corner_radius(8.0)
                                .background_color(pal.s2)
                                .layout(|l| l.align(CenterX, CenterY))
                                .children(|ui| {
                                    widgets::icon(
                                        ui,
                                        glyph,
                                        16.0,
                                        if label == "Disabled" { pal.danger } else { pal.muted },
                                    );
                                });
                            ui.element()
                                .width(grow!())
                                .height(fit!())
                                .layout(|l| l.direction(TopToBottom).gap(1))
                                .children(|ui| {
                                    ui.text(label, |t| {
                                        t.font(&fonts::SANS_SEMIBOLD)
                                            .font_size(widgets::px(13.5))
                                            .color(pal.text)
                                    });
                                    if let Some(d) = desc {
                                        ui.text(d, |t| {
                                            t.font(&fonts::SANS).font_size(12).color(pal.muted)
                                        });
                                    }
                                });
                            for k in keys {
                                widgets::keycap(ui, &pal, k);
                            }
                            if is_current {
                                widgets::icon(ui, &icons::CHECK, 15.0, pal.accent);
                            }
                        });
                }
            }
        });
}

/// Modifier chips + key capture + Assign.
fn custom_footer(ui: &mut Ui2, app: &mut App, chosen: &mut Option<Action>) {
    let pal = app.pal;
    let mods = app.custom_mods.clone();
    let key = app.custom_key;
    let capturing = app.capturing_key;
    let mut toggle_mod: Option<Modifier> = None;
    let mut start_capture = false;
    let mut assign = false;

    ui.element()
        .width(grow!())
        .height(fit!())
        .background_color(pal.win)
        .border(|b| b.top(1).color(pal.line))
        .layout(|l| l.direction(TopToBottom).gap(9).padding((13, 16, 13, 16)))
        .children(|ui| {
            ui.text("CUSTOM SHORTCUT", |t| {
                t.font(&fonts::SANS_BOLD)
                    .font_size(widgets::px(10.5))
                    .color(pal.faint)
            });
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(7))
                .children(|ui| {
                    for (mi, (m, label)) in [
                        (Modifier::Ctrl, "Ctrl"),
                        (Modifier::Win, "Win"),
                        (Modifier::Alt, "Alt"),
                        (Modifier::Shift, "Shift"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let on = mods.contains(&m);
                        let id: Id = ("mod", mi as u32).into();
                        ui.element()
                            .id(id)
                            .height(fixed!(30.0))
                            .corner_radius(8.0)
                            .background_color(if on { pal.accent } else { super::theme::TRANSPARENT })
                            .border(|b| {
                                b.all(1)
                                    .color(if on { super::theme::TRANSPARENT } else { pal.line_strong })
                            })
                            .layout(|l| l.align(CenterX, CenterY).padding((0, 12, 0, 12)))
                            .children(|ui| {
                                if ui.just_pressed() {
                                    toggle_mod = Some(m);
                                }
                                ui.text(label, |t| {
                                    t.font(&fonts::MONO_SEMIBOLD)
                                        .font_size(12)
                                        .color(if on { pal.accent_ink } else { pal.muted })
                                });
                            });
                    }
                    ui.text("+", |t| t.font(&fonts::SANS).font_size(13).color(pal.faint));

                    // Key capture button.
                    ui.element()
                        .id("capture")
                        .width(fit!(78.0))
                        .height(fixed!(30.0))
                        .corner_radius(8.0)
                        .background_color(if capturing {
                            super::theme::Palette::mix(pal.accent, 16.0, pal.win)
                        } else {
                            super::theme::TRANSPARENT
                        })
                        .border(|b| b.all(1).color(pal.accent))
                        .layout(|l| l.align(CenterX, CenterY).padding((0, 14, 0, 14)))
                        .children(|ui| {
                            if ui.just_pressed() {
                                start_capture = true;
                            }
                            let label = if capturing {
                                "Press a key…".to_string()
                            } else {
                                key.map(actions::key_name).unwrap_or_else(|| "Set key".into())
                            };
                            ui.text(&label, |t| {
                                t.font(&fonts::MONO_BOLD).font_size(12).color(pal.accent)
                            });
                        });

                    ui.element().width(grow!()).height(fixed!(1.0)).empty();

                    let can_assign = key.is_some();
                    ui.element()
                        .id("assign")
                        .height(fixed!(32.0))
                        .corner_radius(8.0)
                        .background_color(if can_assign { pal.accent } else { pal.s2 })
                        .layout(|l| l.align(CenterX, CenterY).padding((0, 16, 0, 16)))
                        .children(|ui| {
                            if ui.just_pressed() && can_assign {
                                assign = true;
                            }
                            ui.text("Assign", |t| {
                                t.font(&fonts::SANS_BOLD)
                                    .font_size(widgets::px(12.5))
                                    .color(if can_assign { pal.accent_ink } else { pal.faint })
                            });
                        });
                });
        });

    if let Some(m) = toggle_mod {
        if app.custom_mods.contains(&m) {
            app.custom_mods.retain(|x| x != &m);
        } else {
            app.custom_mods.push(m);
        }
    }
    if start_capture {
        app.capturing_key = true;
    }
    if assign {
        if let Some(k) = key {
            *chosen = Some(Action::key(&app.custom_mods, k));
        }
    }
}
