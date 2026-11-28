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

const SELECTION_FEEDBACK_SECONDS: f32 = 0.12;

pub(super) fn has_mouse_actions(app: &App, target: &PickerTarget) -> bool {
    let device = &app.device_view.device;
    matches!(target, PickerTarget::Button(slot) if slot == "control_00c4")
        && device.connected
        && (device.capabilities.smartshift
            || (device.capabilities.dpi && device.dpi_list.len() > 1))
}

impl PickerTarget {
    pub fn label<'a>(&self, device: &'a crate::hidpp::device::DeviceStatus) -> &'a str {
        match self {
            PickerTarget::Button(s) => match s.as_str() {
                "back" => "Back button",
                "forward" => "Forward button",
                "middle" => "Middle click",
                "control_00c4" => "Wheel mode button",
                "control_00ed" => "DPI change button",
                "control_00fd" => "DPI switch button",
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

fn wheel_changed(app: &App, current: &Action) -> bool {
    app.wheel_draft.as_ref().is_some_and(|w| {
        w != &app.draft.device
            || if app.shortcut_editor {
                pending_shortcut(app, current).is_some()
            } else {
                app.pending_action.is_some()
            }
    })
}

/// Draw the palette. Called last so it floats over the sections.
pub fn overlay(ui: &mut Ui2, app: &mut App) {
    let Some(target) = app.picker.clone() else {
        return;
    };
    let pal = app.pal;
    let return_focus = app.picker_anchor;
    let current = app.action_for(&target);
    if !wheel_changed(app, &current) {
        if let Some((next, anchor)) = app.wheel_switch.take() {
            app.open_button_popover(next, anchor);
            return;
        }
    }
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
        && (matches!(&target, PickerTarget::Button(s) if s == "control_00c4")
            || super::mouse_view::Model::for_device(&app.device_view.device)
                .is_some_and(|model| model.has_detail(app.picker_anchor)))
    {
        width = (screen_width() * 0.45).clamp(350.0, 476.0);
        x = screen_width() - width - 20.0;
    }

    let has_mouse_actions = has_mouse_actions(app, &target);
    // Filtering and category changes keep the same viewport. Long lists scroll.
    let height = if app.naming_button {
        height.min(212.0)
    } else if app.failed_assignment.is_some() {
        height.min(244.0)
    } else if app.picker_row_label.is_some() {
        height.min(478.0)
    } else if has_mouse_actions {
        height.min(482.0)
    } else {
        height.min(462.0)
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
            if ui.just_pressed()
                && !ui.pointer_over("picker_panel")
                && !wheel_changed(app, &current)
            {
                close = true;
            }
            if anchored.is_some() && app.wheel_draft.is_some() {
                if let Some(next) = super::mouse_view::wheel_marker_input(ui, app) {
                    app.wheel_switch = Some(next);
                    close = false;
                }
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
                    if app.naming_button {
                        name_button(ui, app, &target);
                        return;
                    }
                    if let Some((pending, recovery)) = app.failed_assignment.clone() {
                        if retry_assignment(ui, app, &pending, recovery) {
                            chosen = Some(pending);
                        }
                        return;
                    }
                    if app.scrolling_tab {
                        super::scroll::form(ui, app);
                        let action = if app.shortcut_editor {
                            pending_shortcut(app, &current)
                        } else {
                            app.pending_action.clone()
                        };
                        let state = if action.is_some() {
                            SaveState::Ready
                        } else {
                            SaveState::Empty
                        };
                        if assignment_footer(ui, app, EditorTab::Scrolling, state, &mut close) {
                            chosen = Some(action.unwrap_or_else(|| current.clone()));
                        }
                        return;
                    }
                    if app.shortcut_editor {
                        shortcut_editor(ui, app, &target, &current, &mut chosen, &mut close);
                        return;
                    }
                    if app.picker_fresh {
                        ui.set_text_value("picker_search", &app.picker_query);
                        ui.set_focus("picker_search");
                        app.picker_fresh = false;
                    }
                    let query = app.picker_query.to_lowercase();
                    let browse = has_mouse_actions && query.is_empty();

                    let mut selection = None;
                    if browse && !app.wheel_presets {
                        ui.element()
                            .width(grow!())
                            .height(grow!(0.0))
                            .overflow(|o| {
                                o.scroll_y()
                                    .scrollbar(|s| s.width(4.0).thumb_color(pal.track))
                            })
                            .children(|ui| device_actions(ui, app, &query, &mut selection));
                    } else {
                        if !query.is_empty() || !has_mouse_actions {
                            device_actions(ui, app, &query, &mut selection);
                        }
                        results(ui, app, &query, &current, &mut selection);
                    }
                    if app.picker_speed_editor && selection.is_none() {
                        app.pending_action = None;
                    }
                    if let Some(action) = selection {
                        app.pending_action = (action != current).then_some(action);
                    }
                    let device = &app.device_view.device;
                    let available = match &app.pending_action {
                        Some(Action::ShiftWheelMode) => {
                            device.connected && device.capabilities.smartshift
                        }
                        Some(Action::ChangePointerSpeed { first, second }) => {
                            device.connected
                                && device.capabilities.dpi
                                && first != second
                                && device.dpi_list.contains(first)
                                && device.dpi_list.contains(second)
                        }
                        _ => true,
                    };
                    if !available {
                        app.pending_action = None;
                    }
                    let state = if app.picker_feedback.is_some() {
                        SaveState::Saved
                    } else if app.pending_action.is_some() {
                        SaveState::Ready
                    } else {
                        SaveState::Empty
                    };
                    if assignment_footer(ui, app, EditorTab::Actions, state, &mut close) {
                        chosen = Some(
                            app.pending_action
                                .clone()
                                .unwrap_or_else(|| current.clone()),
                        );
                    }
                });
        });

    if let Some(started) = app.picker_feedback {
        if started.elapsed().as_secs_f32() >= SELECTION_FEEDBACK_SECONDS {
            app.picker_feedback = None;
            app.close_picker();
            miniquad::window::schedule_update();
        }
    } else if let Some(a) = chosen {
        let display = actions::display_for(app, &target, &a);
        app.assign_action(&target, a);
        if app.failed_assignment.is_none() {
            let assignment = if display.label.is_empty() {
                display.keys.join(" + ")
            } else {
                display.label
            };
            app.saved_notice = Some((
                if app.wheel_draft.is_some() {
                    "Scroll wheel settings saved".to_owned()
                } else {
                    format!(
                        "{} saved for {}",
                        assignment,
                        target.label(&app.device_view.device)
                    )
                },
                app.section,
                app.selected_profile,
                std::time::Instant::now(),
            ));
            app.picker_feedback = Some(std::time::Instant::now());
        }
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
    let margin = if window_h < 430.0 { 16.0 } else { 20.0 };
    let width = if anchor.is_some() {
        (window_w * 0.385).clamp(310.0, 408.0)
    } else if window_h < 430.0 && window_w >= 640.0 {
        680.0
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
        504.0
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

fn name_button(ui: &mut Ui2, app: &mut App, target: &PickerTarget) {
    let PickerTarget::Button(slot) = target else {
        return;
    };
    let pal = app.pal;
    let key = format!("{}/{}", app.device_view.device.display_name(), slot);
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(10).padding((12, 16, 12, 16)))
        .children(|ui| {
            ui.text("Button name for this mouse model", |t| {
                t.font(&fonts::SANS_SEMIBOLD).font_size(15).color(pal.text)
            });
            ui.element()
                .width(grow!())
                .height(fixed!(35.0))
                .corner_radius(6.0)
                .background_color(pal.win)
                .border(|b| b.all(1).color(pal.accent))
                .layout(|l| l.align(Left, CenterY).padding((0, 10, 0, 10)))
                .children(|ui| {
                    ui.element()
                        .id("button_name_input")
                        .width(grow!())
                        .height(fixed!(26.0))
                        .text_input(|t| {
                            t.font(&fonts::SANS)
                                .font_size(15)
                                .text_color(pal.text)
                                .cursor_color(pal.accent)
                        })
                        .empty();
                });
            let name = ui.get_text_value("button_name_input").trim().to_owned();
            let valid = name.chars().count() <= 24 && !name.chars().any(char::is_control);
            let hint = if app.name_save_failed {
                "Could not save the name. Try again."
            } else if !valid {
                "Use up to 24 printable characters."
            } else {
                "Leave blank to restore the reported name."
            };
            ui.text(hint, |t| {
                t.font(&fonts::SANS).font_size(13).color(pal.muted)
            });
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).gap(8))
                .children(|ui| {
                    for (save, id, caption) in [
                        (true, "save_button_name", "Save name"),
                        (false, "cancel_button_name", "Cancel"),
                    ] {
                        ui.element()
                            .id(id)
                            .width(fit!())
                            .height(fixed!(34.0))
                            .corner_radius(7.0)
                            .accessibility(|a| a.button(caption))
                            .background_color(if save && valid { pal.accent } else { pal.s2 })
                            .layout(|l| l.align(CenterX, CenterY).padding((0, 13, 0, 13)))
                            .children(|ui| {
                                ui.text(caption, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(
                                        if save && valid {
                                            pal.accent_ink
                                        } else {
                                            pal.text
                                        },
                                    )
                                });
                                if !ui.just_pressed() {
                                    return;
                                }
                                if save {
                                    if !valid {
                                        return;
                                    }
                                    let mut prefs = app.prefs.clone();
                                    if name.is_empty() {
                                        prefs.button_names.remove(&key);
                                    } else {
                                        prefs.button_names.insert(key.clone(), name.clone());
                                    }
                                    if prefs.try_save().is_err() {
                                        app.name_save_failed = true;
                                        return;
                                    }
                                    app.prefs = prefs;
                                }
                                app.naming_button = false;
                                app.name_save_failed = false;
                                ui.clear_focus();
                            });
                    }
                });
        });
}

