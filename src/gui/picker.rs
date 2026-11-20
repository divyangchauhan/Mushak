//! The action picker, centered for lists or anchored to a mouse marker.
//!
//! Search filters the presets. The keyboard shortcut tab records a modifier and key
//! combination. Ply has no modal primitive, so this is a floating element pinned over
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
    let (mut x, y, mut width, height) = panel_rect(anchored, screen_width(), screen_height());

    if anchored.is_some()
        && super::mouse_view::Model::for_device(&app.device_view.device)
            .is_some_and(|model| model.has_thumb_detail(app.picker_anchor))
    {
        width = (screen_width() * 0.45).clamp(350.0, 476.0);
        x = screen_width() - width - 20.0;
    }

    let height = if !app.shortcut_editor && !app.picker_query.is_empty() {
        let matches = action_groups(&app.picker_query.to_lowercase());
        let rows: usize = matches.iter().map(|(_, actions)| actions.len()).sum();
        height.min(186.0 + 25.0 * matches.len() as f32 + 42.0 * rows.max(1) as f32)
    } else {
        height
    };

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
            Color::rgba(0.0, 0.0, 0.0, 204.0)
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
                .layout(|l| l.direction(TopToBottom).padding((0, 0, 9, 0)))
                .children(|ui| {
                    header(ui, app, &target, &mut close);
                    if app.shortcut_editor {
                        shortcut_editor(ui, app, &current, &mut chosen, &mut close);
                        return;
                    }
                    if app.picker_fresh {
                        ui.set_text_value("picker_search", "");
                        ui.set_focus("picker_search");
                        app.picker_query.clear();
                        app.picker_fresh = false;
                    }
                    let query = app.picker_query.to_lowercase();
                    results(ui, app, &query, &current, &mut chosen);
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
        } else if app.picker_categories_open {
            app.picker_categories_open = false;
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
    let margin = 20.0;
    let width = if anchor.is_some() {
        (window_w * 0.385).clamp(310.0, 408.0)
    } else {
        456.0_f32
    }
    .min((window_w - 2.0 * margin).max(1.0));
    // Reserve the title bar and leave room around the editor on compact windows.
    let top = if anchor.is_some() && window_h >= 600.0 {
        144.0
    } else if window_h >= 460.0 {
        60.0
    } else {
        margin
    };
    let available = (window_h - top - margin).max(1.0);
    // Switching tabs, filtering and recording must leave the pointer over
    // the same controls. Only the viewport and presentation set the bounds.
    let preferred_height: f32 = if anchor.is_some() {
        436.0
    } else if window_h < 600.0 {
        442.0
    } else {
        504.0
    };
    let height = preferred_height.min(available);
    let x = if anchor.is_some() {
        window_w - width - margin
    } else {
        (window_w - width) / 2.0
    };
    let y = if anchor.is_some() {
        top
    } else {
        top + (available - height) / 2.0
    };
    (x.max(0.0), y, width, height)
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
                            let number = app.picker_anchor.and_then(|id| {
                                super::mouse_view::control_number(&app.device_view.device, id)
                            });
                            let number = number.or_else(|| {
                                if app.section == super::Section::Buttons {
                                    super::mouse_view::target_number(
                                        &app.device_view.device,
                                        target,
                                    )
                                } else {
                                    None
                                }
                            });
                            if let Some(n) = number {
                                ui.element()
                                    .width(grow!())
                                    .height(fit!())
                                    .layout(|l| {
                                        l.direction(LeftToRight).align(Left, CenterY).gap(9)
                                    })
                                    .children(|ui| {
                                        ui.element()
                                            .width(fixed!(23.0))
                                            .height(fixed!(23.0))
                                            .corner_radius(12.0)
                                            .background_color(pal.accent)
                                            .layout(|l| l.align(CenterX, CenterY))
                                            .children(|ui| {
                                                ui.text(&n.to_string(), |t| {
                                                    t.font(&fonts::MONO_BOLD)
                                                        .font_size(14)
                                                        .color(pal.accent_ink)
                                                });
                                            });
                                        ui.text(label, |t| {
                                            t.font(&fonts::SANS_SEMIBOLD)
                                                .font_size(19)
                                                .color(pal.text)
                                        });
                                    });
                            } else {
                                ui.text(label, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(19).color(pal.text)
                                });
                            }
                            let profile = if app.picker_anchor == Some("mouse_haptic") {
                                if matches!(target, PickerTarget::Gesture(_)) {
                                    "Tap the thumb panel"
                                } else {
                                    "Press the thumb panel"
                                }
                            } else if matches!(target, PickerTarget::Gesture(_)) {
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
                .height(fit!())
                .background_color(pal.win)
                .corner_radius(8.0)
                .layout(|l| l.direction(LeftToRight).gap(3).padding(3))
                .children(|ui| {
                    for (keyboard, id, label) in [
                        (false, "preset_tab", "Actions"),
                        (true, "shortcut_tab", "Keyboard shortcut"),
                    ] {
                        let selected = keyboard == app.shortcut_editor;
                        ui.element()
                            .id(id)
                            .width(grow!())
                            .height(fixed!(32.0))
                            .corner_radius(6.0)
                            .background_color(if selected {
                                pal.s2
                            } else {
                                super::theme::TRANSPARENT
                            })
                            .border(|b| {
                                b.bottom(2).color(if selected {
                                    pal.accent
                                } else {
                                    super::theme::TRANSPARENT
                                })
                            })
                            .accessibility(|a| a.button(label))
                            .layout(|l| l.align(CenterX, CenterY))
                            .children(|ui| {
                                if ui.just_pressed() && !selected {
                                    app.shortcut_editor = keyboard;
                                    app.picker_categories_open = false;
                                    app.capturing_key = false;
                                    app.picker_fresh = !keyboard;
                                    ui.clear_focus();
                                }
                                ui.text(label, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD)
                                        .font_size(14)
                                        .color(if selected { pal.text } else { pal.muted })
                                });
                            });
                    }
                });
            if app.shortcut_editor {
                return;
            }
            let focused = ui.focused_element() == Some("picker_search".into());
            ui.element()
                .width(grow!())
                .height(fixed!(38.0))
                .corner_radius(10.0)
                .background_color(pal.win)
                .border(|b| {
                    b.all(if focused { 2 } else { 1 }).color(if focused {
                        pal.accent
                    } else {
                        pal.line_strong
                    })
                })
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
                            if app.picker_anchor.is_some() {
                                t.placeholder_color(pal.muted);
                            }
                            t.font(&fonts::SANS)
                                .font_size(14)
                                .text_color(pal.text)
                                .cursor_color(pal.text)
                                .placeholder("Search actions…")
                        })
                        .empty();
                });
        });
    // Read whatever the input holds now.
    if app.shortcut_editor {
        return;
    }
    let live = ui.get_text_value("picker_search").to_string();
    if live != app.picker_query {
        app.picker_query = live;
        app.picker_categories_open = false;
    }
}

