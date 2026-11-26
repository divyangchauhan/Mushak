//! Profiles section: manage per-application profiles.
//!
//! Config keeps the default profile separate from the list; the design shows it
//! after application profiles because it is the fallback.

use super::widgets::{self, Ui2};
use super::{fonts, icons, App};
use crate::config::Profile;
use ply_engine::prelude::*;
use std::time::{Duration, Instant};

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let mut add = false;
    let mut extend_pick = false;
    let mut delete: Option<usize> = None;
    let mut move_up: Option<usize> = None;
    let mut move_down: Option<usize> = None;
    let mut remove_match: Option<(usize, usize)> = None;
    let mut pick_for: Option<usize> = None;

    let picking = app.pick_target_index();
    let secs_left = app.pick_seconds_left();
    let mut cancel_pick = false;
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).align(Left, Top).gap(16))
        .children(|ui| {
            ui.element().width(grow!()).height(fit!()).children(|ui| {
                widgets::heading(
                    ui,
                    &pal,
                    "Profiles",
                    if app.draft.profiles.is_empty() {
                        "Add a profile to customize buttons and gestures for an app."
                    } else {
                        "Mushak uses the first profile that matches the active app."
                    },
                );
            });
            if picking.is_some() {
                return;
            }
            let hovered = ui.pointer_over("btn_addprofile");
            ui.element()
                .id("btn_addprofile")
                .width(fit!())
                .height(fixed!(38.0))
                .corner_radius(10.0)
                .background_color(if hovered {
                    super::theme::Palette::mix(super::theme::WHITE, 6.0, pal.accent)
                } else {
                    pal.accent
                })
                .layout(|l| {
                    l.direction(LeftToRight)
                        .align(Left, CenterY)
                        .gap(7)
                        .padding((0, 14, 0, 14))
                })
                .children(|ui| {
                    if ui.just_pressed() {
                        add = true;
                    }
                    widgets::icon(ui, &icons::PLUS, 15.0, pal.accent_ink);
                    ui.text("Add profile", |t| {
                        t.font(&fonts::SANS_BOLD)
                            .font_size(13)
                            .color(pal.accent_ink)
                    });
                });
        });

    ui.element().width(grow!()).height(fixed!(22.0)).empty();

    // Row 0 is the default profile; 1.. are the app profiles.
    let names = app.profile_names();
    let count = names.len();
    let matches: Vec<Vec<String>> = (0..count)
        .map(|i| {
            app.profile_at(i)
                .map(|p| p.match_processes.clone())
                .unwrap_or_default()
        })
        .collect();
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(10))
        .children(|ui| {
            for i in (1..count).chain(std::iter::once(0)) {
                let locked = i == 0;
                let mut row_actions = |ui: &mut Ui2, app: &App| {
                    if i > 1 && profile_action(ui, app, ("p_up", i as u32), &icons::ARROW_UP) {
                        move_up = Some(i);
                    }
                    if i + 1 < count
                        && profile_action(ui, app, ("p_down", i as u32), &icons::ARROW_DOWN)
                    {
                        move_down = Some(i);
                    }
                    if profile_action(ui, app, ("p_del", i as u32), &icons::TRASH) {
                        delete = Some(i);
                    }
                };
                widgets::card(ui, &pal)
                    .layout(|l| l.direction(TopToBottom).padding((15, 17, 15, 17)))
                    .children(|ui| {
                        // Header: avatar, name, badge, controls.
                        ui.element()
                            .width(grow!())
                            .height(fit!())
                            .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(12))
                            .children(|ui| {
                                let initial = names[i]
                                    .chars()
                                    .next()
                                    .unwrap_or('?')
                                    .to_uppercase()
                                    .to_string();
                                ui.element()
                                    .width(fixed!(28.0))
                                    .height(fixed!(28.0))
                                    .corner_radius(8.0)
                                    .background_color(if locked {
                                        pal.s2
                                    } else {
                                        super::theme::Palette::mix(pal.accent, 18.0, pal.surface)
                                    })
                                    .layout(|l| l.align(CenterX, CenterY))
                                    .children(|ui| {
                                        let badge = if !locked && count > 2 {
                                            i.to_string()
                                        } else {
                                            initial
                                        };
                                        ui.text(&badge, |t| {
                                            t.font(&fonts::MONO_BOLD)
                                                .font_size(13)
                                                .color(if locked { pal.muted } else { pal.accent })
                                        });
                                    });
                                if app.renaming_profile == Some(i) {
                                    profile_name_editor(ui, app, i);
                                    return;
                                }
                                let name_id: Id = ("rename_profile", i as u32).into();
                                ui.element()
                                    .id(name_id)
                                    .width(grow!())
                                    .height(fit!(34.0))
                                    .layout(|l| {
                                        l.direction(LeftToRight).gap(7).align(Left, CenterY)
                                    })
                                    .accessibility(|a| {
                                        if locked {
                                            a.static_text(&names[i])
                                        } else {
                                            a.button(&format!("Rename {}", names[i]))
                                        }
                                    })
                                    .children(|ui| {
                                        ui.text(&names[i], |t| {
                                            t.font(&fonts::DISPLAY_BOLD)
                                                .font_size(widgets::px(14.5))
                                                .color(pal.text)
                                        });
                                        if !locked {
                                            widgets::icon(ui, &icons::CAT_EDIT, 16.0, pal.muted);
                                            if screen_width() >= 840.0 {
                                                ui.text("Rename", |t| {
                                                    t.font(&fonts::SANS_MEDIUM)
                                                        .font_size(13)
                                                        .color(pal.muted)
                                                });
                                            }
                                            if ui.just_pressed() {
                                                app.renaming_profile = Some(i);
                                                app.profile_name_error = None;
                                                ui.set_text_value("profile_name_input", &names[i]);
                                                ui.set_focus("profile_name_input");
                                                ui.set_selection(
                                                    "profile_name_input",
                                                    0,
                                                    names[i].chars().count(),
                                                );
                                            }
                                        }
                                    });
                                if locked {
                                    ui.element()
                                        .height(fixed!(22.0))
                                        .corner_radius(6.0)
                                        .background_color(pal.s2)
                                        .layout(|l| l.align(CenterX, CenterY).padding((0, 8, 0, 8)))
                                        .children(|ui| {
                                            ui.text("All other apps", |t| {
                                                t.font(&fonts::SANS_SEMIBOLD)
                                                    .font_size(11)
                                                    .color(pal.faint)
                                            });
                                        });
                                } else {
                                    row_actions(ui, app);
                                }
                            });

                        if app.renaming_profile == Some(i) {
                            if let Some(message) = app.profile_name_error {
                                ui.text(message, |t| {
                                    t.font(&fonts::SANS_MEDIUM).font_size(13).color(
                                        super::theme::Palette::mix(pal.text, 30.0, pal.danger),
                                    )
                                });
                            }
                        }
                        // The default profile matches nothing by definition.
                        if locked {
                            return;
                        }

                        ui.element().width(grow!()).height(fixed!(12.0)).empty();
                        ui.element()
                            .width(grow!())
                            .height(fixed!(1.0))
                            .background_color(pal.line)
                            .empty();
                        ui.element().width(grow!()).height(fixed!(12.0)).empty();
                        let waiting = picking == Some(i);
                        let caption = if waiting {
                            "Switch to the app window you want to add"
                        } else {
                            "Apps using this profile"
                        };
                        if waiting {
                            ui.text(&format!("{caption} · {secs_left}s"), |t| {
                                t.font(&fonts::SANS_BOLD).font_size(15).color(pal.text)
                            });
                        } else {
                            widgets::eyebrow(ui, &pal, caption);
                        }
                        ui.element().width(grow!()).height(fixed!(9.0)).empty();

                        ui.element()
                            .width(grow!())
                            .height(fit!())
                            .layout(|l| l.direction(LeftToRight).gap(7).wrap())
                            .children(|ui| {
                                for (j, m) in matches[i].iter().enumerate() {
                                    ui.element()
                                        .height(fixed!(28.0))
                                        .corner_radius(8.0)
                                        .background_color(pal.s2)
                                        .border(|b| b.all(1).color(pal.line))
                                        .layout(|l| {
                                            l.direction(LeftToRight)
                                                .align(Left, CenterY)
                                                .gap(6)
                                                .padding((0, 6, 0, 10))
                                        })
                                        .children(|ui| {
                                            ui.text(m, |t| {
                                                t.font(&fonts::MONO_MEDIUM)
                                                    .font_size(12)
                                                    .color(pal.text)
                                            });
                                            let id: Id = ("m_del", (i * 100 + j) as u32).into();
                                            let hov = ui.pointer_over(id.clone());
                                            ui.element()
                                                .id(id)
                                                .width(fixed!(16.0))
                                                .height(fixed!(16.0))
                                                .corner_radius(4.0)
                                                .background_color(if hov {
                                                    pal.danger
                                                } else {
                                                    super::theme::TRANSPARENT
                                                })
                                                .layout(|l| l.align(CenterX, CenterY))
                                                .children(|ui| {
                                                    if ui.just_pressed() {
                                                        remove_match = Some((i, j));
                                                    }
                                                    widgets::icon(
                                                        ui,
                                                        &icons::CLOSE_SMALL,
                                                        9.0,
                                                        if hov {
                                                            super::theme::WHITE
                                                        } else {
                                                            pal.faint
                                                        },
                                                    );
                                                });
                                        });
                                }

                                if waiting {
                                    ui.element()
                                        .id("extend_window_pick")
                                        .height(fixed!(28.0))
                                        .corner_radius(8.0)
                                        .background_color(pal.surface)
                                        .border(|b| b.all(1).color(pal.line_strong))
                                        .accessibility(|a| a.button("Add 6 seconds"))
                                        .layout(|l| {
                                            l.direction(LeftToRight)
                                                .align(Left, CenterY)
                                                .gap(4)
                                                .padding((0, 8, 0, 8))
                                        })
                                        .children(|ui| {
                                            ui.text("Add 6 seconds", |t| {
                                                t.font(&fonts::SANS_SEMIBOLD)
                                                    .font_size(13)
                                                    .color(pal.text)
                                            });
                                            if ui.just_pressed() {
                                                extend_pick = true;
                                            }
                                        });
                                }
                                let id: Id = if waiting {
                                    "cancel_window_pick".into()
                                } else {
                                    ("p_pick", i as u32).into()
                                };
                                let hov = ui.pointer_over(id.clone());
                                ui.element()
                                    .id(id)
                                    .height(fixed!(28.0))
                                    .corner_radius(8.0)
                                    .border(|b| {
                                        b.all(1).color(if hov {
                                            pal.accent
                                        } else {
                                            pal.line_strong
                                        })
                                    })
                                    .layout(|l| {
                                        l.direction(LeftToRight)
                                            .align(Left, CenterY)
                                            .gap(6)
                                            .padding((0, 11, 0, 11))
                                    })
                                    .children(|ui| {
                                        if ui.just_pressed() {
                                            if waiting {
                                                cancel_pick = true;
                                            } else {
                                                pick_for = Some(i);
                                            }
                                        }
                                        widgets::icon(
                                            ui,
                                            if waiting {
                                                &icons::CLOSE_SMALL
                                            } else {
                                                &icons::TARGET
                                            },
                                            13.0,
                                            if hov { pal.accent } else { pal.muted },
                                        );
                                        ui.text(
                                            if waiting { "Cancel" } else { "Pick a window" },
                                            |t| {
                                                t.font(&fonts::SANS_SEMIBOLD)
                                                    .font_size(12)
                                                    .color(if hov { pal.accent } else { pal.muted })
                                            },
                                        );
                                    });
                            });
                    });
            }
        });

    if extend_pick {
        if let Some(deadline) = &mut app.pick_deadline {
            *deadline += Duration::from_secs(6);
        }
    }
    if cancel_pick {
        app.pick_deadline = None;
    }
    if add {
        app.draft.profiles.push(Profile {
            name: "New profile".to_string(),
            match_processes: Vec::new(),
            buttons: Default::default(),
            gestures: None,
        });
        app.commit_config();
    }
    if let Some(i) = delete {
        if i > 0 {
            if app.renaming_profile == Some(i) {
                app.renaming_profile = None;
            }
            app.draft.profiles.remove(i - 1);
            if app.pick_target == i {
                app.pick_deadline = None;
            }
            for index in [&mut app.selected_profile, &mut app.pick_target]
                .into_iter()
                .chain(app.renaming_profile.iter_mut())
            {
                if *index == i {
                    *index = 0;
                } else if *index > i {
                    *index -= 1;
                }
            }
            app.commit_config();
        }
    }
    let reorder = move_up
        .map(|i| (i, i - 1))
        .or_else(|| move_down.map(|i| (i, i + 1)));
    if let Some((from, to)) = reorder {
        app.draft.profiles.swap(from - 1, to - 1);
        for index in [&mut app.selected_profile, &mut app.pick_target]
            .into_iter()
            .chain(app.renaming_profile.iter_mut())
        {
            if *index == from {
                *index = to;
            } else if *index == to {
                *index = from;
            }
        }
        app.commit_config();
    }
    if let Some((i, j)) = remove_match {
        if let Some(p) = app.profile_at_mut(i) {
            if j < p.match_processes.len() {
                p.match_processes.remove(j);
                app.commit_config();
            }
        }
    }
    if let Some(i) = pick_for {
        app.pick_target = i;
        app.pick_deadline = Some(Instant::now() + Duration::from_secs(6));
    }
}