fn pending_shortcut(app: &App, current: &Action) -> Option<Action> {
    let draft = Action::key(&app.custom_mods, app.custom_key?);
    (draft != *current).then_some(draft)
}

fn draft_replacement_hint(app: &App, current: &Action) -> Option<String> {
    let keys = actions::display(&pending_shortcut(app, current)?)
        .keys
        .join(" + ");
    Some(if keys.chars().count() <= 8 {
        format!("Saving an action replaces draft {keys}")
    } else {
        "Saving an action replaces the draft".into()
    })
}

fn wheel_header(ui: &mut Ui2, app: &mut App, target: &PickerTarget, close: &mut bool) {
    let pal = app.pal;
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(8))
        .children(|ui| {
            widgets::icon(ui, &icons::NAV_SCROLL, 19.0, pal.accent);
            ui.element().width(grow!()).height(fit!()).children(|ui| {
                ui.text("Scroll wheel", |t| {
                    t.font(&fonts::SANS_BOLD).font_size(19).color(pal.text)
                });
            });
            ui.element()
                .id("picker_close")
                .width(fixed!(28.0))
                .height(fixed!(28.0))
                .corner_radius(8.0)
                .background_color(pal.s2)
                .accessibility(|a| a.button("Close wheel editor"))
                .layout(|l| l.align(CenterX, CenterY))
                .children(|ui| {
                    if ui.just_pressed() {
                        *close = true
                    }
                    widgets::icon(ui, &icons::CLOSE, 12.0, pal.muted);
                });
        });
    if app.failed_assignment.is_some() {
        return;
    }
    if target.available(&app.device_view.device) {
        ui.element()
            .width(grow!())
            .height(fit!())
            .layout(|l| l.direction(LeftToRight).gap(4))
            .children(|ui| {
                for (scroll, id, label) in [
                    (true, "scrolling_tab", "Scrolling"),
                    (false, "middle_click_tab", "Middle click"),
                ] {
                    if super::scroll::choice(
                        ui,
                        &pal,
                        id.into(),
                        label,
                        app.scrolling_tab == scroll,
                    ) && app.scrolling_tab != scroll
                    {
                        app.scrolling_tab = scroll;
                        app.capturing_key = false;
                        app.picker_fresh = true;
                        ui.clear_focus();
                    }
                }
            });
    }
    let scope = if app.scrolling_tab {
        "Applies to all profiles".to_owned()
    } else {
        format!(
            "Middle click for {}",
            app.profile_at(app.selected_profile)
                .map_or("Default", |p| p.name.as_str())
        )
    };
    ui.text(&scope, |t| {
        t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.muted)
    });
}

