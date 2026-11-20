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
    let mut delete: Option<usize> = None;
    let mut move_up: Option<usize> = None;
    let mut move_down: Option<usize> = None;
    let mut remove_match: Option<(usize, usize)> = None;
    let mut pick_for: Option<usize> = None;

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
                    "Mushak uses the first profile that matches the active app.",
                );
            });
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
    let picking = app.pick_target_index();
    let secs_left = app.pick_seconds_left();
    let mut cancel_pick = false;

    if let Some(i) = picking {
        if let Some(name) = names.get(i) {
            ui.element()
                .width(grow!())
                .height(fit!())
                .corner_radius(10.0)
                .background_color(super::theme::Palette::mix(pal.accent, 14.0, pal.surface))
                .border(|b| b.left(3).color(pal.accent))
                .layout(|l| {
                    l.direction(LeftToRight)
                        .align(Left, CenterY)
                        .gap(12)
                        .padding(16)
                })
                .children(|ui| {
                    widgets::icon(ui, &icons::TARGET, 22.0, pal.accent);
                    ui.element()
                        .width(grow!())
                        .height(fit!())
                        .layout(|l| l.direction(TopToBottom).gap(5))
                        .children(|ui| {
                            ui.text("Switch to the window you want to add", |t| {
                                t.font(&fonts::SANS_BOLD).font_size(16).color(pal.text)
                            });
                            ui.text(&format!("The active app will be added to {name}."), |t| {
                                t.font(&fonts::SANS).font_size(13).color(pal.muted)
                            });
                        });
                    ui.text(&format!("{secs_left}s"), |t| {
                        t.font(&fonts::MONO_BOLD).font_size(18).color(pal.accent)
                    });
                    ui.element()
                        .id("cancel_window_pick")
                        .height(fixed!(32.0))
                        .corner_radius(6.0)
                        .background_color(pal.surface)
                        .accessibility(|a| a.button("Cancel window selection"))
                        .layout(|l| l.align(CenterX, CenterY).padding((0, 10, 0, 10)))
                        .children(|ui| {
                            if ui.just_pressed() {
                                cancel_pick = true;
                            }
                            ui.text("Cancel", |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                            });
                        });
                });
            ui.element().height(fixed!(12.0)).empty();
        }
    }
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(10))
        .children(|ui| {
            for i in (1..count).chain(std::iter::once(0)) {
                let locked = i == 0;
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
                                        ui.text(&initial, |t| {
                                            t.font(&fonts::MONO_BOLD)
                                                .font_size(13)
                                                .color(if locked { pal.muted } else { pal.accent })
                                        });
                                    });
                                ui.element().width(grow!()).height(fit!()).children(|ui| {
                                    ui.text(&names[i], |t| {
                                        t.font(&fonts::DISPLAY_BOLD)
                                            .font_size(widgets::px(14.5))
                                            .color(pal.text)
                                    });
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
                                    if i > 1
                                        && profile_action(
                                            ui,
                                            app,
                                            ("p_up", i as u32),
                                            &icons::ARROW_UP,
                                        )
                                    {
                                        move_up = Some(i);
                                    }
                                    if i + 1 < count
                                        && profile_action(
                                            ui,
                                            app,
                                            ("p_down", i as u32),
                                            &icons::ARROW_DOWN,
                                        )
                                    {
                                        move_down = Some(i);
                                    }
                                    if profile_action(ui, app, ("p_del", i as u32), &icons::TRASH) {
                                        delete = Some(i);
                                    }
                                }
                            });

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
                        widgets::eyebrow(ui, &pal, "Apps using this profile");
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

                                if picking == Some(i) {
                                    return;
                                }
                                let id: Id = ("p_pick", i as u32).into();
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
                                            pick_for = Some(i);
                                        }
                                        widgets::icon(
                                            ui,
                                            &icons::TARGET,
                                            13.0,
                                            if hov { pal.accent } else { pal.muted },
                                        );
                                        ui.text("Pick a window", |t| {
                                            t.font(&fonts::SANS_SEMIBOLD)
                                                .font_size(12)
                                                .color(if hov { pal.accent } else { pal.muted })
                                        });
                                    });
                            });
                    });
            }
        });

    if cancel_pick {
        app.pick_deadline = None;
    }
    if add {
        app.draft.profiles.push(Profile {
            name: "New profile".to_string(),
            match_processes: Vec::new(),
            buttons: Default::default(),
        });
        app.commit_config();
    }
    if let Some(i) = delete {
        if i > 0 {
            app.draft.profiles.remove(i - 1);
            if app.pick_target == i {
                app.pick_deadline = None;
            }
            for index in [&mut app.selected_profile, &mut app.pick_target] {
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
        for index in [&mut app.selected_profile, &mut app.pick_target] {
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
        app.pick_deadline = Some(Instant::now() + Duration::from_secs(4));
    }
}

/// A small square icon button. Returns true when clicked.
fn profile_action(
    ui: &mut Ui2,
    app: &App,
    id: (&'static str, u32),
    glyph: &'static GraphicAsset,
) -> bool {
    let pal = app.pal;
    let label = match id.0 {
        "p_up" => "Move up",
        "p_down" => "Move down",
        _ => "Delete profile",
    };
    let id: Id = id.into();
    let mut clicked = false;
    let hov = ui.pointer_over(id.clone());
    ui.element()
        .id(id)
        .width(fit!())
        .accessibility(|a| a.button(label))
        .height(fixed!(30.0))
        .corner_radius(7.0)
        .background_color(if hov {
            pal.s2
        } else {
            super::theme::TRANSPARENT
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
            widgets::icon(ui, glyph, 15.0, pal.text);
            ui.text(label, |t| {
                t.font(&fonts::SANS_MEDIUM).font_size(12).color(pal.text)
            });
        });
    clicked
}
