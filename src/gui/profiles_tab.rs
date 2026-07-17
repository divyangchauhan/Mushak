//! Profiles section: manage per-application profiles.
//!
//! Config keeps the default profile separate from the list; the design shows it
//! as the first row, locked, with a "Fallback" badge and no match list.

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
                    "Mushak watches the active window and picks the first profile that \
                     matches. First match wins — move the important ones up.",
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
                        t.font(&fonts::SANS_BOLD).font_size(13).color(pal.accent_ink)
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

    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(10))
        .children(|ui| {
            for i in 0..count {
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
                                            t.font(&fonts::MONO_BOLD).font_size(13).color(
                                                if locked { pal.muted } else { pal.accent },
                                            )
                                        });
                                    });
                                ui.element()
                                    .width(grow!())
                                    .height(fit!())
                                    .children(|ui| {
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
                                            ui.text("Fallback", |t| {
                                                t.font(&fonts::SANS_SEMIBOLD)
                                                    .font_size(11)
                                                    .color(pal.faint)
                                            });
                                        });
                                } else {
                                    if icon_button(ui, app, ("p_up", i as u32), &icons::ARROW_UP, i <= 1) {
                                        move_up = Some(i);
                                    }
                                    if icon_button(
                                        ui,
                                        app,
                                        ("p_down", i as u32),
                                        &icons::ARROW_DOWN,
                                        i + 1 >= count,
                                    ) {
                                        move_down = Some(i);
                                    }
                                    if icon_button(ui, app, ("p_del", i as u32), &icons::TRASH, false) {
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
                        widgets::eyebrow(ui, &pal, "Matches these apps");
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
                                            let id: Id =
                                                ("m_del", (i * 100 + j) as u32).into();
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
                                                        if hov { super::theme::WHITE } else { pal.faint },
                                                    );
                                                });
                                        });
                                }

                                if picking == Some(i) {
                                    ui.element()
                                        .height(fixed!(28.0))
                                        .corner_radius(8.0)
                                        .background_color(super::theme::Palette::mix(
                                            pal.accent, 16.0, pal.surface,
                                        ))
                                        .border(|b| b.all(1).color(pal.accent))
                                        .layout(|l| {
                                            l.direction(LeftToRight)
                                                .align(Left, CenterY)
                                                .gap(8)
                                                .padding((0, 12, 0, 12))
                                        })
                                        .children(|ui| {
                                            ui.element()
                                                .width(fixed!(8.0))
                                                .height(fixed!(8.0))
                                                .corner_radius(99.0)
                                                .background_color(pal.accent)
                                                .empty();
                                            ui.text(
                                                &format!("Click a window… {secs_left}s"),
                                                |t| {
                                                    t.font(&fonts::SANS_SEMIBOLD)
                                                        .font_size(12)
                                                        .color(pal.accent)
                                                },
                                            );
                                        });
                                }

                                let id: Id = ("p_pick", i as u32).into();
                                let hov = ui.pointer_over(id.clone());
                                ui.element()
                                    .id(id)
                                    .height(fixed!(28.0))
                                    .corner_radius(8.0)
                                    .border(|b| {
                                        b.all(1)
                                            .color(if hov { pal.accent } else { pal.line_strong })
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
                                            t.font(&fonts::SANS_SEMIBOLD).font_size(12).color(
                                                if hov { pal.accent } else { pal.muted },
                                            )
                                        });
                                    });
                            });
                    });
            }
        });

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
            if app.selected_profile >= app.profile_names().len() {
                app.selected_profile = 0;
            }
            app.commit_config();
        }
    }
    if let Some(i) = move_up {
        if i > 1 {
            app.draft.profiles.swap(i - 1, i - 2);
            app.commit_config();
        }
    }
    if let Some(i) = move_down {
        if i > 0 && i < app.draft.profiles.len() {
            app.draft.profiles.swap(i - 1, i);
            app.commit_config();
        }
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

/// A small square icon button. Returns true when clicked (and not disabled).
fn icon_button(
    ui: &mut Ui2,
    app: &App,
    id: (&'static str, u32),
    glyph: &'static GraphicAsset,
    disabled: bool,
) -> bool {
    let pal = app.pal;
    let id: Id = id.into();
    let mut clicked = false;
    let hov = !disabled && ui.pointer_over(id.clone());
    ui.element()
        .id(id)
        .width(fixed!(30.0))
        .height(fixed!(30.0))
        .corner_radius(7.0)
        .background_color(if hov { pal.s2 } else { super::theme::TRANSPARENT })
        .layout(|l| l.align(CenterX, CenterY))
        .children(|ui| {
            if ui.just_pressed() && !disabled {
                clicked = true;
            }
            widgets::icon(
                ui,
                glyph,
                15.0,
                if disabled { pal.faint } else { pal.muted },
            );
        });
    clicked
}