fn assignment_heading(ui: &mut Ui2, app: &mut App, target: &PickerTarget, close: &mut bool) {
    let pal = app.pal;
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
                        app.picker_row_label
                            .as_deref()
                            .unwrap_or_else(|| target.label(&app.device_view.device))
                    };
                    let alias_key = match target {
                        PickerTarget::Button(slot) => {
                            format!("{}/{}", app.device_view.device.display_name(), slot)
                        }
                        _ => String::new(),
                    };
                    let label = app
                        .prefs
                        .button_names
                        .get(&alias_key)
                        .map(String::as_str)
                        .unwrap_or(label);
                    let number = app.picker_anchor.and_then(|id| {
                        super::mouse_view::control_number(&app.device_view.device, id)
                    });
                    let number = number.or_else(|| {
                        if app.section == super::Section::Buttons {
                            super::mouse_view::target_number(&app.device_view.device, target)
                        } else {
                            None
                        }
                    });
                    if let Some(n) = number {
                        ui.element()
                            .width(grow!())
                            .height(fit!())
                            .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(9))
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
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(19).color(pal.text)
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
                    } else {
                        app.profile_at(app.selected_profile)
                            .map(|p| p.name.as_str())
                            .unwrap_or("Default")
                    };
                    let control_id = match target {
                        PickerTarget::Button(slot) if app.picker_anchor.is_none() => {
                            slot.strip_prefix("control_")
                        }
                        _ => None,
                    };
                    let pending = app.pending_action.as_ref().filter(|_| !app.shortcut_editor);
                    let subtitle = if let Some(action) = pending {
                        let display = actions::display_for(app, target, action);
                        let name = if display.label.is_empty() {
                            display.keys.join(" + ")
                        } else {
                            display.label
                        };
                        format!("{profile}  /  Unsaved: {name}")
                    } else if let Some(id) = control_id {
                        format!("{profile}  /  Control 0x{}", id.to_uppercase())
                    } else if app.shortcut_editor {
                        profile.to_owned()
                    } else {
                        let display = actions::assignment(app, target);
                        let assigned = if display.label.is_empty() {
                            display.keys.join(" + ")
                        } else {
                            display.label
                        };
                        format!("{profile}  /  Assigned: {assigned}")
                    };
                    let subtitle = if app.device_view.paused {
                        format!("{subtitle}  /  Remapping paused")
                    } else {
                        subtitle
                    };
                    ui.text(&subtitle, |t| {
                        t.font(&fonts::SANS_MEDIUM)
                            .font_size(if screen_height() < 430.0 { 12 } else { 14 })
                            .color(pal.text)
                    });
                    if let PickerTarget::Button(slot) = target {
                        let device = &app.device_view.device;
                        let unplaced = slot.starts_with("control_")
                            && app.picker_anchor.is_none()
                            && super::mouse_view::matches_device(device)
                            && super::mouse_view::target_number(device, target).is_none();
                        if unplaced && !app.naming_button {
                            let task = device
                                .capabilities
                                .buttons
                                .iter()
                                .find(|button| &button.slot == slot)
                                .and_then(|button| button.default_task());
                            let detail = task.map_or_else(
                                || {
                                    "The device report has no location or factory action."
                                        .to_owned()
                                },
                                |task| format!("Factory action: {task}. Position not reported."),
                            );
                            ui.text(&detail, |t| {
                                t.font(&fonts::SANS).font_size(13).color(pal.muted)
                            });
                            ui.element()
                                .id("name_button")
                                .height(fixed!(26.0))
                                .corner_radius(5.0)
                                .background_color(pal.s2)
                                .accessibility(|a| a.button("Name this button"))
                                .layout(|l| l.align(Left, CenterY).padding((0, 8, 0, 8)))
                                .children(|ui| {
                                    ui.text("Name this button", |t| {
                                        t.font(&fonts::SANS_SEMIBOLD)
                                            .font_size(13)
                                            .color(pal.accent)
                                    });
                                    if ui.just_pressed() {
                                        app.naming_button = true;
                                        let name = app
                                            .prefs
                                            .button_names
                                            .get(&alias_key)
                                            .map(String::as_str)
                                            .unwrap_or("");
                                        ui.set_text_value("button_name_input", name);
                                        ui.set_focus("button_name_input");
                                    }
                                });
                        }
                    }
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
}

