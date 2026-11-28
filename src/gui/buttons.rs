//! Buttons section: per-profile remapping of standard and HID++ buttons.

use super::widgets::{self, Ui2};
use super::{actions, fonts, icons, App, PickerTarget};
use crate::config::Config;
use crate::hidpp::{
    device::DeviceStatus,
    features::reprog::{control_name, ButtonInfo},
};
use ply_engine::prelude::*;

/// The three physical buttons, as the design names them.
const ROWS: [(&str, &str); 3] = [
    ("back", "Back button"),
    ("forward", "Forward button"),
    ("middle", "Middle click"),
];

/// Offline rows edit saved actions only. Connected rows always use the mouse's
/// reported controls, so saved mappings cannot invent hardware support.
pub(super) fn editable_buttons(device: &DeviceStatus, saved: &Config) -> Vec<ButtonInfo> {
    if device.connected {
        return device.capabilities.buttons.clone();
    }
    let mut buttons: Vec<_> = ROWS
        .iter()
        .map(|(slot, name)| ButtonInfo {
            default_task_id: None,
            cid: match *slot {
                "middle" => 0x0052,
                "back" => 0x0053,
                _ => 0x0056,
            },
            slot: (*slot).into(),
            name: (*name).into(),
        })
        .collect();
    // Every known Master body has the top wheel-mode button. Keep its saved
    // mapping available even before the first assignment.
    let mut controls = std::collections::BTreeSet::new();
    if super::mouse_view::matches_device(device) {
        controls.insert(0x00c4);
    }
    // A saved control remains available when switching to a profile where it
    // has no assignment yet.
    for profile in std::iter::once(&saved.default_profile).chain(saved.profiles.iter()) {
        controls.extend(
            profile
                .buttons
                .controls
                .keys()
                .filter_map(|key| control_id(key)),
        );
    }
    buttons.extend(controls.into_iter().map(|cid| ButtonInfo {
        default_task_id: None,
        cid,
        slot: format!("control_{cid:04x}"),
        name: control_name(cid),
    }));
    buttons
}

pub(super) fn control_id(key: &str) -> Option<u16> {
    (key.len() == 4
        && key
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)))
    .then(|| u16::from_str_radix(key, 16).ok())
    .flatten()
    .filter(|cid| !matches!(cid, 0 | 0x0050..=0x0056))
}

pub(super) fn wheel_summary(device: &crate::hidpp::device::DeviceStatus, click: String) -> String {
    if super::scroll::available(device) {
        format!("Scrolling · {click}")
    } else {
        click
    }
}

fn row_icon(slot: &str) -> &'static GraphicAsset {
    match slot {
        "back" => &icons::BTN_BACK,
        "forward" => &icons::BTN_FORWARD,
        "middle" => &icons::BTN_MIDDLE,
        "control_00c4" => &icons::WHEEL_MODE,
        _ => &icons::NAV_BUTTONS,
    }
}

/// Ply's `Id` is built from (&'static str, u32), so slots need a stable index.
pub(super) fn slot_index(slot: &str) -> u32 {
    match slot {
        "back" => 0,
        "forward" => 1,
        "middle" => 2,
        _ => slot
            .strip_prefix("control_")
            .and_then(|key| u32::from_str_radix(key, 16).ok())
            .map(|cid| cid + 3)
            .unwrap_or(3),
    }
}

