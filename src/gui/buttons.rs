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

fn row_icon(slot: &str) -> &'static GraphicAsset {
    match slot {
        "back" => &icons::BTN_BACK,
        "forward" => &icons::BTN_FORWARD,
        _ => &icons::BTN_MIDDLE,
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

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let mut open_picker: Option<PickerTarget> = None;
    let mut toggle_menu = false;
    let mut pick_profile: Option<usize> = None;

    // Heading + profile dropdown.
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).align(Left, Top).gap(16))
        .children(|ui| {
            ui.element().width(grow!()).height(fit!()).children(|ui| {
                widgets::heading(
                    ui,
                    &pal,
                    "Buttons",
                    "Remap available mouse buttons. Mappings are saved per profile.",
                );
            });
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
                    widgets::eyebrow(ui, &pal, "Profile");
                    let hovered = ui.pointer_over("profile_btn");
                    ui.element()
                        .id("profile_btn")
                        .width(fit!(168.0))
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

    ui.element().width(grow!()).height(fixed!(22.0)).empty();

    if super::mouse_view::matches_device(&app.device_view.device) && screen_width() >= 840.0 {
        if toggle_menu {
            app.profile_menu_open = !app.profile_menu_open;
        }
        if let Some(i) = pick_profile {
            app.selected_profile = i;
            app.profile_menu_open = false;
        }
        super::mouse_view::section(ui, app);
        return;
    }

    // Standard buttons and any extra controls reported by the mouse.
    let idx = app.selected_profile;
    let rows: Vec<(String, String, String)> = editable_buttons(&app.device_view.device, &app.draft)
        .into_iter()
        .map(|button| {
            (
                button.slot.clone(),
                button.name.clone(),
                if button.slot.starts_with("control_") {
                    "Pass through keeps its normal action."
                } else {
                    "Choose an action for this button."
                }
                .to_owned(),
            )
        })
        .collect();
    let bindings: Vec<(String, String, String, actions::Display)> = rows
        .iter()
        .map(|(slot, name, desc)| {
            let action = app
                .profile_at(idx)
                .map(|p| match slot.as_str() {
                    "back" => p.buttons.back.clone(),
                    "forward" => p.buttons.forward.clone(),
                    "middle" => p.buttons.middle.clone(),
                    _ => slot
                        .strip_prefix("control_")
                        .and_then(|key| p.buttons.controls.get(key))
                        .cloned()
                        .unwrap_or_default(),
                })
                .unwrap_or_default();
            (
                slot.to_string(),
                name.to_string(),
                desc.to_string(),
                actions::display(&action),
            )
        })
        .collect();

    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(10))
        .children(|ui| {
            for (slot, name, desc, disp) in &bindings {
                widgets::card(ui, &pal)
                    .layout(|l| {
                        l.direction(LeftToRight)
                            .align(Left, CenterY)
                            .gap(16)
                            .padding((15, 17, 15, 17))
                    })
                    .children(|ui| {
                        widgets::icon(ui, row_icon(slot), 26.0, pal.muted);
                        ui.element()
                            .width(grow!())
                            .height(fit!())
                            .layout(|l| l.direction(TopToBottom).gap(1))
                            .children(|ui| {
                                ui.text(name, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                                });
                                ui.text(desc, |t| {
                                    t.font(&fonts::SANS)
                                        .font_size(widgets::px(12.5))
                                        .color(pal.muted)
                                });
                            });
                        let id: Id = ("btn_chip", slot_index(slot)).into();
                        if widgets::action_chip(ui, &pal, id, disp) {
                            open_picker = Some(PickerTarget::Button(slot.clone()));
                        }
                    });
            }
        });

    let gesture_target = PickerTarget::Gesture("tap".into());
    if gesture_target.available(&app.device_view.device) {
        ui.element().height(fixed!(10.0)).empty();
        let display = actions::display(&app.action_for(&gesture_target));
        widgets::card(ui, &pal)
            .layout(|l| {
                l.direction(LeftToRight)
                    .align(Left, CenterY)
                    .gap(16)
                    .padding((15, 17, 15, 17))
            })
            .children(|ui| {
                widgets::icon(ui, &icons::G_TAP, 26.0, pal.muted);
                ui.element().width(grow!()).height(fit!()).children(|ui| {
                    let label =
                        if app.device_view.device.capabilities.gesture_control == Some(0x01a0) {
                            "Haptic Sense Panel tap"
                        } else {
                            "Gesture button tap"
                        };
                    ui.text(label, |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                    });
                    let note = if app.selected_gestures().enabled {
                        "Uses the selected profile. Configure swipes in Gestures."
                    } else {
                        "Gestures are off. Saved actions remain editable."
                    };
                    ui.text(note, |t| {
                        t.font(&fonts::SANS).font_size(12).color(pal.muted)
                    });
                });
                if widgets::action_chip(ui, &pal, "gesture_chip".into(), &display) {
                    open_picker = Some(gesture_target);
                }
            });
    }

    ui.element().width(grow!()).height(fixed!(16.0)).empty();
    ui.text(
        "Left-click and scroll can't be remapped — Windows needs them. Changes apply the \
         instant you pick them.",
        |t| {
            t.font(&fonts::SANS)
                .font_size(widgets::px(12.5))
                .color(pal.faint)
        },
    );

    if toggle_menu {
        app.profile_menu_open = !app.profile_menu_open;
    }
    if let Some(i) = pick_profile {
        app.selected_profile = i;
        app.profile_menu_open = false;
    }
    if let Some(t) = open_picker {
        app.open_picker(t);
    }
}

/// Dropdown listing the profiles. Floats over the content below it.
///
/// `attach_id` is not optional decoration: `FloatingAttachToElement::None` is
/// the default, and it means *not attached*, so the whole floating config is
/// skipped and the menu lays out inline — shoving every row below it down the
/// page. (`attach_parent`'s doc comment calls itself the default; it is not.)
/// Anchoring the menu's top-right to the button's bottom-right reproduces the
/// design's `top: 100%; right: 0; margin-top: 6px`.
fn profile_menu(ui: &mut Ui2, app: &App, names: &[String], pick: &mut Option<usize>) {
    let pal = app.pal;
    let selected = app.selected_profile;
    ui.element()
        .width(fixed!(200.0))
        .height(fit!())
        .floating(|f| {
            f.attach_id("profile_btn")
                .anchor((Right, Top), (Right, Bottom))
                .offset((0.0, 6.0))
                .z_index(50)
        })
        .corner_radius(11.0)
        .background_color(pal.pop)
        .border(|b| b.all(1).color(pal.line_strong))
        .layout(|l| l.direction(TopToBottom).gap(2).padding(5))
        .children(|ui| {
            for (i, name) in names.iter().enumerate() {
                let id: Id = ("profile_opt", i as u32).into();
                let hovered = ui.pointer_over(id.clone());
                let active = i == selected;
                ui.element()
                    .id(id)
                    .width(grow!())
                    .height(fixed!(34.0))
                    .corner_radius(8.0)
                    .background_color(if active || hovered {
                        pal.s2
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