fn header(ui: &mut Ui2, app: &mut App, target: &PickerTarget, close: &mut bool) {
    let pal = app.pal;
    ui.element()
        .width(grow!())
        .height(fit!())
        .border(|b| b.bottom(1).color(pal.line))
        .layout(|l| {
            if screen_height() < 430.0 {
                l.direction(TopToBottom).gap(4).padding((6, 16, 5, 16))
            } else {
                l.direction(TopToBottom).gap(9).padding((13, 16, 11, 16))
            }
        })
        .children(|ui| {
            if app.wheel_draft.is_some() {
                wheel_header(ui, app, target, close);
            } else {
                assignment_heading(ui, app, target, close);
            }
            if app.failed_assignment.is_some() || app.naming_button || app.scrolling_tab {
                return;
            }
            ui.element()
                .width(grow!())
                .height(fit!())
                .background_color(pal.win)
                .corner_radius(8.0)
                .layout(|l| l.direction(LeftToRight).gap(3).padding(3))
                .children(|ui| {
                    let draft = !app.shortcut_editor
                        && pending_shortcut(app, &app.action_for(target)).is_some();
                    for (keyboard, id, label) in [
                        (false, "preset_tab", "Actions"),
                        (
                            true,
                            "shortcut_tab",
                            if draft {
                                "Keyboard shortcut · Draft"
                            } else {
                                "Keyboard shortcut"
                            },
                        ),
                    ] {
                        let nested = app.wheel_draft.is_some();
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
                                if nested {
                                    return b.all(1).color(if selected {
                                        pal.line_strong
                                    } else {
                                        super::theme::TRANSPARENT
                                    });
                                }
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
                                    app.failed_assignment = None;
                                    app.shortcut_editor = keyboard;
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
            if app.shortcut_editor || app.failed_assignment.is_some() {
                return;
            }
            if has_mouse_actions(app, target) && app.picker_query.is_empty() {
                wheel_action_tabs(ui, app);
                if app.picker_speed_editor {
                    return;
                }
            }
            let focused = ui.focused_element() == Some("picker_search".into());
            ui.element()
                .width(grow!())
                .height(fixed!(38.0))
                .corner_radius(10.0)
                .background_color(pal.win)
                .border(|b| {
                    b.all(1)
                        .color(if focused { pal.accent } else { pal.line_strong })
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
                    if !app.picker_query.is_empty() {
                        ui.element()
                            .id("clear_search")
                            .width(fixed!(28.0))
                            .height(fixed!(28.0))
                            .accessibility(|a| a.button("Clear search"))
                            .layout(|l| l.align(CenterX, CenterY))
                            .children(|ui| {
                                widgets::icon(ui, &icons::CLOSE, 14.0, pal.text);
                                if ui.just_pressed() {
                                    ui.set_text_value("picker_search", "");
                                    ui.set_focus("picker_search");
                                }
                            });
                    }
                });
        });
    if app.shortcut_editor
        || app.scrolling_tab
        || app.failed_assignment.is_some()
        || app.naming_button
        || app.picker_speed_editor
        || app.picker_fresh
    {
        return;
    }
    let live = ui.get_text_value("picker_search").to_string();
    if live != app.picker_query {
        app.picker_query = live;
        app.picker_speed_editor = false;
    }
}

fn wheel_action_tabs(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    ui.element()
        .width(grow!())
        .height(fixed!(35.0))
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(6)
                .padding((4, 16, 0, 16))
        })
        .children(|ui| {
            for (id, label, presets) in [
                (
                    "mouse_actions_back",
                    if app.picker_speed_editor {
                        "Back to mouse actions"
                    } else {
                        "Mouse actions"
                    },
                    false,
                ),
                ("browse_wheel_actions", "All actions", true),
            ] {
                if presets && app.picker_speed_editor {
                    continue;
                }
                let active = presets == app.wheel_presets && !app.picker_speed_editor;
                ui.element()
                    .id(id)
                    .height(fixed!(28.0))
                    .width(fixed!(if presets { 106.0 } else { 152.0 }))
                    .corner_radius(6.0)
                    .background_color(if active {
                        pal.s2
                    } else {
                        super::theme::TRANSPARENT
                    })
                    .border(|b| {
                        b.all(1)
                            .color(if active { pal.line_strong } else { pal.line })
                    })
                    .accessibility(|a| a.button(label))
                    .layout(|l| l.align(CenterX, CenterY))
                    .children(|ui| {
                        ui.text(label, |t| {
                            t.font(&fonts::SANS_SEMIBOLD)
                                .font_size(13)
                                .color(if active { pal.text } else { pal.muted })
                        });
                        if ui.just_pressed() {
                            app.wheel_presets = presets;
                            app.picker_speed_editor = false;
                        }
                    });
            }
        });
}

fn device_actions(ui: &mut Ui2, app: &mut App, query: &str, chosen: &mut Option<Action>) {
    if !matches!(&app.picker, Some(PickerTarget::Button(slot)) if slot == "control_00c4") {
        return;
    }
    let d = &app.device_view.device;
    if !d.connected {
        app.picker_speed_editor = false;
        let pal = app.pal;
        ui.element()
            .width(grow!())
            .height(fit!())
            .layout(|l| l.direction(TopToBottom).gap(6).padding((8, 16, 8, 16)))
            .children(|ui| {
                for (label, search) in [
                    (
                        "Toggle ratchet and free-spin",
                        "switch shift wheel mode ratchet free spin",
                    ),
                    ("Switch pointer speed", "switch change pointer speed dpi"),
                ] {
                    if !query.is_empty() && !search.contains(query) {
                        continue;
                    }
                    ui.element()
                        .width(grow!())
                        .height(fit!())
                        .corner_radius(6.0)
                        .background_color(pal.s2)
                        .layout(|l| l.direction(TopToBottom).gap(4).padding((8, 10, 8, 10)))
                        .children(|ui| {
                            ui.text(label, |t| {
                                t.font(&fonts::SANS).font_size(13).color(pal.muted)
                            });
                            ui.text(
                                "Reconnect the mouse to check availability and configure.",
                                |t| t.font(&fonts::SANS).font_size(11).color(pal.muted),
                            );
                        });
                }
            });
        return;
    }
    let assigned = app.picker.as_ref().map(|target| app.action_for(target));
    let wheel_selected =
        app.pending_action.as_ref().or(assigned.as_ref()) == Some(&Action::ShiftWheelMode);
    let editing = app.picker_speed_editor;
    let side_by_side = screen_height() < 430.0 && screen_width() >= 640.0;
    let wheel = !editing
        && d.capabilities.smartshift
        && (query.is_empty() || "shift wheel mode ratchet free spin".contains(query));
    let dpi = d.capabilities.dpi
        && d.dpi_list.len() > 1
        && (editing || query.is_empty() || "change pointer speed dpi".contains(query));
    if !wheel && !dpi {
        return;
    }
    let pal = app.pal;
    let current_dpi = d.dpi_current;
    let choices = d.dpi_list.clone();
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(6).padding((8, 16, 8, 16)))
        .children(|ui| {
            if editing {
                let steps = format!("{} supported DPI values", choices.len());
                if !side_by_side {
                    ui.text("Switch between two pointer speeds", |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(15).color(pal.text)
                    });
                }
                ui.text(&steps, |t| {
                    t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.muted)
                });
            }
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(TopToBottom).gap(4))
                .children(|ui| {
                    if wheel
                        && hardware_row(
                            ui,
                            &pal,
                            "picker_wheel_action",
                            "Toggle ratchet and free-spin",
                            "Mushak changes mode on each press",
                            &icons::NAV_SCROLL,
                            wheel_selected,
                            if wheel_selected {
                                app.picker_feedback
                            } else {
                                None
                            },
                        )
                    {
                        *chosen = Some(Action::ShiftWheelMode);
                    }
                    if dpi
                        && !editing
                        && hardware_row(
                            ui,
                            &pal,
                            "picker_speed_setup",
                            "Switch pointer speed",
                            "Alternate two DPI values",
                            &icons::POINTER_SPEED,
                            false,
                            None,
                        )
                    {
                        app.picker_speed_editor = true;
                        for (i, value) in app.picker_dpi.iter().enumerate() {
                            let nearest = choices
                                .iter()
                                .min_by_key(|choice| choice.abs_diff(*value))
                                .unwrap();
                            ui.set_text_value(("dpi_value", i as u32), &nearest.to_string());
                        }
                    }
                });
            if dpi && editing {
                let mut valid = true;
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| {
                        l.direction(if side_by_side {
                            LeftToRight
                        } else {
                            TopToBottom
                        })
                        .gap(if side_by_side { 18 } else { 6 })
                    })
                    .children(|ui| {
                        for (i, slider_id) in
                            ["picker_dpi_first", "picker_dpi_second"].iter().enumerate()
                        {
                            let field: Id = ("dpi_value", i as u32).into();
                            let selected = choices
                                .iter()
                                .position(|v| *v == app.picker_dpi[i])
                                .unwrap_or(0);
                            ui.element()
                                .width(grow!())
                                .height(fit!())
                                .layout(|l| l.direction(TopToBottom).gap(3))
                                .children(|ui| {
                                    ui.element()
                                        .width(grow!())
                                        .height(fixed!(32.0))
                                        .layout(|l| {
                                            l.direction(LeftToRight).align(Left, CenterY).gap(8)
                                        })
                                        .children(|ui| {
                                            ui.text(&format!("Speed {}", i + 1), |t| {
                                                t.font(&fonts::SANS_SEMIBOLD)
                                                    .font_size(14)
                                                    .color(pal.text)
                                            });
                                            let focused =
                                                ui.focused_element() == Some(field.clone());
                                            ui.element()
                                                .width(fixed!(64.0))
                                                .height(fixed!(30.0))
                                                .corner_radius(5.0)
                                                .background_color(pal.win)
                                                .border(|b| {
                                                    b.all(1).color(if focused {
                                                        pal.accent
                                                    } else {
                                                        pal.line_strong
                                                    })
                                                })
                                                .layout(|l| {
                                                    l.align(Left, CenterY).padding((0, 6, 0, 6))
                                                })
                                                .children(|ui| {
                                                    ui.element()
                                                        .id(field.clone())
                                                        .width(grow!())
                                                        .height(fixed!(24.0))
                                                        .text_input(|t| {
                                                            t.font(&fonts::MONO_MEDIUM)
                                                                .font_size(14)
                                                                .text_color(pal.text)
                                                                .cursor_color(pal.accent)
                                                        })
                                                        .empty();
                                                });
                                            ui.text("DPI", |t| {
                                                t.font(&fonts::SANS).font_size(13).color(pal.muted)
                                            });
                                        });
                                    if let Some(v) = widgets::slider(
                                        ui,
                                        &pal,
                                        *slider_id,
                                        selected as i64,
                                        0,
                                        (choices.len() - 1) as i64,
                                        1,
                                    ) {
                                        ui.set_text_value(
                                            field.clone(),
                                            &choices[v as usize].to_string(),
                                        );
                                    }
                                    if choices.len() <= 6 {
                                        ui.element()
                                            .width(grow!())
                                            .height(fit!())
                                            .layout(|l| {
                                                l.direction(LeftToRight).align(Left, CenterY)
                                            })
                                            .children(|ui| {
                                                for (j, step) in choices.iter().enumerate() {
                                                    if j > 0 {
                                                        ui.element()
                                                            .width(grow!())
                                                            .height(fixed!(1.0))
                                                            .empty();
                                                    }
                                                    ui.text(&step.to_string(), |t| {
                                                        t.font(&fonts::MONO_MEDIUM)
                                                            .font_size(11)
                                                            .color(if *step == app.picker_dpi[i] {
                                                                pal.text
                                                            } else {
                                                                pal.muted
                                                            })
                                                    });
                                                }
                                            });
                                    }
                                    let value = ui
                                        .get_text_value(field)
                                        .trim()
                                        .parse::<u16>()
                                        .ok()
                                        .filter(|v| choices.contains(v));
                                    if let Some(v) = value {
                                        app.picker_dpi[i] = v;
                                    } else {
                                        valid = false;
                                    }
                                });
                        }
                    });
                let distinct = valid && app.picker_dpi[0] != app.picker_dpi[1];
                ui.element()
                    .id("dpi_outlook")
                    .width(grow!())
                    .height(fixed!(30.0))
                    .corner_radius(6.0)
                    .background_color(pal.s2)
                    .layout(|l| {
                        l.direction(LeftToRight)
                            .align(Left, CenterY)
                            .gap(8)
                            .padding((0, 8, 0, 8))
                    })
                    .children(|ui| {
                        if let Some(current) = current_dpi.filter(|_| distinct) {
                            let Ok(next) = crate::hidpp::features::dpi::toggle_target(
                                &choices,
                                current,
                                app.picker_dpi[0],
                                app.picker_dpi[1],
                            ) else {
                                return;
                            };
                            ui.text(&format!("Current  {current} DPI"), |t| {
                                t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.muted)
                            });
                            widgets::icon(ui, &icons::CHEVRON_RIGHT, 13.0, pal.muted);
                            ui.text(&format!("Next press  {next} DPI"), |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                            });
                        } else if !distinct {
                            ui.text(
                                if valid {
                                    "Choose two different speeds"
                                } else {
                                    "Enter a DPI value reported by this mouse"
                                },
                                |t| {
                                    t.font(&fonts::SANS_SEMIBOLD)
                                        .font_size(13)
                                        .color(pal.danger)
                                },
                            );
                        } else {
                            ui.text("Each press switches between these speeds", |t| {
                                t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.muted)
                            });
                        }
                    });
                if distinct {
                    *chosen = Some(Action::ChangePointerSpeed {
                        first: app.picker_dpi[0],
                        second: app.picker_dpi[1],
                    });
                }
            }
        });
}