pub(super) fn profile_heading(ui: &mut Ui2, app: &mut App, title: &str, subtitle: &str) {
    let pal = app.pal;
    let mut toggle_menu = false;
    let mut pick_profile: Option<usize> = None;

    if app.profile_menu_open
        && (is_key_pressed(KeyCode::Escape)
            || (is_mouse_button_pressed(MouseButton::Left)
                && !ui.pointer_over("profile_menu")
                && !ui.pointer_over("profile_btn")))
    {
        app.profile_menu_open = false;
    }
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(16))
        .children(|ui| {
            ui.text(title, |t| {
                t.font(&fonts::DISPLAY_BOLD).font_size(30).color(pal.text)
            });
            if screen_width() >= 840.0 {
                ui.element().width(grow!()).height(fixed!(1.0)).empty();
            }
            // Profile picker.
            let names = app.profile_names();
            let current = names
                .get(app.selected_profile)
                .cloned()
                .unwrap_or_else(|| "Default".into());
            ui.element()
                .width(fit!())
                .height(fit!())
                .layout(|l| l.direction(TopToBottom).align(Right, Top).gap(5))
                .children(|ui| {
                    let hovered = ui.pointer_over("profile_btn");
                    ui.element()
                        .id("profile_btn")
                        .width(fixed!(212.0))
                        .height(fixed!(38.0))
                        .corner_radius(10.0)
                        .background_color(pal.surface)
                        .border(|b| {
                            b.all(1)
                                .color(if hovered { pal.accent } else { pal.line_strong })
                        })
                        .layout(|l| {
                            l.direction(LeftToRight)
                                .align(Left, CenterY)
                                .gap(10)
                                .padding((0, 12, 0, 12))
                        })
                        .children(|ui| {
                            if ui.just_pressed() {
                                toggle_menu = true;
                            }
                            ui.text("Edit profile", |t| {
                                t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.muted)
                            });
                            ui.element().width(grow!()).height(fit!()).children(|ui| {
                                ui.text(&current, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD)
                                        .font_size(widgets::px(13.5))
                                        .color(pal.text)
                                });
                            });
                            widgets::icon(ui, &icons::CHEVRON_DOWN, 13.0, pal.muted);
                        });

                    if app.profile_menu_open {
                        profile_menu(ui, app, &names, &mut pick_profile);
                    }
                });
        });

    ui.element().height(fixed!(5.0)).empty();
    let notice = app
        .saved_notice
        .as_ref()
        .filter(|_| app.inline_mapping_notice() && app.picker.is_none() && !app.profile_menu_open)
        .map(|(message, _, _, _)| {
            if message.chars().count() <= 48 {
                message.as_str()
            } else {
                "Mapping saved"
            }
        });
    let offer_diagram = app.section == super::Section::Buttons
        && screen_width() < 840.0
        && super::mouse_view::matches_device(&app.device_view.device);
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(8))
        .children(|ui| {
            ui.element()
                .id("profile_caption")
                .width(if offer_diagram { grow!(0.0) } else { fit!() })
                .height(fit!())
                .children(|ui| {
                    // The caption sets the row height even when an offline hint wraps.
                    ui.text(subtitle, |t| {
                        t.font(&fonts::SANS)
                            .font_size(14)
                            .color(if notice.is_some() {
                                super::theme::TRANSPARENT
                            } else {
                                pal.muted
                            })
                    });
                    if let Some(message) = notice {
                        ui.element()
                            .width(fit!())
                            .height(fit!())
                            .floating(|f| {
                                f.attach_id("profile_caption")
                                    .anchor((Left, CenterY), (Left, CenterY))
                                    .passthrough()
                            })
                            .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(6))
                            .children(|ui| {
                                widgets::icon(ui, &icons::CHECK, 14.0, pal.good);
                                ui.text(message, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                                });
                            });
                    }
                });
            if offer_diagram {
                let label = if app.compact_diagram {
                    "Button list"
                } else {
                    "Show diagram"
                };
                ui.element()
                    .id("compact_diagram_toggle")
                    .height(fixed!(26.0))
                    .width(fixed!(112.0))
                    .corner_radius(6.0)
                    .background_color(pal.surface)
                    .border(|b| b.all(1).color(pal.line_strong))
                    .accessibility(|a| a.button(label))
                    .layout(|l| l.align(CenterX, CenterY))
                    .children(|ui| {
                        ui.text(label, |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                        });
                        if ui.just_pressed() {
                            app.compact_diagram = !app.compact_diagram;
                        }
                    });
            }
        });

    if toggle_menu {
        app.profile_menu_open = !app.profile_menu_open;
    }
    if let Some(i) = pick_profile {
        app.selected_profile = i;
        app.profile_menu_open = false;
    }
}

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let mut open_picker = None;
    let subtitle = if app.mouse_awake() {
        if screen_width() < 840.0 && !app.compact_diagram {
            "Choose a control to edit."
        } else {
            "Select a control on the mouse or in the list."
        }
    } else {
        "Offline edits apply when your mouse reconnects."
    };
    profile_heading(ui, app, "Mouse controls", subtitle);
    ui.element()
        .width(grow!())
        .height(fixed!(if screen_width() < 840.0 { 12.0 } else { 22.0 }))
        .empty();

    if super::mouse_view::matches_device(&app.device_view.device)
        && (screen_width() >= 840.0 || app.compact_diagram)
    {
        super::mouse_view::section(ui, app);
        return;
    }

    // Standard buttons and any extra controls reported by the mouse.
    let mut buttons = editable_buttons(&app.device_view.device, &app.draft);
    buttons.sort_by_key(|b| match b.slot.as_str() {
        "middle" => (0, 0),
        "back" => (1, 0),
        "forward" => (2, 0),
        _ => (3, b.cid),
    });
    let device = &app.device_view.device;
    let mut rows = Vec::new();
    for button in buttons {
        let slot = button.slot;
        let mut name = app
            .prefs
            .button_names
            .get(&format!("{}/{}", device.display_name(), slot))
            .cloned()
            .unwrap_or(button.name);
        let id = ("btn_chip", slot_index(&slot)).into();
        let glyph = row_icon(&slot);
        let target = PickerTarget::Button(slot.clone());
        let mut action = actions::assignment(app, &target);
        if slot == "middle" && (target.opens_scroll(device) || !device.connected) {
            name = "Scroll wheel".into();
            let binding = if action.label.is_empty() {
                action.keys.join(" + ")
            } else {
                action.label
            };
            action.label = if matches!(action.kind, actions::Kind::PassThrough) {
                "Standard middle click".into()
            } else {
                format!("Middle click: {binding}")
            };
            action.label = wheel_summary(device, action.label);
            action.keys.clear();
        }
        rows.push(MappingRow {
            number: super::mouse_view::target_number(device, &target),
            target,
            id,
            glyph,
            name,
            action,
            note: None,
        });
    }
    let middle = PickerTarget::Button("middle".into());
    if middle.opens_scroll(device) && !middle.available(device) {
        rows.insert(
            0,
            MappingRow {
                number: super::mouse_view::target_number(device, &middle),
                target: middle,
                id: ("btn_chip", slot_index("middle")).into(),
                glyph: &icons::NAV_SCROLL,
                name: "Scroll wheel".into(),
                action: actions::Display {
                    label: "Scrolling settings".into(),
                    ..actions::display(&crate::config::Action::PassThrough)
                },
                note: None,
            },
        );
    }
    let target = PickerTarget::Gesture("tap".into());
    if target.available(device) {
        let name = if device.capabilities.gesture_control == Some(0x01a0) {
            "Haptic Sense Panel tap"
        } else {
            "Gesture button tap"
        };
        rows.push(MappingRow {
            number: super::mouse_view::target_number(device, &target),
            action: actions::display(&app.action_for(&target)),
            id: "gesture_chip".into(),
            glyph: &icons::G_TAP,
            name: name.into(),
            note: (!app.selected_gestures().enabled).then_some("Saved for when gestures are on."),
            target,
        });
    }
    rows.sort_by_key(|row| row.number.unwrap_or(usize::MAX));
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(10))
        .children(|ui| {
            for row in &rows {
                if mapping_row(ui, &pal, row) {
                    open_picker = Some(row.target.clone());
                }
            }
        });

    if super::Section::Gestures.available(&app.device_view.device) {
        ui.element().width(grow!()).height(fixed!(10.0)).empty();
        let hint = "Gesture actions follow the selected profile.";
        ui.element().width(grow!()).height(fit!()).children(|ui| {
            super::mouse_view::gesture_footer(ui, app, hint, "Configure gestures");
        });
    } else if app.mouse_awake() && !super::scroll::available(&app.device_view.device) {
        ui.element().width(grow!()).height(fixed!(16.0)).empty();
        ui.text("Windows handles left click and scrolling.", |t| {
            t.font(&fonts::SANS)
                .font_size(widgets::px(12.5))
                .color(pal.faint)
        });
    }

    if let Some(t) = open_picker {
        app.open_picker(t);
    }
}