type ActionChoice = (
    String,
    Option<String>,
    Vec<String>,
    Action,
    &'static GraphicAsset,
);
type ActionGroup = (String, Vec<ActionChoice>);

fn category_menu(ui: &mut Ui2, app: &mut App, groups: &[ActionGroup]) -> bool {
    let pal = app.pal;
    let active = &groups[app.picker_category];
    let selector_width = ui
        .bounding_box("picker_panel")
        .map_or(168.0, |panel| (panel.width - 173.0).clamp(135.0, 180.0));
    ui.element()
        .width(grow!())
        .height(fixed!(48.0))
        .layout(|l| {
            l.direction(LeftToRight)
                .gap(10)
                .align(Left, CenterY)
                .padding((0, 16, 0, 16))
        })
        .children(|ui| {
            ui.text("Category", |t| {
                t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.muted)
            });
            ui.element()
                .id("category_menu")
                .width(fixed!(selector_width))
                .height(fixed!(33.0))
                .corner_radius(7.0)
                .border(|b| {
                    b.all(1).color(if app.picker_categories_open {
                        pal.accent
                    } else {
                        pal.line_strong
                    })
                })
                .background_color(pal.s2)
                .accessibility(|a| a.button("Choose action category"))
                .layout(|l| {
                    l.direction(LeftToRight)
                        .align(Left, CenterY)
                        .gap(6)
                        .padding((0, 10, 0, 10))
                })
                .children(|ui| {
                    if ui.just_pressed() {
                        app.picker_categories_open = !app.picker_categories_open;
                    }
                    ui.element().width(grow!()).height(fit!()).children(|ui| {
                        ui.text(&active.0, |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                        });
                    });
                    widgets::icon(ui, &icons::CHEVRON_DOWN, 14.0, pal.muted);
                });
            ui.element().width(grow!()).empty();
            if !app.picker_categories_open {
                ui.text(&format!("{} actions", active.1.len()), |t| {
                    t.font(&fonts::SANS).font_size(13).color(pal.muted)
                });
            }
        });
    if app.picker_categories_open {
        ui.element()
            .width(grow!())
            .height(grow!(0.0))
            .overflow(|o| o.scroll_y())
            .background_color(pal.win)
            .border(|b| b.top(1).color(pal.line_strong))
            .layout(|l| l.direction(TopToBottom).gap(4).padding((5, 10, 8, 10)))
            .children(|ui| {
                for (index, (name, items)) in groups.iter().enumerate() {
                    let selected = index == app.picker_category;
                    let id: Id = ("action_category", index as u32).into();
                    let hovered = ui.pointer_over(id.clone());
                    ui.element()
                        .id(id)
                        .width(grow!())
                        .height(fixed!(40.0))
                        .corner_radius(8.0)
                        .background_color(if selected {
                            super::theme::Palette::mix(pal.accent, 20.0, pal.pop)
                        } else if hovered {
                            pal.s2
                        } else {
                            super::theme::TRANSPARENT
                        })
                        .accessibility(|a| a.button(name))
                        .layout(|l| {
                            l.direction(LeftToRight)
                                .align(Left, CenterY)
                                .gap(10)
                                .padding((0, 11, 0, 11))
                        })
                        .children(|ui| {
                            if ui.just_pressed() {
                                app.picker_category = index;
                                app.picker_categories_open = false;
                            }
                            let icon = if index == 0 {
                                &icons::NAV_BUTTONS
                            } else {
                                actions::Category::ORDER[index - 1].icon()
                            };
                            widgets::icon(
                                ui,
                                icon,
                                19.0,
                                if selected { pal.accent } else { pal.text },
                            );
                            ui.element().width(grow!()).height(fit!()).children(|ui| {
                                ui.text(name, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                                });
                            });
                            ui.text(&format!("{} actions", items.len()), |t| {
                                t.font(&fonts::SANS).font_size(13).color(pal.muted)
                            });
                            widgets::icon(
                                ui,
                                if selected {
                                    &icons::CHECK
                                } else {
                                    &icons::CHEVRON_RIGHT
                                },
                                14.0,
                                if selected { pal.accent } else { pal.muted },
                            );
                        });
                }
            });
        return true;
    }
    false
}