/// Match the preset rows, including their icon tile, hover fill and selected check.
fn hardware_row(
    ui: &mut Ui2,
    pal: &super::theme::Palette,
    id: &'static str,
    label: &str,
    description: &str,
    glyph: &'static GraphicAsset,
    selected: bool,
    feedback: Option<std::time::Instant>,
) -> bool {
    let hover = ui.pointer_over(id);
    let pressed = ui.is_pressed(id);
    let progress =
        feedback.map(|t| (t.elapsed().as_secs_f32() / SELECTION_FEEDBACK_SECONDS).clamp(0.0, 1.0));
    let pulse = progress.map(|t| (1.0 - t) * (1.0 - t)).unwrap_or(0.0);
    let mut clicked = false;
    ui.element()
        .id(id)
        .accessibility(|a| a.button(label))
        .width(grow!())
        .height(fixed!(58.0))
        .corner_radius(9.0)
        .background_color(if selected || feedback.is_some() {
            super::theme::Palette::mix(pal.accent, 12.0 + 24.0 * pulse, pal.pop)
        } else if hover || pressed {
            pal.s2
        } else {
            pal.win
        })
        .border(|b| {
            b.all(1).color(if hover || selected {
                pal.accent
            } else {
                pal.line_strong
            })
        })
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(6)
                .padding((0, 10, 0, 10))
        })
        .children(|ui| {
            clicked = ui.just_pressed();
            ui.element()
                .width(fixed!(20.0))
                .height(fixed!(30.0))
                .corner_radius(8.0)
                .background_color(pal.s2)
                .layout(|l| l.align(CenterX, CenterY))
                .children(|ui| {
                    widgets::icon(
                        ui,
                        glyph,
                        if pressed { 14.0 } else { 16.0 - 2.0 * pulse },
                        pal.muted,
                    );
                });
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(TopToBottom).gap(2))
                .children(|ui| {
                    ui.text(label, |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                    });
                    ui.text(description, |t| {
                        t.font(&fonts::SANS).font_size(13).color(pal.muted)
                    });
                });
            if !selected {
                let direct = id == "picker_wheel_action";
                ui.element()
                    .width(fixed!(96.0))
                    .height(fixed!(28.0))
                    .corner_radius(6.0)
                    .background_color(pal.s2)
                    .border(|b| b.all(1).color(pal.line_strong))
                    .layout(|l| l.direction(LeftToRight).gap(4).align(CenterX, CenterY))
                    .children(|ui| {
                        ui.text(if direct { "Select" } else { "Set speeds" }, |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                        });
                        if !direct {
                            widgets::icon(ui, &icons::CHEVRON_RIGHT, 12.0, pal.muted);
                        }
                    });
            }
            if selected || feedback.is_some() {
                widgets::icon(ui, &icons::CHECK, 15.0 - 3.0 * pulse, pal.accent);
            }
        });
    clicked
}

type ActionChoice = (
    String,
    Option<String>,
    Vec<String>,
    Action,
    &'static GraphicAsset,
);
type ActionGroup = (String, Vec<ActionChoice>);

fn shortcut_hint(ui: &mut Ui2, pal: &super::theme::Palette, keys: &[String]) {
    if !keys.is_empty() {
        ui.text(&keys.join(" + "), |t| {
            t.font(&fonts::MONO_BOLD).font_size(12).color(pal.muted)
        });
    }
}

