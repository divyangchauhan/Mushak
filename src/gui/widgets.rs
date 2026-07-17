//! Shared components. The design has no stock widgets — every card, pill,
//! toggle and keycap here is built from Ply elements to match the handoff.

use super::theme::{Palette, TRANSPARENT, WHITE};
use super::{chrome, fonts, icons, App, Section};
use ply_engine::layout::Sizing;
use ply_engine::ElementBuilder;
use ply_engine::prelude::GraphicAsset;
use ply_engine::prelude::*;

/// Ply's `Ui` with the default (unused) custom-element payload.
pub type Ui2<'a> = ply_engine::Ui<'a, ()>;

/// The design uses fractional type sizes (12.5px, 13.5px); Ply's `font_size`
/// is a `u16`, so they round to the nearest pixel here.
pub fn px(size: f32) -> u16 {
    size.round().max(1.0) as u16
}

/// "Remapping paused" pill in the title bar.
pub fn paused_pill(ui: &mut Ui2, pal: &Palette) {
    let bg = Palette::mix(pal.danger, 15.0, pal.win);
    let border = Palette::mix(pal.danger, 35.0, pal.win);
    ui.element()
        .height(fixed!(22.0))
        .corner_radius(99.0)
        .background_color(bg)
        .border(|b| b.all(1).color(border))
        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(6).padding((3, 9, 3, 9)))
        .children(|ui| {
            ui.element()
                .width(fixed!(6.0))
                .height(fixed!(6.0))
                .corner_radius(3.0)
                .background_color(pal.danger)
                .empty();
            ui.text("Remapping paused", |t| {
                t.font(&fonts::SANS_SEMIBOLD)
                    .font_size(px(11.5))
                    .color(pal.danger)
            });
        });
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
    let bg = Palette::mix(pal.warn, 14.0, pal.win);
    let border = Palette::mix(pal.warn, 40.0, pal.win);
    let names = app.conflict_names.join(", ");
    let outcome = app.kill_outcome.clone();
    let mut quit = false;
    let mut ignore = false;

    ui.element()
        .width(grow!())
        .height(fit!())
        .background_color(bg)
        .border(|b| b.bottom(1).color(border))
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(13)
                .padding((12, 16, 12, 16))
        })
        .children(|ui| {
            icon(ui, &icons::WARN, 20.0, pal.warn);
            ui.element()
                .width(grow!())
                .height(fit!())
                .layout(|l| l.direction(TopToBottom).gap(2))
                .children(|ui| {
                    ui.text(&format!("{names} is running"), |t| {
                        t.font(&fonts::SANS_SEMIBOLD)
                            .font_size(px(13.5))
                            .color(pal.text)
                    });
                    // After a failed kill, say what actually happened rather
                    // than repeating the generic advice.
                    let sub = match &outcome {
                        Some(o) if !o.all_gone() => {
                            let (name, why) = &o.failed[0];
                            format!("Could not quit {name}: {why}")
                        }
                        _ => "It fights Mushak for control of the mouse. Quit it so your mappings \
                              take effect reliably."
                            .to_string(),
                    };
                    ui.text(&sub, |t| {
                        t.font(&fonts::SANS).font_size(px(12.5)).color(pal.muted)
                    });
                });

            let hovered = ui.pointer_over("banner_quit");
            ui.element()
                .id("banner_quit")
                .height(fixed!(30.0))
                .corner_radius(8.0)
                .background_color(if hovered {
                    Palette::mix(WHITE, 8.0, pal.warn)
                } else {
                    pal.warn
                })
                .layout(|l| l.align(CenterX, CenterY).padding((7, 13, 7, 13)))
                .children(|ui| {
                    if ui.just_pressed() {
                        quit = true;
                    }
                    ui.text("Quit Options+", |t| {
                        t.font(&fonts::SANS_SEMIBOLD)
                            .font_size(px(12.5))
                            .color(pal.warn_ink)
                    });
                });

            let ig_hover = ui.pointer_over("banner_ignore");
            ui.element()
                .id("banner_ignore")
                .height(fixed!(30.0))
                .layout(|l| l.align(CenterX, CenterY).padding((7, 6, 7, 6)))
                .children(|ui| {
                    if ui.just_pressed() {
                        ignore = true;
                    }
                    ui.text("Ignore", |t| {
                        t.font(&fonts::SANS)
                            .font_size(px(12.5))
                            .color(if ig_hover { pal.text } else { pal.muted })
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
pub fn nav_item(ui: &mut Ui2, pal: &Palette, section: Section, active: bool) -> bool {
    let mut clicked = false;
    let hovered = ui.pointer_over(section.id());
    let bg = if active {
        pal.surface
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
        .height(fixed!(38.0))
        .corner_radius(10.0)
        .background_color(bg)
        .border(|b| {
            b.all(1).color(if active {
                pal.line_strong
            } else {
                TRANSPARENT
            })
        })
        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(11).padding(12))
        .children(|ui| {
            if ui.just_pressed() {
                clicked = true;
            }
            icon(ui, section.icon(), 19.0, fg);
            ui.text(section.label(), |t| {
                t.font(font).font_size(px(13.5)).color(fg)
            });
        });
    clicked
}

/// Connection + battery card pinned to the bottom of the rail.
pub fn status_card(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let d = &app.device_view.device;
    let paused = app.device_view.paused;
    let awake = d.connected;

    let (dot, label) = if paused {
        (pal.danger, "Paused")
    } else if !awake {
        (pal.faint, "Asleep")
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
        Some(p) if awake => format!("{p}%"),
        _ => "—".to_string(),
    };
    let fill = pct.unwrap_or(0) as f32 / 100.0;
    let charging = d.charging;

    ui.element()
        .width(grow!())
        .height(fit!())
        .corner_radius(12.0)
        .background_color(pal.surface)
        .border(|b| b.all(1).color(pal.line))
        .layout(|l| l.direction(TopToBottom).gap(11).padding(12))
        .children(|ui| {
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
                t.font(&fonts::DISPLAY_BOLD).font_size(23).color(pal.text)
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
    ui.text(&text.to_uppercase(), |t| {
        t.font(&fonts::SANS_SEMIBOLD).font_size(12).color(pal.faint)
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
    let track_bg = if on { pal.accent } else { pal.s2 };
    let knob_bg = if on { pal.accent_ink } else { pal.muted };
    ui.element()
        .id(id)
        .width(fixed!(46.0))
        .height(fixed!(27.0))
        .corner_radius(99.0)
        .background_color(track_bg)
        .border(|b| b.all(1).color(if on { TRANSPARENT } else { pal.line_strong }))
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
    let mut out = None;
    let frac = ((value - min) as f32 / (max - min).max(1) as f32).clamp(0.0, 1.0);
    let active = ui.is_pressed(id);

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
                .background_color(pal.s2)
                .layout(|l| l.direction(LeftToRight).align(Left, CenterY))
                .children(|ui| {
                    // Filled portion.
                    ui.element()
                        .width(Sizing::Percent(frac.max(0.001)))
                        .height(fixed!(4.0))
                        .corner_radius(2.0)
                        .background_color(pal.accent)
                        .layout(|l| l.direction(LeftToRight).align(Right, CenterY))
                        .children(|ui| {
                            // Knob rides the end of the fill.
                            ui.element()
                                .width(fixed!(14.0))
                                .height(fixed!(14.0))
                                .corner_radius(99.0)
                                .background_color(pal.accent)
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
                    super::actions::Kind::PassThrough => (&fonts::SANS, pal.muted),
                    super::actions::Kind::Disabled => (&fonts::SANS_SEMIBOLD, pal.muted),
                    _ => (&fonts::SANS_SEMIBOLD, pal.text),
                };
                ui.text(&disp.label, |t| t.font(font).font_size(13).color(color));
            }
            icon(ui, &icons::CHEVRON_RIGHT, 14.0, pal.faint);
        });
    clicked
}

/// The "Mouse is asleep" card that replaces Scroll / Gestures / Device bodies
/// when the mouse cannot be reached.
pub fn asleep_card(ui: &mut Ui2, pal: &Palette) {
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(TopToBottom).align(CenterX, Top).padding((60, 0, 0, 0)))
        .children(|ui| {
            ui.element()
                .width(fixed!(320.0))
                .height(fit!())
                .corner_radius(16.0)
                .background_color(pal.surface)
                .border(|b| b.all(1).color(pal.line_strong))
                .layout(|l| {
                    l.direction(TopToBottom)
                        .align(CenterX, Top)
                        .gap(6)
                        .padding(28)
                })
                .children(|ui| {
                    ui.element()
                        .width(fixed!(44.0))
                        .height(fixed!(44.0))
                        .corner_radius(12.0)
                        .background_color(pal.s2)
                        .layout(|l| l.align(CenterX, CenterY))
                        .children(|ui| icon(ui, &icons::ASLEEP, 24.0, pal.muted));
                    ui.element().width(grow!()).height(fixed!(8.0)).empty();
                    ui.text("Mouse is asleep", |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(15).color(pal.text)
                    });
                    ui.text(
                        "Click or move it to wake it. Button mappings still work while it \
                         sleeps — scroll, gestures and live status don't.",
                        |t| t.font(&fonts::SANS).font_size(13).color(pal.muted),
                    );
                });
        });
}