fn common_tiles(
    ui: &mut Ui2,
    pal: super::theme::Palette,
    items: &[ActionChoice],
    current: &Action,
    chosen: &mut Option<Action>,
) {
    // Keep native behavior distinct from the keyboard shortcuts it can be replaced with.
    ui.element()
        .width(grow!())
        .height(grow!(0.0))
        .overflow(|o| o.scroll_y())
        .layout(|l| l.direction(TopToBottom).gap(7).padding((4, 12, 9, 12)))
        .children(|ui| {
            for (title, offset, section) in [
                ("Button behavior", 0, &items[..2]),
                ("Shortcuts", 2, &items[2..]),
            ] {
                if offset > 0 {
                    ui.element().height(fixed!(5.0)).empty();
                }
                ui.text(title, |t| {
                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.muted)
                });
                for (pair_index, pair) in section.chunks(2).enumerate() {
                    ui.element()
                        .width(grow!())
                        .height(fixed!(49.0))
                        .layout(|l| l.direction(LeftToRight).gap(8))
                        .children(|ui| {
                            for (i, (label, desc, keys, action, _)) in pair.iter().enumerate() {
                                let id: Id = ("pk", (offset + pair_index * 2 + i) as u32).into();
                                let selected = action == current;
                                let hover = ui.pointer_over(id.clone());
                                ui.element()
                                    .id(id)
                                    .width(grow!())
                                    .height(grow!())
                                    .corner_radius(8.0)
                                    .background_color(if selected {
                                        super::theme::Palette::mix(pal.accent, 20.0, pal.pop)
                                    } else if hover {
                                        pal.s2
                                    } else {
                                        pal.win
                                    })
                                    .border(|b| {
                                        b.all(1).color(if selected || hover {
                                            pal.accent
                                        } else {
                                            pal.line_strong
                                        })
                                    })
                                    .accessibility(|a| a.button(label))
                                    .layout(|l| {
                                        l.direction(LeftToRight)
                                            .align(Left, CenterY)
                                            .gap(5)
                                            .padding((0, 10, 0, 10))
                                    })
                                    .children(|ui| {
                                        if ui.just_pressed() {
                                            *chosen = Some(action.clone());
                                        }
                                        ui.element()
                                            .width(grow!())
                                            .height(fit!())
                                            .layout(|l| l.direction(TopToBottom).gap(3))
                                            .children(|ui| {
                                                ui.text(label, |t| {
                                                    t.font(&fonts::SANS_SEMIBOLD)
                                                        .font_size(15)
                                                        .color(pal.text)
                                                });
                                                let detail = if desc.is_some() {
                                                    if *action == Action::PassThrough {
                                                        "Normal button action".to_owned()
                                                    } else {
                                                        "Ignore presses".to_owned()
                                                    }
                                                } else {
                                                    keys.join(" + ")
                                                };
                                                ui.text(&detail, |t| {
                                                    t.font(&fonts::SANS)
                                                        .font_size(13)
                                                        .color(pal.muted)
                                                });
                                            });
                                        if selected {
                                            widgets::icon(ui, &icons::CHECK, 14.0, pal.accent);
                                        }
                                    });
                            }
                        });
                }
            }
        });
}

