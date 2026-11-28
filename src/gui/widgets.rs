//! Shared components. The design has no stock widgets — every card, pill,
//! toggle and keycap here is built from Ply elements to match the handoff.

use super::theme::{Palette, TRANSPARENT, WHITE};
use super::{chrome, fonts, icons, App, Section};
use ply_engine::layout::Sizing;
use ply_engine::prelude::GraphicAsset;
use ply_engine::prelude::*;
use ply_engine::ElementBuilder;

/// Ply's `Ui` with the default (unused) custom-element payload.
pub type Ui2<'a> = ply_engine::Ui<'a, ()>;

/// The design uses fractional type sizes (12.5px, 13.5px); Ply's `font_size`
/// is a `u16`, so they round to the nearest pixel here.
pub fn px(size: f32) -> u16 {
    size.round().max(1.0) as u16
}

/// "Remapping paused" pill in the title bar.
pub fn paused_status(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let pid = app.device_view.resume_pid;
    let failed = matches!(app.resume, super::ResumeState::Failed);
    let has_steps = failed || pid.is_none();
    let expanded = app.resume_steps_visible();
    let bg = Palette::mix(pal.danger, 15.0, pal.win);
    ui.element()
        .id("pause_state")
        .height(fixed!(28.0))
        .corner_radius(8.0)
        .background_color(bg)
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(8)
                .padding((2, 3, 2, 9))
        })
        .children(|ui| {
            ui.element()
                .width(fixed!(6.0))
                .height(fixed!(6.0))
                .corner_radius(3.0)
                .background_color(pal.danger)
                .empty();
            ui.text("Remapping paused", |t| {
                t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
            });
            resume_button(ui, app);
            if has_steps {
                let (id, label) = if expanded {
                    ("hide_resume_steps", "Close help")
                } else {
                    ("show_resume_steps", "Tray help")
                };
                if pause_command(ui, &pal, id, label) {
                    app.show_resume_steps = Some(!expanded);
                }
            }
        });
}

fn resume_button(ui: &mut Ui2, app: &mut App) {
    if let Some(pid) = app.device_view.resume_pid {
        let label = if matches!(app.resume, super::ResumeState::Failed) {
            "Retry"
        } else {
            "Resume"
        };
        if pause_command(ui, &app.pal, "resume_remapping", label) {
            if crate::single_instance::ResumeEvent::request(pid).is_ok() {
                app.resume = super::ResumeState::Requested {
                    pid,
                    at: std::time::Instant::now(),
                };
            } else {
                app.resume = super::ResumeState::Failed;
                app.show_resume_steps = Some(true);
            }
        }
    }
}

pub(super) fn resume_help(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let failed = matches!(app.resume, super::ResumeState::Failed);
    ui.element()
        .id("resume_help")
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(14))
        .children(|ui| {
            let back = format!("Back to {}", app.section.label());
            if pause_command(ui, &pal, "return_from_tray_help", &back) {
                app.show_resume_steps = Some(false);
            }
            heading(ui, &pal,
                if failed { "Resume failed" } else { "Resume from the tray" },
                if failed { "Mushak is still paused. Use Retry or resume remapping from the tray." }
                else { "Your saved button assignments will apply when remapping resumes." });
            card(ui, &pal)
                .layout(|l| l.direction(TopToBottom).gap(12).padding((16,18,16,18)))
                .children(|ui| {
                    for (step, instruction) in [
                        ("1", "Right-click Mushak by the Windows clock. Check hidden icons if needed."),
                        ("2", "Choose Resume remapping."),
                    ] {
                        ui.element().width(grow!()).height(fit!())
                            .layout(|l| l.direction(LeftToRight).gap(12).align(Left,CenterY))
                            .children(|ui| {
                                ui.element().width(fixed!(26.0)).height(fixed!(26.0)).corner_radius(13.0)
                                    .background_color(pal.s2)
                                    .layout(|l| l.align(CenterX, CenterY))
                                    .children(|ui| { ui.text(step, |t| t.font(&fonts::SANS_BOLD).font_size(14).color(pal.accent)); });
                                ui.element().width(grow!(0.0)).height(fit!()).children(|ui| {
                                    ui.text(instruction, |t| t.font(&fonts::SANS_MEDIUM).font_size(15).color(pal.text));
                                });
                            });
                    }
                });
        });
}