fn browse_categories(
    ui: &mut Ui2,
    app: &mut App,
    groups: &[ActionGroup],
    current: &Action,
    chosen: &mut Option<Action>,
) {
    let pal = app.pal;
    let wide_short = screen_height() < 430.0
        && ui
            .bounding_box("picker_panel")
            .is_some_and(|r| r.width >= 600.0);
    let columns = if wide_short { 2 } else { 1 };
    let wheel = app.wheel_draft.is_some();
    let rail_width = if wheel { 124.0 } else { 136.0 };
    ui.element()
        .width(grow!())
        .height(grow!(0.0))
        .layout(|l| l.direction(LeftToRight).padding((8, 9, 0, 9)).gap(8))
        .children(|ui| {
            ui.element()
                .width(fixed!(rail_width))
                .height(grow!(0.0))
                .overflow(|o| {
                    o.scroll_y()
                        .scrollbar(|bar| bar.width(4.0).thumb_color(pal.track))
                })
                .border(|b| b.right(1).color(pal.line_strong))
                .layout(|l| {
                    l.direction(TopToBottom)
                        .gap(if wide_short { 1 } else { 3 })
                        .padding((3, 7, 4, 2))
                })
                .children(|ui| {
                    if !wide_short && !wheel {
                        widgets::eyebrow(ui, &pal, "Categories");
                    }
                    for (i, (name, _)) in groups.iter().enumerate() {
                        let active = i == app.picker_category;
                        let id: Id = ("action_category", i as u32).into();
                        let hover = ui.pointer_over(id.clone());
                        ui.element()
                            .id(id)
                            .width(grow!())
                            .height(fixed!(if wide_short {
                                22.0
                            } else if wheel {
                                25.0
                            } else {
                                32.0
                            }))
                            .corner_radius(6.0)
                            .accessibility(|a| a.button(name))
                            .border(|b| {
                                b.left(3).color(if active {
                                    pal.accent
                                } else {
                                    super::theme::TRANSPARENT
                                })
                            })
                            .background_color(if active {
                                super::theme::Palette::mix(pal.accent, 15.0, pal.pop)
                            } else if hover {
                                pal.s2
                            } else {
                                super::theme::TRANSPARENT
                            })
                            .layout(|l| {
                                l.direction(LeftToRight)
                                    .gap(6)
                                    .align(Left, CenterY)
                                    .padding((0, 6, 0, 6))
                            })
                            .children(|ui| {
                                ui.text(name, |t| {
                                    t.font(if active {
                                        &fonts::SANS_SEMIBOLD
                                    } else {
                                        &fonts::SANS
                                    })
                                    .font_size(14)
                                    .color(if active {
                                        pal.text
                                    } else {
                                        pal.muted
                                    })
                                });
                                if ui.just_pressed() {
                                    app.picker_category = i;
                                }
                            });
                    }
                });
            let choices = &groups[app.picker_category].1;
            ui.element()
                .id(("category_results", app.picker_category as u32))
                .width(grow!())
                .height(grow!(0.0))
                .overflow(|o| {
                    o.scroll_y()
                        .scrollbar(|bar| bar.width(5.0).thumb_color(pal.track))
                })
                .layout(|l| l.direction(TopToBottom).gap(3).padding((3, 9, 4, 0)))
                .children(|ui| {
                    if !wheel || draft_replacement_hint(app, current).is_some() {
                        ui.element()
                            .height(fixed!(24.0))
                            .layout(|l| l.align(Left, CenterY))
                            .children(|ui| {
                                let hint = draft_replacement_hint(app, current);
                                ui.text(
                                    hint.as_deref().unwrap_or(&groups[app.picker_category].0),
                                    |t| t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text),
                                );
                            });
                    }
                    for (line, pair) in choices.chunks(columns).enumerate() {
                        ui.element()
                            .width(grow!())
                            .height(fit!())
                            .layout(|l| l.direction(LeftToRight).gap(6))
                            .children(|ui| {
                                for (column, (label, _, keys, action, _)) in pair.iter().enumerate()
                                {
                                    let i = line * columns + column;
                                    let factory = app
                                        .picker
                                        .as_ref()
                                        .filter(|_| *action == Action::PassThrough)
                                        .map(|target| {
                                            actions::display_for(app, target, action).label
                                        });
                                    let label = factory.as_deref().unwrap_or(label);
                                    let id: Id =
                                        ("pk", (100 * app.picker_category + i) as u32).into();
                                    let active =
                                        action == app.pending_action.as_ref().unwrap_or(current);
                                    let hover = ui.pointer_over(id.clone());
                                    ui.element()
                                        .id(id)
                                        .width(grow!())
                                        .height(fixed!(if wide_short {
                                            32.0
                                        } else if wheel {
                                            27.0
                                        } else if screen_width() < 840.0 {
                                            42.0
                                        } else {
                                            31.0
                                        }))
                                        .corner_radius(6.0)
                                        .background_color(if active || hover {
                                            pal.s2
                                        } else {
                                            super::theme::TRANSPARENT
                                        })
                                        .border(|b| {
                                            b.bottom(1).color(if hover {
                                                pal.accent
                                            } else {
                                                pal.line
                                            })
                                        })
                                        .accessibility(|a| a.button(label))
                                        .layout(|l| {
                                            l.direction(LeftToRight)
                                                .align(Left, CenterY)
                                                .gap(4)
                                                .padding((0, 12, 0, 10))
                                        })
                                        .children(|ui| {
                                            ui.element().width(grow!()).height(fit!()).children(
                                                |ui| {
                                                    ui.text(label, |t| {
                                                        t.font(&fonts::SANS_SEMIBOLD)
                                                            .font_size(14)
                                                            .color(pal.text)
                                                    });
                                                },
                                            );
                                            shortcut_hint(ui, &pal, keys);
                                            if active && app.pending_action.is_some() {
                                                ui.text("Draft", |t| {
                                                    t.font(&fonts::SANS_SEMIBOLD)
                                                        .font_size(12)
                                                        .color(pal.accent)
                                                });
                                            } else if active {
                                                widgets::icon(ui, &icons::CHECK, 14.0, pal.accent);
                                            } else if hover && !wide_short {
                                                ui.text("Select", |t| {
                                                    t.font(&fonts::SANS_SEMIBOLD)
                                                        .font_size(13)
                                                        .color(pal.accent)
                                                });
                                            }
                                            if ui.just_pressed() {
                                                *chosen = Some(action.clone());
                                            }
                                        });
                                }
                                if columns == 2 && pair.len() == 1 {
                                    ui.element().width(grow!()).height(fixed!(1.0)).empty();
                                }
                            });
                    }
                });
        });
}

