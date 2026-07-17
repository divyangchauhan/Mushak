//! Settings window (runs as the `--settings` subprocess). This process owns no
//! hooks, HID handle, or tray — it only edits `config.toml` and reads live
//! device status from the status file the resident process publishes. Keeping
//! the GPU-backed GUI out of the always-resident process is what keeps idle RAM
//! low.

mod chrome;
mod fonts;
mod theme;
mod uiprefs;
mod widgets;

use crate::config::Config;
use crate::statusfile::{self, SharedStatus};
use crate::{conflicts, state};
use conflicts::KillOutcome;
use ply_engine::prelude::*;
use std::time::{Duration, Instant};
use theme::{Palette, Theme};
use uiprefs::UiPrefs;
use widgets::Ui2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Buttons,
    Scroll,
    Gestures,
    Profiles,
    Device,
}

impl Section {
    const ALL: [Section; 5] = [
        Section::Buttons,
        Section::Scroll,
        Section::Gestures,
        Section::Profiles,
        Section::Device,
    ];
    fn label(self) -> &'static str {
        match self {
            Section::Buttons => "Buttons",
            Section::Scroll => "Scroll",
            Section::Gestures => "Gestures",
            Section::Profiles => "Profiles",
            Section::Device => "Device",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Section::Buttons => "nav_buttons",
            Section::Scroll => "nav_scroll",
            Section::Gestures => "nav_gestures",
            Section::Profiles => "nav_profiles",
            Section::Device => "nav_device",
        }
    }
}

pub struct App {
    pub section: Section,
    /// Editable working copy, persisted to config.toml on change.
    pub draft: Config,
    pub prefs: UiPrefs,
    pub pal: Palette,
    /// Selected profile for the Buttons section (0 = default, 1.. = profiles[n-1]).
    pub selected_profile: usize,
    /// "Pick a running window" countdown + target profile index.
    pub pick_deadline: Option<Instant>,
    pub pick_target: usize,
    own_exe: String,
    /// Cached conflicting-driver scan.
    pub conflict_names: Vec<String>,
    pub conflict_dismissed: bool,
    /// Result of the last "Quit Options+" press, shown in the banner.
    pub kill_outcome: Option<KillOutcome>,
    conflict_next_check: Instant,
    /// Live device status read from the resident process.
    pub device_view: SharedStatus,
    status_next_check: Instant,
    /// The OS title bar is stripped after the first frame, once the window exists.
    framed: bool,
}

impl App {
    fn new() -> Self {
        let own_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
            .unwrap_or_default();
        let prefs = UiPrefs::load();
        App {
            section: Section::Buttons,
            draft: (*state::config()).clone(),
            prefs,
            pal: Palette::resolve(prefs.theme, prefs.accent),
            selected_profile: 0,
            pick_deadline: None,
            pick_target: 0,
            own_exe,
            conflict_names: conflicts::detect(),
            conflict_dismissed: false,
            kill_outcome: None,
            conflict_next_check: Instant::now() + Duration::from_secs(3),
            device_view: statusfile::read(),
            status_next_check: Instant::now() + Duration::from_millis(500),
            framed: false,
        }
    }

    /// Persist the draft. The resident process notices the file change and
    /// applies any device-affecting settings.
    pub fn commit_config(&mut self) {
        if let Err(e) = self.draft.save() {
            tracing::error!("saving config failed: {e:#}");
        }
        state::set_config(self.draft.clone());
    }

    pub fn toggle_theme(&mut self) {
        self.prefs.theme = self.prefs.theme.toggled();
        self.pal = Palette::resolve(self.prefs.theme, self.prefs.accent);
        self.prefs.save();
    }

    /// True while the mouse is reachable. Scroll / Gestures / Device all read
    /// live device values, so they dim to an "asleep" card without it.
    pub fn mouse_awake(&self) -> bool {
        self.device_view.device.connected
    }

    pub fn show_conflict(&self) -> bool {
        !self.conflict_names.is_empty() && !self.conflict_dismissed
    }

    fn poll_timers(&mut self) {
        let now = Instant::now();
        if now >= self.conflict_next_check {
            self.conflict_names = conflicts::detect();
            if self.conflict_names.is_empty() {
                // Nothing to warn about any more; a later relaunch of Options+
                // should raise a fresh banner rather than stay dismissed.
                self.conflict_dismissed = false;
                self.kill_outcome = None;
            }
            self.conflict_next_check = now + Duration::from_secs(3);
        }
        if now >= self.status_next_check {
            self.device_view = statusfile::read();
            self.status_next_check = now + Duration::from_millis(500);
        }
        if let Some(deadline) = self.pick_deadline {
            if now >= deadline {
                self.pick_deadline = None;
                self.capture_foreground_window();
            }
        }
    }

    fn capture_foreground_window(&mut self) {
        let Some(proc) = crate::profiles::foreground_process_name() else {
            return;
        };
        if proc.eq_ignore_ascii_case(&self.own_exe) {
            return;
        }
        let target = self.pick_target;
        if let Some(profile) = self.profile_at_mut(target) {
            if !profile
                .match_processes
                .iter()
                .any(|p| p.eq_ignore_ascii_case(&proc))
            {
                profile.match_processes.push(proc);
                self.commit_config();
            }
        }
    }

    pub fn profile_at_mut(&mut self, idx: usize) -> Option<&mut crate::config::Profile> {
        if idx == 0 {
            Some(&mut self.draft.default_profile)
        } else {
            self.draft.profiles.get_mut(idx - 1)
        }
    }

    pub fn profile_names(&self) -> Vec<String> {
        let mut names = vec![self.draft.default_profile.name.clone()];
        for p in &self.draft.profiles {
            names.push(p.name.clone());
        }
        names
    }