fn profile_name_editor(ui: &mut Ui2, app: &mut App, index: usize) {
    let pal = app.pal;
    ui.element()
        .width(grow!())
        .height(fixed!(34.0))
        .corner_radius(6.0)
        .background_color(pal.win)
        .border(|b| b.all(1).color(pal.accent))
        .layout(|l| l.align(Left, CenterY).padding((0, 8, 0, 8)))
        .children(|ui| {
            ui.element()
                .id("profile_name_input")
                .width(grow!())
                .height(fixed!(26.0))
                .text_input(|t| {
                    t.font(&fonts::SANS_SEMIBOLD)
                        .font_size(14)
                        .placeholder("Profile name")
                        .text_color(pal.text)
                        .cursor_color(pal.accent)
                })
                .empty();
        });
    let cancel = profile_action(
        ui,
        app,
        ("cancel_profile_name", index as u32),
        &icons::CLOSE_SMALL,
    );
    let save = profile_action(ui, app, ("save_profile_name", index as u32), &icons::CHECK);
    if cancel || is_key_pressed(KeyCode::Escape) {
        app.renaming_profile = None;
        app.profile_name_error = None;
        ui.clear_focus();
    } else if save || is_key_pressed(KeyCode::Enter) {
        let value = ui.get_text_value("profile_name_input").trim().to_owned();
        if value.is_empty() || value.chars().any(char::is_control) {
            app.profile_name_error = Some("Enter a profile name on one line.");
            return;
        }
        let previous = std::mem::replace(&mut app.draft.profiles[index - 1].name, value);
        if app.commit_config() {
            app.renaming_profile = None;
            app.profile_name_error = None;
            ui.clear_focus();
        } else {
            app.draft.profiles[index - 1].name = previous;
            app.profile_name_error = Some("Could not save the name. Try again.");
        }
    }
}