struct MappingRow {
    target: PickerTarget,
    number: Option<usize>,
    id: Id,
    glyph: &'static GraphicAsset,
    name: String,
    action: actions::Display,
    note: Option<&'static str>,
}

fn mapping_row(ui: &mut Ui2, pal: &super::theme::Palette, row: &MappingRow) -> bool {
    let MappingRow {
        id,
        name,
        action,
        note,
        ..
    } = row;
    let hovered = ui.pointer_over(id.clone());
    let mut clicked = false;
    widgets::card(ui, pal)
        .id(id.clone())
        .border(|b| b.all(1).color(if hovered { pal.accent } else { pal.line }))
        .accessibility(|a| a.button(name))
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(14)
                .padding((8, 14, 8, 14))
        })
        .children(|ui| {
            clicked = ui.just_pressed();
            if let Some(number) = row.number {
                ui.element()
                    .width(fixed!(24.0))
                    .height(fixed!(24.0))
                    .corner_radius(12.0)
                    .border(|b| b.all(1).color(pal.line_strong))
                    .background_color(pal.win)
                    .layout(|l| l.align(CenterX, CenterY))
                    .children(|ui| {
                        ui.text(&number.to_string(), |t| {
                            t.font(&fonts::MONO_BOLD).font_size(13).color(pal.text)
                        });
                    });
            } else {
                widgets::icon(ui, row.glyph, 24.0, pal.muted);
            }
            ui.element()
                .width(grow!(0.0))
                .height(fit!())
                .layout(|l| l.direction(TopToBottom).gap(4))
                .children(|ui| {
                    ui.element()
                        .width(grow!())
                        .height(fit!())
                        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(10).wrap())
                        .children(|ui| {
                            ui.text(name, |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(15).color(pal.text)
                            });
                            if let Some(note) = note {
                                ui.text(note, |t| {
                                    t.font(&fonts::SANS).font_size(12).color(pal.muted)
                                });
                            }
                        });
                    ui.element()
                        .height(fixed!(22.0))
                        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(5))
                        .children(|ui| {
                            for key in &action.keys {
                                widgets::keycap(ui, pal, key);
                            }
                            if !action.label.is_empty() {
                                ui.text(&action.label, |t| {
                                    t.font(&fonts::SANS_MEDIUM).font_size(14).color(pal.accent)
                                });
                            }
                        });
                });
            widgets::icon(ui, &icons::CHEVRON_RIGHT, 15.0, pal.muted);
        });
    clicked
}

