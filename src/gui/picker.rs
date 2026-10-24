//! The action picker, centered for lists or anchored to a mouse marker.
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
    pub fn label<'a>(&self, device: &'a crate::hidpp::device::DeviceStatus) -> &'a str {
        match self {
            PickerTarget::Button(s) => match s.as_str() {
                "back" => "Back button",
                "forward" => "Forward button",
                "middle" => "Middle click",
                _ => device
                    .capabilities
                    .buttons
                    .iter()
                    .find(|button| button.slot == *s)
                    .map(|button| button.name.as_str())
                    .unwrap_or("Mouse button"),
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
    let return_focus = app.picker_anchor;
    let current = app.action_for(&target);
    let mut chosen: Option<Action> = None;
    let mut close = false;
    let anchored = app
        .picker_anchor
        .filter(|_| {
            app.section == super::Section::Buttons
                && screen_width() >= 840.0
                && super::mouse_view::matches_device(&app.device_view.device)
        })
        .and_then(|id| ui.bounding_box(id));
    let (x, y, width, height) = panel_rect(anchored, screen_width(), screen_height());

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
        .background_color(if anchored.is_some() {
            super::theme::TRANSPARENT
        } else {
            Color::rgba(0.0, 0.0, 0.0, 128.0)
        })
        .children(|ui| {
            // A press on any descendant also marks its ancestors pressed, so the
            // backdrop sees every click inside the panel too. Only a press that
            // misses the panel is a dismissal.
            if ui.just_pressed() && !ui.pointer_over("picker_panel") {
                close = true;
            }
            ui.element()
                .id("picker_panel")
                .width(fixed!(width))
                .height(fixed!(height))
                .floating(|f| f.attach_root().offset((x, y)).z_index(101))
                .corner_radius(16.0)
                .background_color(pal.pop)
                .border(|b| b.all(1).color(pal.line_strong))
                .layout(|l| l.direction(TopToBottom))
                .children(|ui| {
                    header(ui, app, &target, &mut close);
                    if app.picker_fresh {
                        ui.set_text_value("picker_search", "");
                        ui.set_focus("picker_search");
                        app.picker_query.clear();
                        app.picker_fresh = false;
                    }
                    let query = app.picker_query.to_lowercase();
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
    if app.picker.is_none() {
        ui.clear_focus();
        if let Some(id) = return_focus {
            ui.set_focus(id);
        }
    }
}

/// Keep the picker within the window, including on small displays and at high DPI.
fn panel_rect(
    anchor: Option<ply_engine::math::BoundingBox>,
    window_w: f32,
    window_h: f32,
) -> (f32, f32, f32, f32) {
    let margin = 16.0;
    let width = if anchor.is_some() {
        410.0_f32
    } else {
        500.0_f32
    }
    .min((window_w - margin * 2.0).max(1.0));
    let height = 540.0_f32.min((window_h - margin * 2.0).max(1.0));
    let max_x = (window_w - width - margin).max(margin);
    let max_y = (window_h - height - margin).max(margin);
    let (x, y) = if let Some(a) = anchor {
        let right = a.x + a.width + 14.0;
        (
            if right + width <= window_w - margin {
                right
            } else {
                a.x - width - 14.0
            },
            a.y - 60.0,
        )
    } else {
        ((window_w - width) / 2.0, 62.0)
    };
    (
        x.clamp(margin, max_x),
        y.clamp(margin, max_y),
        width,
        height,
    )
}

fn header(ui: &mut Ui2, app: &mut App, target: &PickerTarget, close: &mut bool) {
    let pal = app.pal;
    ui.element()
        .width(grow!())
        .height(fit!())
        .border(|b| b.bottom(1).color(pal.line))
        .layout(|l| l.direction(TopToBottom).gap(9).padding((13, 16, 11, 16)))
        .children(|ui| {
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(8))
                .children(|ui| {
                    ui.element()
                        .width(grow!())
                        .height(fit!())
                        .layout(|l| l.direction(TopToBottom).gap(3))
                        .children(|ui| {
                            let label = if app.picker_anchor.is_some()
                                && matches!(target, PickerTarget::Gesture(_))
                            {
                                if app.picker_anchor == Some("mouse_haptic") {
                                    "Haptic Sense Panel tap"
                                } else {
                                    "Gesture button tap"
                                }
                            } else {
                                target.label(&app.device_view.device)
                            };
                            ui.text(label, |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(15).color(pal.text)
                            });
                            let profile = if matches!(target, PickerTarget::Gesture(_)) {
                                "All profiles"
                            } else {
                                app.profile_at(app.selected_profile)
                                    .map(|p| p.name.as_str())
                                    .unwrap_or("Default")
                            };
                            ui.text(profile, |t| {
                                t.font(&fonts::SANS).font_size(12).color(pal.muted)
                            });
                        });
                    ui.element()
                        .id("picker_close")
                        .width(fixed!(28.0))
                        .height(fixed!(28.0))
                        .corner_radius(8.0)
                        .background_color(pal.s2)
                        .accessibility(|a| a.button("Close action picker"))
                        .layout(|l| l.align(CenterX, CenterY))
                        .children(|ui| {
                            if ui.just_pressed() {
                                *close = true;
                            }
                            widgets::icon(ui, &icons::CLOSE, 12.0, pal.muted);
                        });
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

fn results(ui: &mut Ui2, app: &App, query: &str, current: &Action, chosen: &mut Option<Action>) {
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

    let mut groups: Vec<(
        String,
        Vec<(
            String,
            Option<String>,
            Vec<String>,
            Action,
            &'static GraphicAsset,
        )>,
    )> = Vec::new();
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
                        .accessibility(|a| a.button(label))
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
                            .accessibility(|a| a.button(label))
                            .height(fixed!(30.0))
                            .corner_radius(8.0)
                            .background_color(if on {
                                pal.accent
                            } else {
                                super::theme::TRANSPARENT
                            })
                            .border(|b| {
                                b.all(1).color(if on {
                                    super::theme::TRANSPARENT
                                } else {
                                    pal.line_strong
                                })
                            })
                            .layout(|l| l.align(CenterX, CenterY).padding((0, 12, 0, 12)))
                            .children(|ui| {
                                if ui.just_pressed() {
                                    toggle_mod = Some(m);
                                }
                                ui.text(label, |t| {
                                    t.font(&fonts::MONO_SEMIBOLD).font_size(12).color(if on {
                                        pal.accent_ink
                                    } else {
                                        pal.muted
                                    })
                                });
                            });
                    }
                });
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(7))
                .children(|ui| {
                    // Key capture button.
                    ui.element()
                        .id("capture")
                        .accessibility(|a| a.button("Capture shortcut key"))
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
                                key.map(actions::key_name)
                                    .unwrap_or_else(|| "Set key".into())
                            };
                            ui.text(&label, |t| {
                                t.font(&fonts::MONO_BOLD).font_size(12).color(pal.accent)
                            });
                        });

                    ui.element().width(grow!()).height(fixed!(1.0)).empty();

                    let can_assign = key.is_some();
                    ui.element()
                        .id("assign")
                        .accessibility(|a| a.button("Assign shortcut"))
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
                                    .color(if can_assign {
                                        pal.accent_ink
                                    } else {
                                        pal.faint
                                    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use ply_engine::math::BoundingBox;

    #[test]
    fn anchored_picker_stays_inside_resized_windows() {
        for (w, h) in [
            (968.0, 668.0),
            (700.0, 520.0),
            (500.0, 400.0),
            (1440.0, 900.0),
        ] {
            for (ax, ay) in [(0.0, 0.0), (w - 24.0, h - 24.0), (w / 2.0, h / 2.0)] {
                let (x, y, width, height) =
                    panel_rect(Some(BoundingBox::new(ax, ay, 24.0, 24.0)), w, h);
                assert!(x >= 16.0 && y >= 16.0);
                assert!(x + width <= w - 16.0 && y + height <= h - 16.0);
            }
        }
    }
}
