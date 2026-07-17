//! Buttons section: per-profile remapping of back / forward / middle.

use super::widgets::{self, Ui2};
use super::{actions, fonts, icons, App, PickerTarget};
use ply_engine::prelude::*;

/// The three physical buttons, as the design names them.
const ROWS: [(&str, &str, &str); 3] = [
    ("back", "Back button", "Lower side button"),
    ("forward", "Forward button", "Upper side button"),
    ("middle", "Middle click", "Press the scroll wheel"),
];

fn row_icon(slot: &str) -> &'static GraphicAsset {
    match slot {
        "back" => &icons::BTN_BACK,
        "forward" => &icons::BTN_FORWARD,
        _ => &icons::BTN_MIDDLE,
    }
}

/// Ply's `Id` is built from (&'static str, u32), so slots need a stable index.
fn slot_index(slot: &str) -> u32 {
    match slot {
        "back" => 0,
        "forward" => 1,
        _ => 2,
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
            ui.element()
                .width(grow!())
                .height(fit!())
                .children(|ui| {
                    widgets::heading(
                        ui,
                        &pal,
                        "Buttons",
                        "Remap the three physical buttons. Mappings are saved per profile.",
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
                            b.all(1).color(if hovered { pal.accent } else { pal.line_strong })
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
                            ui.element()
                                .width(grow!())
                                .height(fit!())
                                .children(|ui| {
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

    // The three rows.
    let idx = app.selected_profile;
    let bindings: Vec<(String, String, String, actions::Display)> = ROWS
        .iter()
        .map(|(slot, name, desc)| {
            let action = app
                .profile_at(idx)
                .map(|p| match *slot {
                    "back" => p.buttons.back.clone(),
                    "forward" => p.buttons.forward.clone(),
                    _ => p.buttons.middle.clone(),
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
                    .background_color(if active || hovered { pal.s2 } else { super::theme::TRANSPARENT })
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
                        ui.element()
                            .width(grow!())
                            .height(fit!())
                            .children(|ui| {
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