fn pause_command(ui: &mut Ui2, pal: &Palette, id: &'static str, label: &str) -> bool {
    let hovered = ui.pointer_over(id);
    let mut clicked = false;
    ui.element()
        .id(id)
        .width(if matches!(id, "hide_resume_steps" | "show_resume_steps") {
            fixed!(81.0)
        } else {
            fit!()
        })
        .height(fixed!(26.0))
        .corner_radius(6.0)
        .background_color(if hovered { pal.s2 } else { pal.surface })
        .border(|b| b.all(1).color(pal.line_strong))
        .accessibility(|a| a.button(label))
        .layout(|l| l.align(CenterX, CenterY).padding((0, 9, 0, 9)))
        .children(|ui| {
            ui.text(label, |t| {
                t.font(&fonts::SANS_BOLD).font_size(13).color(pal.text)
            });
            clicked = ui.just_pressed();
        });
    clicked
}

/// A tinted icon. Ply multiplies the texture by the element's background
/// colour, which is how the design's `currentColor` icons are reproduced.
pub fn icon(ui: &mut Ui2, asset: &'static GraphicAsset, size: f32, tint: Color) {
    ui.element()
        .width(fixed!(size))
        .height(fixed!(size))
        .background_color(tint)
        .image(asset)
        .empty();
}

/// Theme toggle, minimise, close-to-tray.
pub fn titlebar_buttons(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let mut toggle = false;
    let mut minimize = false;
    let mut close = false;

    let theme_icon: &'static GraphicAsset = match app.prefs.theme {
        super::Theme::Dark => &icons::MOON,
        super::Theme::Light => &icons::SUN,
    };

    for (id, glyph, hot) in [
        ("tb_theme", theme_icon, false),
        ("tb_min", &icons::MINIMIZE, false),
        ("tb_close", &icons::CLOSE, true),
    ] {
        let hovered = ui.pointer_over(id);
        let bg = if hovered && hot {
            pal.danger
        } else if hovered {
            pal.surface
        } else {
            TRANSPARENT
        };
        let fg = if hovered && hot { WHITE } else { pal.muted };
        ui.element()
            .id(id)
            .width(fixed!(34.0))
            .height(fixed!(30.0))
            .corner_radius(8.0)
            .background_color(bg)
            .layout(|l| l.align(CenterX, CenterY))
            .children(|ui| {
                if ui.just_pressed() {
                    match id {
                        "tb_theme" => toggle = true,
                        "tb_min" => minimize = true,
                        _ => close = true,
                    }
                }
                icon(ui, glyph, 13.0, fg);
            });
    }

    if toggle {
        app.toggle_theme();
    }
    if minimize {
        chrome::minimize();
    }
    if close {
        // This process *is* the settings window; the resident owns the tray and
        // keeps running.
        std::process::exit(0);
    }
}

/// Warning strip shown while Logitech Options+ / G HUB is running.
pub fn conflict_banner(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let names = app.conflict_names.join(", ");
    let failure = app.kill_outcome.as_ref().and_then(|o| o.failed.first());
    let tone = if failure.is_some() {
        pal.danger
    } else {
        pal.warn
    };
    let bg = Palette::mix(tone, 14.0, pal.win);
    let border = Palette::mix(tone, 40.0, pal.win);
    let mut quit = false;
    let mut ignore = false;

    ui.element()
        .width(grow!())
        .height(fit!())
        .background_color(bg)
        .border(|b| b.top(1).color(border))
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(13)
                .padding((12, 16, 12, 16))
        })
        .children(|ui| {
            icon(ui, &icons::WARN, 20.0, tone);
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(TopToBottom).gap(5))
                .children(|ui| {
                    let (title, sub) = match failure {
                        Some((name, why)) => (
                            format!("Could not quit {name}"),
                            format!("{why}. {name} can still intercept button presses. Quit it from its tray menu."),
                        ),
                        None => (
                            format!("{names} is running"),
                            "The other app can intercept button presses. Quit it, then use Resume if Mushak is paused.".into(),
                        ),
                    };
                    ui.element()
                        .width(grow!())
                        .height(fit!())
                        .layout(|l| l.direction(LeftToRight).gap(12).align(Left, CenterY).wrap())
                        .children(|ui| {
                            ui.text(&title, |t| {
                                t.font(&fonts::SANS_BOLD).font_size(15).color(pal.text)
                            });
                        });
                    ui.text(&sub, |t| {
                        t.font(&fonts::SANS).font_size(14).color(pal.muted)
                    });
                });

            let hovered = ui.pointer_over("banner_quit");
            ui.element()
                .id("banner_quit")
                .height(fixed!(30.0))
                .corner_radius(8.0)
                .background_color(if failure.is_some() {
                    pal.surface
                } else if hovered {
                    Palette::mix(WHITE, 8.0, pal.warn)
                } else {
                    pal.warn
                })
                .layout(|l| l.align(CenterX, CenterY).padding((7, 13, 7, 13)))
                .children(|ui| {
                    if ui.just_pressed() {
                        quit = true;
                    }
                    let command = if failure.is_some() {
                        "Retry quit".to_owned()
                    } else {
                        format!("Quit {names}")
                    };
                    ui.text(&command, |t| {
                        t.font(&fonts::SANS_SEMIBOLD)
                            .font_size(14)
                            .color(if failure.is_some() { pal.text } else { pal.warn_ink })
                    });
                });

            let ig_hover = ui.pointer_over("banner_ignore");
            ui.element()
                .id("banner_ignore")
                .corner_radius(8.0)
                .border(|b| b.all(1).color(pal.line_strong))
                .height(fixed!(30.0))
                .layout(|l| l.align(CenterX, CenterY).padding((7, 12, 7, 12)))
                .children(|ui| {
                    if ui.just_pressed() {
                        ignore = true;
                    }
                    ui.text("Dismiss", |t| {
                        t.font(&fonts::SANS).font_size(14).color(if ig_hover {
                            pal.text
                        } else {
                            pal.muted
                        })
                    });
                });
        });

    if quit {
        app.quit_options_plus();
    }
    if ignore {
        app.conflict_dismissed = true;
    }
}