/// A labeled profile command.
fn profile_action(
    ui: &mut Ui2,
    app: &App,
    id: (&'static str, u32),
    glyph: &'static GraphicAsset,
) -> bool {
    let pal = app.pal;
    let label = match id.0 {
        "save_profile_name" => "Save name",
        "cancel_profile_name" => "Cancel",
        "p_up" => "Move up",
        "p_down" => "Move down",
        _ => "Delete profile",
    };
    let visible_label = if screen_width() < 840.0 {
        match id.0 {
            "p_up" => "Up",
            "p_down" => "Down",
            "p_del" => "Delete",
            _ => label,
        }
    } else {
        label
    };
    let destructive = id.0 == "p_del";
    let primary = id.0 == "save_profile_name";
    let ink = if destructive {
        super::theme::Palette::mix(pal.text, 30.0, pal.danger)
    } else if primary {
        pal.accent_ink
    } else {
        pal.text
    };
    let id: Id = id.into();
    let mut clicked = false;
    let hov = ui.pointer_over(id.clone());
    ui.element()
        .id(id)
        .width(fit!())
        .accessibility(|a| a.button(label))
        .height(fixed!(34.0))
        .corner_radius(7.0)
        .background_color(if destructive {
            super::theme::Palette::mix(pal.danger, if hov { 18.0 } else { 6.0 }, pal.surface)
        } else if primary {
            pal.accent
        } else {
            pal.s2
        })
        .layout(|l| {
            l.direction(LeftToRight)
                .align(CenterX, CenterY)
                .gap(4)
                .padding((0, 6, 0, 6))
        })
        .children(|ui| {
            if ui.just_pressed() {
                clicked = true;
            }
            widgets::icon(ui, glyph, 15.0, ink);
            ui.text(visible_label, |t| {
                t.font(&fonts::SANS_MEDIUM).font_size(13).color(ink)
            });
        });
    clicked
}