fn action_groups(query: &str) -> Vec<ActionGroup> {
    let presets = actions::presets();

    // "Quick" group: pass-through and disabled.
    let quick: Vec<(&str, &str, Action, &'static GraphicAsset, &str)> = vec![
        (
            "Native action",
            "Keep the button's normal behavior",
            Action::PassThrough,
            &icons::PASSTHROUGH,
            "native pass through leave normal default",
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
                    p.display_label().to_string(),
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
                    preset.display_label().into(),
                    None,
                    preset.keys.iter().map(|s| s.to_string()).collect(),
                    (preset.action)(),
                    preset.cat.icon(),
                ));
            }
        }
        browse_categories(ui, app, &groups, current, chosen);
        return;
    }
    let body = ui.element().width(grow!()).height(grow!(0.0));
    let body = if empty {
        body
    } else {
        body.overflow(|o| {
            o.scroll_y().scrollbar(|s| {
                s.width(5.0)
                    .corner_radius(6.0)
                    .thumb_color(pal.line_strong)
                    .track_color(pal.pop)
            })
        })
    };
    body.layout(|l| l.direction(TopToBottom).padding((7, 16, 7, 16)))
        .children(|ui| {
            if empty {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| {
                        l.direction(TopToBottom)
                            .align(CenterX, CenterY)
                            .gap(12)
                            .padding(16)
                    })
                    .children(|ui| {
                        widgets::icon(ui, &icons::SEARCH, 25.0, pal.muted);
                        ui.text("No matching actions", |t| {
                            t.font(&fonts::SANS_BOLD).font_size(16).color(pal.text)
                        });
                        ui.element()
                            .width(fit!())
                            .height(fit!())
                            .layout(|l| l.direction(LeftToRight).align(CenterX, CenterY).gap(8))
                            .children(|ui| {
                                ui.element()
                                    .id("empty_clear")
                                    .height(fixed!(34.0))
                                    .corner_radius(7.0)
                                    .background_color(pal.s2)
                                    .border(|b| b.all(1).color(pal.line_strong))
                                    .accessibility(|a| a.button("Clear search"))
                                    .layout(|l| l.align(CenterX, CenterY).padding((0, 12, 0, 12)))
                                    .children(|ui| {
                                        ui.text("Clear search", |t| {
                                            t.font(&fonts::SANS_SEMIBOLD)
                                                .font_size(14)
                                                .color(pal.text)
                                        });
                                        if ui.just_pressed() {
                                            app.picker_query.clear();
                                            ui.set_text_value("picker_search", "");
                                            ui.set_focus("picker_search");
                                        }
                                    });
                                ui.element()
                                    .id("empty_record")
                                    .border(|b| b.all(1).color(pal.accent))
                                    .height(fixed!(34.0))
                                    .corner_radius(7.0)
                                    .background_color(pal.s2)
                                    .accessibility(|a| a.button("Record a keyboard shortcut"))
                                    .layout(|l| l.align(Left, CenterY).padding((0, 12, 0, 12)))
                                    .children(|ui| {
                                        ui.text("Record a keyboard shortcut", |t| {
                                            t.font(&fonts::SANS_SEMIBOLD)
                                                .font_size(14)
                                                .color(pal.accent)
                                        });
                                        if ui.just_pressed() {
                                            app.shortcut_editor = true;
                                            app.capturing_key = true;
                                            ui.clear_focus();
                                        }
                                    });
                            });
                    });
                return;
            }
            for (gi, (header, items)) in groups.iter().enumerate() {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.padding((8, 10, 4, 10)))
                    .children(|ui| {
                        let count = items.len();
                        let result_count = if count == 1 {
                            "1 match".to_owned()
                        } else {
                            format!("{count} matches")
                        };
                        let heading = if gi == 0 {
                            draft_replacement_hint(app, current)
                        } else {
                            None
                        }
                        .unwrap_or_else(|| format!("{header}  ·  {result_count}"));
                        ui.text(&heading, |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                        });
                    });

                for (n, (label, desc, keys, action, glyph)) in items.iter().enumerate() {
                    let factory = app
                        .picker
                        .as_ref()
                        .filter(|_| *action == Action::PassThrough)
                        .map(|target| actions::display_for(app, target, action).label);
                    let label = factory.as_deref().unwrap_or(label);
                    let id: Id = ("pk", (gi * 100 + n) as u32).into();
                    let hov = ui.pointer_over(id.clone());
                    let is_current = action == app.pending_action.as_ref().unwrap_or(current);
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
                            shortcut_hint(ui, &pal, keys);
                            if !is_current {
                                ui.text("Select", |t| {
                                    t.font(&fonts::SANS_SEMIBOLD)
                                        .font_size(13)
                                        .color(pal.accent)
                                });
                            }
                            if is_current && app.pending_action.is_some() {
                                ui.text("Draft", |t| {
                                    t.font(&fonts::SANS_SEMIBOLD)
                                        .font_size(12)
                                        .color(pal.accent)
                                });
                            } else if is_current {
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
    target: &PickerTarget,
    current: &Action,
    chosen: &mut Option<Action>,
    close: &mut bool,
) {
    let pal = app.pal;
    let mods = app.custom_mods.clone();
    let key = app.custom_key;
    let capturing = app.capturing_key;
    let unchanged = key.is_some_and(|vk| Action::key(&mods, vk) == *current);
    let mut toggle_mod: Option<Modifier> = None;
    let mut start_capture = false;

    let short = ui
        .bounding_box("picker_panel")
        .is_some_and(|r| r.height < 380.0);
    ui.element()
        .width(grow!())
        .height(grow!(0.0))
        .overflow(|o| {
            o.scroll_y()
                .scrollbar(|s| s.width(6.0).thumb_color(pal.track))
        })
        .layout(|l| {
            l.direction(TopToBottom)
                .gap(if short { 4 } else { 12 })
                .padding((8, 16, 8, 16))
        })
        .children(|ui| {
            let assigned = actions::display_for(app, target, current);
            ui.element()
                .width(grow!())
                .height(fixed!(if short { 24.0 } else { 44.0 }))
                .border(|b| b.bottom(if short { 0 } else { 1 }).color(pal.line))
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
            ui.text(
                "Hold modifiers while recording, or choose them here.",
                |t| t.font(&fonts::SANS_BOLD).font_size(13).color(pal.faint),
            );
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
                                super::theme::Palette::mix(pal.accent, 16.0, pal.surface)
                            } else {
                                super::theme::TRANSPARENT
                            })
                            .border(|b| {
                                b.all(1)
                                    .color(if on { pal.accent } else { pal.line_strong })
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
                                    widgets::icon(ui, &icons::CHECK, 12.0, pal.accent);
                                }
                                ui.text(label, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(if on {
                                        pal.text
                                    } else {
                                        pal.muted
                                    })
                                });
                            });
                    }
                });
            ui.element()
                .width(grow!())
                .height(fixed!(if short { 54.0 } else { 90.0 }))
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
                .layout(|l| {
                    l.direction(LeftToRight)
                        .gap(8)
                        .align(Left, CenterY)
                        .padding((0, 12, 0, 12))
                })
                .children(|ui| {
                    ui.element()
                        .width(grow!())
                        .height(fit!())
                        .layout(|l| l.direction(TopToBottom).gap(3))
                        .children(|ui| {
                            let label = if capturing {
                                "Press a key"
                            } else if unchanged {
                                "Saved shortcut"
                            } else {
                                "Unsaved shortcut"
                            };
                            ui.text(label, |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                            });
                            if capturing {
                                ui.text("Esc cancels recording.", |t| {
                                    t.font(&fonts::SANS).font_size(13).color(pal.muted)
                                });
                            } else if let Some(vk) = key {
                                let chord =
                                    actions::display(&Action::key(&mods, vk)).keys.join(" + ");
                                ui.text(&chord, |t| {
                                    t.font(&fonts::MONO_BOLD)
                                        .font_size(if mods.is_empty() { 18 } else { 13 })
                                        .color(pal.text)
                                });
                            } else {
                                ui.text("No key recorded", |t| {
                                    t.font(&fonts::SANS).font_size(13).color(pal.muted)
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
                        .width(fixed!(if short { 117.0 } else { 132.0 }))
                        .height(fixed!(36.0))
                        .corner_radius(8.0)
                        .background_color(if capturing {
                            super::theme::Palette::mix(pal.accent, 16.0, pal.win)
                        } else {
                            super::theme::TRANSPARENT
                        })
                        .border(|b| b.all(1).color(pal.accent))
                        .layout(|l| l.align(CenterX, CenterY).padding((0, 8, 0, 8)))
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
                                key.map(|_| "Record again".to_owned())
                                    .unwrap_or_else(|| "Record key".into())
                            };
                            ui.text(&label, |t| {
                                t.font(&fonts::SANS_SEMIBOLD)
                                    .font_size(14)
                                    .color(pal.accent)
                            });
                        });
                });
        });
    let save_state = if capturing || key.is_none() {
        SaveState::Empty
    } else if unchanged {
        SaveState::Saved
    } else {
        SaveState::Ready
    };
    let assign = assignment_footer(ui, app, EditorTab::Shortcut, save_state, close);
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
        *chosen = Some(key.map_or_else(|| current.clone(), |k| Action::key(&app.custom_mods, k)));
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SaveState {
    Empty,
    Ready,
    Saved,
}
enum EditorTab {
    Scrolling,
    Actions,
    Shortcut,
}