/// One entry in the left rail. Returns true when clicked.
pub fn nav_item(ui: &mut Ui2, pal: &Palette, section: Section, active: bool, short: bool) -> bool {
    let padding_y = if short { 5 } else { 12 };
    let mut clicked = false;
    let hovered = ui.pointer_over(section.id());
    let bg = if active {
        Palette::mix(pal.accent, 14.0, pal.rail)
    } else if hovered {
        Palette::mix(pal.surface, 50.0, pal.win)
    } else {
        TRANSPARENT
    };
    let fg = if active { pal.text } else { pal.muted };
    let font = if active {
        &fonts::SANS_SEMIBOLD
    } else {
        &fonts::SANS_MEDIUM
    };
    ui.element()
        .id(section.id())
        .width(grow!())
        .height(fixed!(if short { 30.0 } else { 38.0 }))
        .corner_radius(10.0)
        .background_color(bg)
        .border(|b| {
            b.left(3)
                .color(if active { pal.accent } else { TRANSPARENT })
        })
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(11)
                .padding((padding_y, 12, padding_y, 12))
        })
        .children(|ui| {
            if ui.just_pressed() {
                clicked = true;
            }
            icon(
                ui,
                section.icon(),
                19.0,
                if active { pal.accent } else { fg },
            );
            ui.text(section.label(), |t| t.font(font).font_size(15).color(fg));
        });
    clicked
}

/// Connection + battery card pinned to the bottom of the rail.
pub fn status_card(ui: &mut Ui2, app: &mut App, short: bool) {
    let pal = app.pal;
    let d = &app.device_view.device;
    let awake = d.connected;

    let (dot, label) = if !awake {
        (pal.faint, d.availability_label())
    } else {
        (pal.good, "Connected")
    };

    let pct = d.battery_percent;
    let batt_color = match pct {
        Some(p) if p <= 20 => pal.danger,
        Some(p) if p <= 45 => pal.warn,
        _ => pal.good,
    };
    let batt_text = match pct {
        _ if awake => d.battery_label(),
        _ => "—".to_string(),
    };
    let fill = pct.unwrap_or(0) as f32 / 100.0;
    let charging = d.charging;

    ui.element()
        .id("rail_device_status")
        .width(grow!())
        .height(fit!())
        .corner_radius(12.0)
        .background_color(pal.surface)
        .border(|b| b.all(1).color(pal.line))
        .layout(|l| {
            l.direction(TopToBottom)
                .gap(if short { 3 } else { 7 })
                .padding(if short { 8 } else { 13 })
        })
        .children(|ui| {
            if d.name.is_some() {
                let model = d
                    .display_name()
                    .strip_prefix("Wireless Mouse ")
                    .filter(|s| !s.is_empty())
                    .unwrap_or(d.display_name());
                ui.text(model, |t| {
                    t.font(&fonts::SANS_BOLD).font_size(15).color(pal.text)
                });
            }
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(8))
                .children(|ui| {
                    ui.element()
                        .width(fixed!(7.0))
                        .height(fixed!(7.0))
                        .corner_radius(3.5)
                        .background_color(dot)
                        .empty();
                    ui.text(label, |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(12).color(pal.text)
                    });
                });
            if !d.capabilities.battery {
                return;
            }
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(9))
                .children(|ui| {
                    // Battery shell + fill.
                    ui.element()
                        .width(fixed!(34.0))
                        .height(fixed!(17.0))
                        .corner_radius(4.0)
                        .border(|b| b.all(2).color(pal.muted))
                        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).padding(2))
                        .children(|ui| {
                            if pct.is_none() {
                                return;
                            }
                            ui.element()
                                .width(Sizing::Percent(fill.max(0.02)))
                                .height(grow!())
                                .corner_radius(2.0)
                                .background_color(batt_color)
                                .empty();
                        });
                    ui.text(&batt_text, |t| {
                        t.font(&fonts::MONO_SEMIBOLD).font_size(13).color(pal.text)
                    });
                    if charging {
                        icon(ui, &icons::BOLT, 12.0, pal.good);
                    }
                });
        });
}