    pub fn quit_options_plus(&mut self) {
        let outcome = conflicts::kill_all();
        tracing::info!(
            "quit Options+: {} killed, {} failed",
            outcome.killed,
            outcome.failed.len()
        );
        if outcome.all_gone() {
            self.conflict_names.clear();
        }
        self.kill_outcome = Some(outcome);
        // Re-scan promptly rather than waiting out the 3s cadence.
        self.conflict_next_check = Instant::now() + Duration::from_millis(400);
    }
}

/// The design's window size, in logical points.
pub const DESIGN_W: f32 = 968.0;
pub const DESIGN_H: f32 = 668.0;

/// Entry point for the `--settings` subprocess.
pub fn run() {
    let conf = macroquad::conf::Conf {
        miniquad_conf: miniquad::conf::Conf {
            window_title: "Mushak Settings".to_string(),
            window_width: 968,
            window_height: 668,
            high_dpi: true,
            window_resizable: true,
            platform: miniquad::conf::Platform {
                // Idle at ~zero CPU: only redraw when something happens. The
                // frame loop calls schedule_update() while anything is
                // animating or counting down.
                blocking_event_loop: true,
                ..Default::default()
            },
            ..Default::default()
        },
        update_on: Some(macroquad::conf::UpdateTrigger {
            key_down: true,
            mouse_down: true,
            mouse_up: true,
            mouse_motion: true,
            mouse_wheel: true,
            touch: true,
            specific_key: None,
        }),
        ..Default::default()
    };
    macroquad::Window::from_config(conf, frame_loop());
}

async fn frame_loop() {
    let mut ply = Ply::<()>::new(&fonts::SANS).await;
    let mut app = App::new();

    loop {
        app.poll_timers();

        clear_background(app.pal.win.into());
        let mut ui = ply.begin();
        window(&mut ui, &mut app);
        ui.show(|_| {}).await;

        if !app.framed {
            // miniquad has created the window by the time the first frame is
            // presented, so the HWND is live now.
            chrome::make_frameless(DESIGN_W, DESIGN_H);
            app.framed = true;
        }

        // blocking_event_loop parks the loop until input arrives; anything
        // time-driven has to ask for its own frame.
        if app.pick_deadline.is_some() {
            miniquad::window::schedule_update();
        }

        next_frame().await;
    }
}

/// The whole window: title bar, optional conflict banner, then rail + content.
fn window(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    ui.element()
        .width(grow!())
        .height(grow!())
        .background_color(pal.win)
        .layout(|l| l.direction(TopToBottom))
        .children(|ui| {
            titlebar(ui, app);
            if app.show_conflict() {
                widgets::conflict_banner(ui, app);
            }
            ui.element()
                .width(grow!())
                .height(grow!())
                .layout(|l| l.direction(LeftToRight))
                .children(|ui| {
                    rail(ui, app);
                    content(ui, app);
                });
        });
}

fn titlebar(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let paused = app.device_view.paused;
    ui.element()
        .id("titlebar")
        .width(grow!())
        .height(fixed!(46.0))
        .layout(|l| {
            l.direction(LeftToRight)
                .align(Left, CenterY)
                .gap(11)
                .padding((0, 10, 0, 15))
        })
        .border(|b| b.bottom(1).color(pal.line))
        .children(|ui| {
            // Dragging the title bar drags the window.
            if ui.just_pressed() {
                chrome::begin_drag();
            }
            ui.text("Mushak", |t| {
                t.font(&fonts::DISPLAY_BOLD).font_size(14).color(pal.text)
            });
            ui.element()
                .width(fixed!(1.0))
                .height(fixed!(15.0))
                .background_color(pal.line_strong)
                .empty();
            ui.text("MX Master 2S", |t| {
                t.font(&fonts::SANS).font_size(13).color(pal.muted)
            });
            if paused {
                widgets::paused_pill(ui, &pal);
            }
            // Push the window controls to the right.
            ui.element().width(grow!()).height(fixed!(1.0)).empty();
            widgets::titlebar_buttons(ui, app);
        });
}

fn rail(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let current = app.section;
    let mut clicked: Option<Section> = None;
    ui.element()
        .width(fixed!(226.0))
        .height(grow!())
        .layout(|l| l.direction(TopToBottom).gap(3).padding(12))
        .border(|b| b.right(1).color(pal.line))
        .children(|ui| {
            for s in Section::ALL {
                if widgets::nav_item(ui, &pal, s, s == current) {
                    clicked = Some(s);
                }
            }
            // Status card sits at the bottom of the rail.
            ui.element().width(grow!()).height(grow!()).empty();
            widgets::status_card(ui, app);
        });
    if let Some(s) = clicked {
        app.section = s;
    }
}

fn content(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    ui.element()
        .width(grow!())
        .height(grow!())
        .overflow(|o| {
            o.scroll_y().scrollbar(|s| {
                s.width(11.0)
                    .corner_radius(6.0)
                    .thumb_color(pal.line_strong)
                    .track_color(pal.win)
            })
        })
        .children(|ui| {
            ui.element()
                .width(grow!(0.0, 660.0))
                .height(fit!())
                .layout(|l| l.direction(TopToBottom).padding((26, 30, 40, 30)))
                .children(|ui| match app.section {
                    Section::Buttons => widgets::placeholder(ui, app, "Buttons"),
                    Section::Scroll => widgets::placeholder(ui, app, "Scroll"),
                    Section::Gestures => widgets::placeholder(ui, app, "Gestures"),
                    Section::Profiles => widgets::placeholder(ui, app, "Profiles"),
                    Section::Device => widgets::placeholder(ui, app, "Device"),
                });
        });
}