fn assignment_footer(
    ui: &mut Ui2,
    app: &mut App,
    tab: EditorTab,
    state: SaveState,
    close: &mut bool,
) -> bool {
    let pal = app.pal;
    if app.wheel_switch.is_some() {
        ui.element()
            .width(grow!())
            .height(fit!())
            .border(|b| b.top(1).color(pal.line))
            .layout(|l| {
                l.direction(LeftToRight)
                    .align(Left, CenterY)
                    .gap(6)
                    .padding((6, 12, 0, 12))
            })
            .children(|ui| {
                ui.text("Wheel edits", |t| {
                    t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.text)
                });
                if super::scroll::choice(ui, &pal, "keep_wheel".into(), "Keep editing", true) {
                    app.wheel_switch = None;
                }
                if super::scroll::choice(ui, &pal, "discard_wheel".into(), "Discard edits", false) {
                    if let Some((target, anchor)) = app.wheel_switch.take() {
                        app.open_button_popover(target, anchor);
                    }
                }
            });
        return false;
    }
    let (cancel_id, save_id, save_label) = match tab {
        EditorTab::Scrolling => ("cancel_wheel", "save_wheel", "Save"),
        EditorTab::Actions => ("cancel_action", "save_action", "Save action"),
        EditorTab::Shortcut => ("cancel_shortcut", "assign", "Save shortcut"),
    };
    let save_label = if app.wheel_draft.is_some() {
        "Save"
    } else {
        save_label
    };
    let state = if app
        .wheel_draft
        .as_ref()
        .is_some_and(|d| d != &app.draft.device)
        && !app.capturing_key
    {
        SaveState::Ready
    } else {
        state
    };
    let mut assign = false;
    ui.element()
        .width(grow!())
        .height(fit!())
        .border(|b| b.top(1).color(pal.line))
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(7)
                .padding((if screen_height() < 430.0 { 4 } else { 8 }, 16, 0, 16))
        })
        .children(|ui| {
            ui.element()
                .id(cancel_id)
                .accessibility(|a| {
                    a.button(if state == SaveState::Saved {
                        "Done"
                    } else {
                        "Cancel changes"
                    })
                })
                .height(fixed!(32.0))
                .corner_radius(8.0)
                .border(|b| b.all(1).color(pal.line_strong))
                .layout(|l| l.align(CenterX, CenterY).padding((0, 16, 0, 16)))
                .children(|ui| {
                    if ui.just_pressed() {
                        *close = true;
                    }
                    ui.text(
                        if state == SaveState::Saved {
                            "Done"
                        } else {
                            "Cancel"
                        },
                        |t| t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.text),
                    );
                });
            ui.element().width(grow!()).height(fixed!(1.0)).empty();

            if app.device_view.paused {
                ui.text("Applies after resume", |t| {
                    t.font(&fonts::SANS_MEDIUM).font_size(12).color(pal.muted)
                });
            } else if app.wheel_draft.is_some() && state == SaveState::Ready {
                ui.text("Unsaved changes", |t| {
                    t.font(&fonts::SANS_MEDIUM).font_size(12).color(pal.muted)
                });
            }
            if state != SaveState::Ready {
                ui.element()
                    .id(save_id)
                    .width(fixed!(127.0))
                    .height(fixed!(32.0))
                    .corner_radius(8.0)
                    .background_color(if state == SaveState::Saved {
                        super::theme::TRANSPARENT
                    } else {
                        pal.s2
                    })
                    .layout(|l| l.align(CenterX, CenterY))
                    .children(|ui| {
                        if state == SaveState::Saved {
                            widgets::icon(ui, &icons::CHECK, 14.0, pal.good);
                            ui.text("Saved", |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.good)
                            });
                        } else {
                            ui.text(save_label, |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.faint)
                            });
                        }
                    });
                return;
            }
            ui.element()
                .id(save_id)
                .accessibility(|a| a.button(save_label))
                .width(fixed!(127.0))
                .height(fixed!(32.0))
                .corner_radius(8.0)
                .background_color(super::theme::Palette::mix(
                    pal.pop,
                    30.0 * feedback_pulse(app),
                    pal.accent,
                ))
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
                    ui.text(save_label, |t| {
                        t.font(&fonts::SANS_BOLD)
                            .font_size(widgets::px(12.5))
                            .color(pal.accent_ink)
                    });
                });
        });

    assign
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

/// Keep the attempted action and the saved binding separate until the write succeeds.
fn retry_assignment(ui: &mut Ui2, app: &mut App, action: &Action, recovery: &str) -> bool {
    let pal = app.pal;
    let display = actions::display(action);
    let name = if display.label.is_empty() {
        display.keys.join(" + ")
    } else {
        display.label
    };
    let mut retry = false;
    ui.element()
        .width(grow!())
        .height(grow!(0.0))
        .overflow(|o| o.scroll_y())
        .layout(|l| l.direction(TopToBottom).gap(12).padding(16))
        .children(|ui| {
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.gap(8).align(Left, CenterY))
                .children(|ui| {
                    widgets::icon(ui, &icons::WARN, 18.0, pal.danger);
                    ui.text("Couldn't save assignment", |t| {
                        t.font(&fonts::SANS_BOLD).font_size(17).color(pal.text)
                    });
                });
            ui.text(&format!("{name} is still unsaved."), |t| {
                t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
            });
            ui.text(recovery, |t| {
                t.font(&fonts::SANS).font_size(15).color(pal.text)
            });
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.gap(8))
                .children(|ui| {
                    for (id, label) in [
                        ("retry_assignment", "Try again"),
                        ("edit_assignment", "Back to editing"),
                    ] {
                        let primary = id == "retry_assignment";
                        ui.element()
                            .id(id)
                            .height(fixed!(36.0))
                            .corner_radius(8.0)
                            .background_color(if primary { pal.accent } else { pal.s2 })
                            .accessibility(|a| a.button(label))
                            .layout(|l| l.padding((0, 14, 0, 14)).align(CenterX, CenterY))
                            .children(|ui| {
                                ui.text(label, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD)
                                        .font_size(14)
                                        .color(if primary { pal.accent_ink } else { pal.text })
                                });
                                if ui.just_pressed() {
                                    if primary {
                                        retry = true;
                                    } else {
                                        app.failed_assignment = None;
                                    }
                                }
                            });
                    }
                });
        });
    retry
}

fn feedback_pulse(app: &App) -> f32 {
    app.picker_feedback
        .map(|t| {
            let progress = (t.elapsed().as_secs_f32() / SELECTION_FEEDBACK_SECONDS).clamp(0.0, 1.0);
            (1.0 - progress).powi(2)
        })
        .unwrap_or(0.0)
}