/// Anchor to the trigger so opening the menu does not shift the mappings.
fn profile_menu(ui: &mut Ui2, app: &App, names: &[String], pick: &mut Option<usize>) {
    let pal = app.pal;
    let selected = app.selected_profile;
    let wide = screen_width() >= 840.0;
    ui.element()
        .id("profile_menu")
        .width(fixed!(212.0))
        .height(fit!(0.0, (screen_height() - 170.0).clamp(52.0, 298.0)))
        .overflow(|o| {
            o.scroll_y()
                .scrollbar(|s| s.width(6.0).thumb_color(pal.track))
        })
        .floating(|f| {
            f.attach_id("profile_btn")
                .anchor((Right, Top), (Right, Top))
                .offset((0.0, if wide { 41.0 } else { 65.0 }))
                .z_index(50)
        })
        .corner_radius(11.0)
        .background_color(pal.pop)
        .border(|b| b.all(1).color(pal.line_strong))
        .layout(|l| l.direction(TopToBottom).gap(1).padding(2))
        .children(|ui| {
            for (i, name) in names.iter().enumerate() {
                let id: Id = ("profile_opt", i as u32).into();
                let hovered = ui.pointer_over(id.clone());
                let active = i == selected;
                ui.element()
                    .id(id)
                    .width(grow!())
                    .height(fit!(26.0))
                    .corner_radius(8.0)
                    .background_color(if active || hovered {
                        super::theme::Palette::mix(pal.accent, 12.0, pal.pop)
                    } else {
                        super::theme::TRANSPARENT
                    })
                    .layout(|l| {
                        l.direction(LeftToRight)
                            .align(Left, CenterY)
                            .gap(10)
                            .padding((0, 11, 0, 11))
                    })
                    .children(|ui| {
                        if ui.just_pressed() {
                            *pick = Some(i);
                        }
                        ui.element().width(grow!()).height(fit!()).children(|ui| {
                            ui.text(name, |t| {
                                t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.text)
                            });
                        });
                        if active {
                            widgets::icon(ui, &icons::CHECK, 14.0, pal.accent);
                        }
                    });
            }
            if let Some(name) = app
                .device_view
                .active_profile
                .as_ref()
                .filter(|_| !app.device_view.paused)
            {
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .border(|b| b.top(1).color(pal.line))
                    .layout(|l| l.direction(TopToBottom).gap(3).padding((7, 10, 7, 10)))
                    .children(|ui| {
                        ui.text("Last reported active profile", |t| {
                            t.font(&fonts::SANS).font_size(12).color(pal.muted)
                        });
                        ui.text(name, |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                        });
                    });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Action;

    #[test]
    fn offline_controls_include_master_top_button_and_saved_dpi_mappings() {
        let device = DeviceStatus {
            name: Some("MX Master 2S".into()),
            ..Default::default()
        };
        let mut saved = Config::default();
        for key in ["00c4", "00ed", "00fd", "0050", "not-a-control"] {
            saved
                .default_profile
                .buttons
                .controls
                .insert(key.into(), Action::Disabled);
        }
        saved.profiles.push(crate::config::Profile {
            name: "Mapped app".into(),
            match_processes: vec!["mapped.exe".into()],
            buttons: std::mem::take(&mut saved.default_profile.buttons),
            gestures: None,
        });
        let buttons = editable_buttons(&device, &saved);
        let slots: Vec<_> = buttons.iter().map(|b| b.slot.as_str()).collect();
        assert_eq!(
            slots,
            [
                "back",
                "forward",
                "middle",
                "control_00c4",
                "control_00ed",
                "control_00fd"
            ]
        );
        assert_eq!(buttons[4].name, "DPI change button");
        assert_eq!(buttons[5].name, "DPI switch button");
        let mut connected = device.clone();
        connected.connected = true;
        assert!(
            editable_buttons(&connected, &saved).is_empty(),
            "Saved mappings cannot create connected capabilities"
        );
        connected.capabilities.buttons.push(buttons[4].clone());
        assert_eq!(editable_buttons(&connected, &saved).len(), 1);
        assert_eq!(editable_buttons(&connected, &saved)[0].cid, 0x00ed);
    }
}