/// Section heading + one line of explanation.
pub fn heading(ui: &mut Ui2, pal: &Palette, title: &str, sub: &str) {
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).gap(5))
        .children(|ui| {
            ui.text(title, |t| {
                t.font(&fonts::DISPLAY_BOLD).font_size(30).color(pal.text)
            });
            if !sub.is_empty() {
                ui.text(sub, |t| {
                    t.font(&fonts::SANS).font_size(px(13.5)).color(pal.muted)
                });
            }
        });
}

/// Small uppercase label the design uses above groups of controls.
pub fn eyebrow(ui: &mut Ui2, pal: &Palette, text: &str) {
    ui.text(text, |t| {
        t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.muted)
    });
}

/// The standard 13px-radius card the whole design is built from.
pub fn card<'a>(ui: &'a mut Ui2, pal: &Palette) -> ElementBuilder<'a, ()> {
    ui.element()
        .width(grow!())
        .height(fit!())
        .corner_radius(13.0)
        .background_color(pal.surface)
        .border(|b| b.all(1).color(pal.line))
}

/// A toggle switch. Returns true when clicked.
pub fn toggle(ui: &mut Ui2, pal: &Palette, id: &'static str, on: bool) -> bool {
    let mut clicked = false;
    // The knob slides by (track - knob - 2*inset) = 46 - 21 - 4.
    let track_bg = if on { pal.accent } else { pal.track };
    let knob_bg = if on { pal.accent_ink } else { pal.surface };
    ui.element()
        .id(id)
        .width(fixed!(46.0))
        .height(fixed!(27.0))
        .corner_radius(99.0)
        .background_color(track_bg)
        .border(|b| {
            b.all(1)
                .color(if on { TRANSPARENT } else { pal.line_strong })
        })
        .layout(|l| {
            l.direction(LeftToRight)
                .align(if on { Right } else { Left }, CenterY)
                .padding(2)
        })
        .children(|ui| {
            if ui.just_pressed() {
                clicked = true;
            }
            ui.element()
                .width(fixed!(21.0))
                .height(fixed!(21.0))
                .corner_radius(99.0)
                .background_color(knob_bg)
                .empty();
        });
    clicked
}

/// A horizontal slider. Returns Some(new value) while dragging.
///
/// Ply has no range widget, so this is a track the pointer is projected onto:
/// on press or drag, the x offset within the track maps back to the value.
pub fn slider(
    ui: &mut Ui2,
    pal: &Palette,
    id: &'static str,
    value: i64,
    min: i64,
    max: i64,
    step: i64,
) -> Option<i64> {
    slider_enabled(ui, pal, id, value, min..=max, step, true)
}