fn action_groups(query: &str) -> Vec<ActionGroup> {
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
            "Ignore this button press",
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

    let mut groups: Vec<ActionGroup> = Vec::new();
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

    groups
}

fn results(
    ui: &mut Ui2,
    app: &mut App,
    query: &str,
    current: &Action,
    chosen: &mut Option<Action>,
) {
    let pal = app.pal;
    let mut groups = action_groups(query);
    let empty = groups.is_empty();
    let categories = query.is_empty();
    if categories {
        // Common assignments are shortcuts into the same preset collection used by search.
        let quick = &mut groups[0];
        quick.0 = "Common".into();
        for preset in actions::presets() {
            if matches!(preset.label, "Copy" | "Paste" | "Back" | "Forward") {
                quick.1.push((
                    preset.label.into(),
                    None,
                    preset.keys.iter().map(|s| s.to_string()).collect(),
                    (preset.action)(),
                    preset.cat.icon(),
                ));
            }
        }
        if category_menu(ui, app, &groups) {
            return;
        }
    }

    if categories && app.picker_category == 0 {
        common_tiles(ui, pal, &groups[0].1, current, chosen);
        return;
    }
    let list = ui
        .element()
        .width(grow!())
        .height(grow!(0.0))
        .overflow(|o| {
            o.scroll_y().scrollbar(|s| {
                s.width(if categories { 8.0 } else { 5.0 })
                    .corner_radius(6.0)
                    .thumb_color(if categories {
                        pal.track
                    } else {
                        pal.line_strong
                    })
                    .track_color(pal.pop)
            })
        });
    list.layout(|l| {
        l.direction(TopToBottom).gap(0).padding(if categories {
            (2, 12, 2, 12)
        } else {
            (7, 16, 7, 16)
        })
    })
    .children(|ui| {
        if empty {
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| {
                    l.direction(TopToBottom)
                        .align(Left, CenterY)
                        .gap(7)
                        .padding(12)
                })
                .children(|ui| {
                    ui.text("No matching actions", |t| {
                        t.font(&fonts::SANS_BOLD).font_size(16).color(pal.text)
                    });
                    ui.text("Try another search or record a keyboard shortcut.", |t| {
                        t.font(&fonts::SANS).font_size(13).color(pal.muted)
                    });
                });
            return;
        }
        if categories {
            let category = app.picker_category;
            for (index, (label, _, keys, action, _)) in groups[category].1.iter().enumerate() {
                let id: Id = ("pk", (100 * category + index) as u32).into();
                let active = action == current;
                let hovered = ui.pointer_over(id.clone());
                ui.element()
                    .width(grow!())
                    .height(fixed!(46.0))
                    .layout(|l| l.padding((2, 0, 2, 0)))
                    .children(|ui| {
                        ui.element()
                            .id(id)
                            .width(grow!())
                            .height(grow!())
                            .accessibility(|a| a.button(label))
                            .corner_radius(6.0)
                            .background_color(if active {
                                super::theme::Palette::mix(pal.accent, 20.0, pal.pop)
                            } else {
                                pal.win
                            })
                            .border(|b| {
                                b.all(1).color(if active || hovered {
                                    pal.accent
                                } else {
                                    pal.line
                                })
                            })
                            .layout(|l| {
                                l.direction(LeftToRight)
                                    .align(Left, CenterY)
                                    .gap(5)
                                    .padding((0, 10, 0, 10))
                            })
                            .children(|ui| {
                                if ui.just_pressed() {
                                    *chosen = Some(action.clone());
                                }
                                let glyph = match label.as_str() {
                                    "Copy" => Some(&icons::EDIT_COPY),
                                    "Paste" => Some(&icons::EDIT_PASTE),
                                    "Cut" => Some(&icons::EDIT_CUT),
                                    "Undo" => Some(&icons::EDIT_UNDO),
                                    "Redo" => Some(&icons::EDIT_REDO),
                                    _ => None,
                                };
                                if let Some(glyph) = glyph {
                                    widgets::icon(ui, glyph, 15.0, pal.muted);
                                }
                                ui.element().width(grow!()).height(fit!()).children(|ui| {
                                    ui.text(label, |t| {
                                        t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                                    });
                                });
                                for key in keys {
                                    widgets::keycap(ui, &pal, key);
                                }
                                if active {
                                    widgets::icon(ui, &icons::CHECK, 14.0, pal.accent);
                                }
                            });
                    });
            }
            return;
        }
        for (gi, (header, items)) in groups.iter().enumerate() {
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.padding((8, 10, 4, 10)))
                .children(|ui| {
                    ui.text(header, |t| {
                        t.font(&fonts::SANS_BOLD).font_size(13).color(pal.faint)
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
                        pal.win
                    })
                    .border(|b| {
                        b.all(1).color(if hov || is_current {
                            pal.accent
                        } else {
                            pal.line_strong
                        })
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
                        if desc.is_some() {
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
                                        if label == "Disabled" {
                                            pal.danger
                                        } else {
                                            pal.muted
                                        },
                                    );
                                });
                        }
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
                        if !query.is_empty() && !is_current {
                            ui.text("Assign", |t| {
                                t.font(&fonts::SANS_SEMIBOLD)
                                    .font_size(13)
                                    .color(pal.accent)
                            });
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
fn shortcut_editor(
    ui: &mut Ui2,
    app: &mut App,
    current: &Action,
    chosen: &mut Option<Action>,
    close: &mut bool,
) {
    let pal = app.pal;
    let mods = app.custom_mods.clone();
    let key = app.custom_key;
    let capturing = app.capturing_key;
    let mut toggle_mod: Option<Modifier> = None;
    let mut start_capture = false;
    let mut assign = false;

    ui.element()
        .width(grow!())
        .height(grow!(0.0))
        .overflow(|o| o.scroll_y())
        .layout(|l| l.direction(TopToBottom).gap(12).padding((16, 16, 8, 16)))
        .children(|ui| {
            let assigned = actions::display(current);
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(8))
                .children(|ui| {
                    ui.text("Assigned now", |t| {
                        t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.muted)
                    });
                    if assigned.label.is_empty() {
                        for key in &assigned.keys {
                            widgets::keycap(ui, &pal, key);
                        }
                    } else {
                        ui.text(&assigned.label, |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                        });
                    }
                });
            ui.text("Modifiers", |t| {
                t.font(&fonts::SANS_BOLD).font_size(13).color(pal.faint)
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
                            .width(grow!())
                            .height(fixed!(34.0))
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
                            .layout(|l| {
                                l.direction(LeftToRight)
                                    .gap(6)
                                    .align(CenterX, CenterY)
                                    .padding((0, 4, 0, 4))
                            })
                            .children(|ui| {
                                if ui.just_pressed() {
                                    toggle_mod = Some(m);
                                }
                                if on {
                                    widgets::icon(ui, &icons::CHECK, 12.0, pal.accent_ink);
                                }
                                ui.text(label, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(if on {
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
                .height(grow!(104.0))
                .corner_radius(8.0)
                .background_color(if capturing {
                    super::theme::Palette::mix(pal.accent, 9.0, pal.win)
                } else {
                    pal.win
                })
                .border(|b| {
                    b.all(1)
                        .color(if capturing { pal.accent } else { pal.line })
                })
                .layout(|l| l.direction(TopToBottom).gap(8).align(CenterX, CenterY))
                .children(|ui| {
                    ui.text(
                        if capturing {
                            "Press a key"
                        } else {
                            "New shortcut"
                        },
                        |t| t.font(&fonts::SANS_BOLD).font_size(17).color(pal.text),
                    );
                    ui.element()
                        .width(fit!())
                        .height(fit!())
                        .layout(|l| l.direction(LeftToRight).gap(5).align(CenterX, CenterY))
                        .children(|ui| {
                            if capturing {
                                ui.text("Escape cancels recording.", |t| {
                                    t.font(&fonts::SANS).font_size(14).color(pal.muted)
                                });
                            } else if let Some(vk) = key {
                                for m in &mods {
                                    widgets::keycap(ui, &pal, &format!("{m:?}"));
                                }
                                widgets::keycap(ui, &pal, &actions::key_name(vk));
                            } else {
                                ui.text("Choose modifiers, then record the main key.", |t| {
                                    t.font(&fonts::SANS).font_size(14).color(pal.muted)
                                });
                            }
                        });
                    // Key capture button.
                    ui.element()
                        .id("capture")
                        .accessibility(|a| {
                            a.button(if capturing {
                                "Cancel key capture"
                            } else {
                                "Capture shortcut key"
                            })
                        })
                        .width(fixed!(140.0))
                        .height(fixed!(36.0))
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
                                if capturing {
                                    app.capturing_key = false;
                                } else {
                                    start_capture = true;
                                }
                            }
                            let label = if capturing {
                                "Stop recording".to_string()
                            } else {
                                key.map(|_| "Change key".to_owned())
                                    .unwrap_or_else(|| "Record key".into())
                            };
                            ui.text(&label, |t| {
                                t.font(&fonts::SANS_SEMIBOLD)
                                    .font_size(14)
                                    .color(pal.accent)
                            });
                        });
                });
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(7))
                .children(|ui| {
                    ui.element()
                        .id("cancel_shortcut")
                        .accessibility(|a| a.button("Cancel shortcut changes"))
                        .height(fixed!(32.0))
                        .corner_radius(8.0)
                        .border(|b| b.all(1).color(pal.line_strong))
                        .layout(|l| l.align(CenterX, CenterY).padding((0, 16, 0, 16)))
                        .children(|ui| {
                            if ui.just_pressed() {
                                *close = true;
                            }
                            ui.text("Cancel", |t| {
                                t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.text)
                            });
                        });
                    ui.element().width(grow!()).height(fixed!(1.0)).empty();

                    if key.is_none() || capturing {
                        ui.element()
                            .id("assign")
                            .width(fit!())
                            .height(fixed!(32.0))
                            .layout(|l| l.align(Right, CenterY))
                            .children(|ui| {
                                ui.text(
                                    if capturing {
                                        "Waiting for keypress"
                                    } else {
                                        "No key recorded"
                                    },
                                    |t| t.font(&fonts::SANS).font_size(13).color(pal.muted),
                                );
                            });
                        return;
                    }
                    ui.element()
                        .id("assign")
                        .accessibility(|a| a.button("Save shortcut"))
                        .height(fixed!(32.0))
                        .corner_radius(8.0)
                        .background_color(pal.accent)
                        .layout(|l| {
                            l.direction(LeftToRight)
                                .align(CenterX, CenterY)
                                .gap(6)
                                .padding((0, 14, 0, 14))
                        })
                        .children(|ui| {
                            if ui.just_pressed() {
                                assign = true;
                            }
                            ui.text("Save shortcut", |t| {
                                t.font(&fonts::SANS_BOLD)
                                    .font_size(widgets::px(12.5))
                                    .color(pal.accent_ink)
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