pub fn slider_enabled(
    ui: &mut Ui2,
    pal: &Palette,
    id: &'static str,
    value: i64,
    range: std::ops::RangeInclusive<i64>,
    step: i64,
    enabled: bool,
) -> Option<i64> {
    let (min, max) = range.into_inner();
    let ink = if enabled { pal.accent } else { pal.line_strong };
    let mut out = None;
    let frac = ((value - min) as f32 / (max - min).max(1) as f32).clamp(0.0, 1.0);
    let active = enabled && ui.is_pressed(id);

    // Pointer x within the track, resolved from last frame's geometry.
    if active {
        if let Some(bb) = ui.bounding_box(id) {
            // mouse_position() is in the same logical space as screen_width(),
            // which is what Ply lays out in — no DPI conversion needed.
            let t = ((mouse_position().0 - bb.x) / bb.width.max(1.0)).clamp(0.0, 1.0);
            let raw = min as f32 + t * (max - min) as f32;
            let snapped = (raw / step as f32).round() as i64 * step;
            let snapped = snapped.clamp(min, max);
            if snapped != value {
                out = Some(snapped);
            }
        }
    }

    ui.element()
        .id(id)
        .width(grow!())
        .height(fixed!(20.0))
        .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
        .children(|ui| {
            // Track.
            ui.element()
                .width(grow!())
                .height(fixed!(4.0))
                .corner_radius(2.0)
                .background_color(pal.track)
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                .children(|ui| {
                    // Filled portion.
                    ui.element()
                        .width(Sizing::Percent(frac.max(0.001)))
                        .height(fixed!(4.0))
                        .corner_radius(2.0)
                        .background_color(ink)
                        .layout(|l| l.direction(LeftToRight).align(Right, CenterY))
                        .children(|ui| {
                            // Knob rides the end of the fill.
                            ui.element()
                                .width(fixed!(if enabled { 14.0 } else { 3.0 }))
                                .height(fixed!(if enabled { 14.0 } else { 8.0 }))
                                .corner_radius(if enabled { 7.0 } else { 1.0 })
                                .background_color(ink)
                                .empty();
                        });
                });
        });
    out
}

/// One key of a shortcut, e.g. `Ctrl` or `C`.
pub fn keycap(ui: &mut Ui2, pal: &Palette, label: &str) {
    ui.element()
        .height(fixed!(22.0))
        .corner_radius(6.0)
        .background_color(pal.keycap)
        .border(|b| b.all(1).color(pal.line_strong))
        .layout(|l| l.align(CenterX, CenterY).padding((0, 6, 0, 6)))
        .children(|ui| {
            ui.text(label, |t| {
                t.font(&fonts::MONO_SEMIBOLD)
                    .font_size(px(11.5))
                    .color(pal.text)
            });
        });
}

/// The clickable chip showing what an action is bound to. Returns true when
/// clicked (which opens the action picker).
pub fn action_chip(ui: &mut Ui2, pal: &Palette, id: Id, disp: &super::actions::Display) -> bool {
    let mut clicked = false;
    let hovered = ui.pointer_over(id.clone());
    ui.element()
        .id(id)
        .height(fixed!(36.0))
        .corner_radius(9.0)
        .background_color(pal.s2)
        .border(|b| {
            b.all(1)
                .color(if hovered { pal.accent } else { pal.line_strong })
        })
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(6)
                .padding((7, 11, 7, 11))
        })
        .children(|ui| {
            if ui.just_pressed() {
                clicked = true;
            }
            for cap in &disp.keys {
                keycap(ui, pal, cap);
            }
            if !disp.label.is_empty() {
                let (font, color) = match disp.kind {
                    super::actions::Kind::PassThrough => (&fonts::SANS_SEMIBOLD, pal.text),
                    super::actions::Kind::Disabled => (&fonts::SANS_SEMIBOLD, pal.muted),
                    _ => (&fonts::SANS_SEMIBOLD, pal.text),
                };
                ui.text(&disp.label, |t| t.font(font).font_size(13).color(color));
            }
            icon(ui, &icons::CAT_EDIT, 16.0, pal.accent);
        });
    clicked
}

/// The "Mouse is asleep" card that replaces Scroll / Gestures / Device bodies
/// when the mouse cannot be reached.
pub fn asleep_card(ui: &mut Ui2, pal: &Palette, device: &crate::hidpp::device::DeviceStatus) {
    card(ui, pal)
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(18)
                .padding(24)
        })
        .children(|ui| {
            ui.element()
                .width(fixed!(52.0))
                .height(fixed!(52.0))
                .corner_radius(12.0)
                .background_color(pal.s2)
                .layout(|l| l.align(CenterX, CenterY))
                .children(|ui| icon(ui, &icons::ASLEEP, 26.0, pal.muted));
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(TopToBottom).gap(7))
                .children(|ui| {
                    ui.text(device.availability_label(), |t| {
                        t.font(&fonts::DISPLAY_BOLD).font_size(20).color(pal.text)
                    });
                    let instruction = if device.name.is_some() {
                        "Wake or reconnect your mouse. Saved button mappings are still editable."
                    } else {
                        "Edit standard button mappings offline, or connect a mouse to see its controls."
                    };
                    ui.text(instruction, |t| {
                        t.font(&fonts::SANS).font_size(14).color(pal.muted)
                    });
                });
        });
}
