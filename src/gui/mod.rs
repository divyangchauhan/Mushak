//! Settings window (runs as the `--settings` subprocess). This process owns no
//! hooks, HID handle, or tray — it only edits `config.toml` and reads live
//! device status from the status file the resident process publishes. Keeping
//! the GPU-backed GUI out of the always-resident process is what keeps idle RAM
//! low.

mod actions;
mod buttons;
mod chrome;
mod device_tab;
mod fonts;
mod gestures_tab;
mod icons;
mod picker;
mod profiles_tab;
mod scroll;
mod theme;
mod uiprefs;
mod widgets;

use crate::config::{Config, Modifier};
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
    fn icon(self) -> &'static ply_engine::prelude::GraphicAsset {
        match self {
            Section::Buttons => &icons::NAV_BUTTONS,
            Section::Scroll => &icons::NAV_SCROLL,
            Section::Gestures => &icons::NAV_GESTURES,
            Section::Profiles => &icons::NAV_PROFILES,
            Section::Device => &icons::NAV_DEVICE,
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
    /// Buttons section: is the profile dropdown open?
    pub profile_menu_open: bool,
    /// Device section: is the HID++ feature list expanded?
    pub hidpp_expanded: bool,
    /// Action picker state. `None` when closed.
    pub picker: Option<PickerTarget>,
    pub picker_query: String,
    pub custom_mods: Vec<Modifier>,
    pub custom_key: Option<u16>,
    pub capturing_key: bool,
}

/// What the action picker is currently editing.
#[derive(Clone, PartialEq, Eq)]
pub enum PickerTarget {
    /// Slot name: back / forward / middle.
    Button(String),
    /// Slot name: tap / up / down / left / right.
    Gesture(String),
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
            profile_menu_open: false,
            hidpp_expanded: false,
            picker: None,
            picker_query: String::new(),
            custom_mods: Vec::new(),
            custom_key: None,
            capturing_key: false,
        }
    }

    pub fn profile_at(&self, idx: usize) -> Option<&crate::config::Profile> {
        if idx == 0 {
            Some(&self.draft.default_profile)
        } else {
            self.draft.profiles.get(idx - 1)
        }
    }

    /// Which profile row, if any, is waiting on a window pick.
    pub fn pick_target_index(&self) -> Option<usize> {
        self.pick_deadline.map(|_| self.pick_target)
    }

    pub fn pick_seconds_left(&self) -> u64 {
        self.pick_deadline
            .map(|d| d.saturating_duration_since(Instant::now()).as_secs() + 1)
            .unwrap_or(0)
    }

    pub fn open_picker(&mut self, target: PickerTarget) {
        // Seed the custom builder from the current binding, so reopening a
        // custom combo shows what it already is.
        let current = self.action_for(&target);
        match &current {
            crate::config::Action::Key { mods, vk } if actions::display(&current).kind == actions::Kind::Custom => {
                self.custom_mods = mods.clone();
                self.custom_key = Some(*vk);
            }
            _ => {
                self.custom_mods.clear();
                self.custom_key = None;
            }
        }
        self.picker = Some(target);
        self.picker_query.clear();
        self.capturing_key = false;
        self.profile_menu_open = false;
    }

    pub fn close_picker(&mut self) {
        self.picker = None;
        self.capturing_key = false;
    }

    /// The action currently bound to a picker target.
    pub fn action_for(&self, target: &PickerTarget) -> crate::config::Action {
        match target {
            PickerTarget::Button(slot) => self
                .profile_at(self.selected_profile)
                .map(|p| match slot.as_str() {
                    "back" => p.buttons.back.clone(),
                    "forward" => p.buttons.forward.clone(),
                    _ => p.buttons.middle.clone(),
                })
                .unwrap_or_default(),
            PickerTarget::Gesture(slot) => {
                let g = &self.draft.gestures;
                match slot.as_str() {
                    "tap" => g.tap.clone(),
                    "up" => g.up.clone(),
                    "down" => g.down.clone(),
                    "left" => g.left.clone(),
                    _ => g.right.clone(),
                }
            }
        }
    }

    pub fn assign_action(&mut self, target: &PickerTarget, action: crate::config::Action) {
        match target {
            PickerTarget::Button(slot) => {
                let idx = self.selected_profile;
                let slot = slot.clone();
                if let Some(p) = self.profile_at_mut(idx) {
                    match slot.as_str() {
                        "back" => p.buttons.back = action,
                        "forward" => p.buttons.forward = action,
                        _ => p.buttons.middle = action,
                    }
                }
            }
            PickerTarget::Gesture(slot) => {
                let g = &mut self.draft.gestures;
                match slot.as_str() {
                    "tap" => g.tap = action,
                    "up" => g.up = action,
                    "down" => g.down = action,
                    "left" => g.left = action,
                    _ => g.right = action,
                }
            }
        }
        self.commit_config();
    }

    /// While the picker is capturing, the next key press becomes the custom key.
    fn poll_key_capture(&mut self) {
        if !self.capturing_key {
            return;
        }
        for (code, vk) in KEY_MAP {
            if is_key_pressed(code) {
                self.custom_key = Some(vk);
                self.capturing_key = false;

                // Adopt the modifiers physically held at capture time, so that
                // pressing Ctrl+C records Ctrl+C. But only when something *was*
                // held: pressing a bare key must not wipe modifiers the user
                // picked with the chips beforehand.
                let mut held = Vec::new();
                if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
                    held.push(Modifier::Ctrl);
                }
                if is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt) {
                    held.push(Modifier::Alt);
                }
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    held.push(Modifier::Shift);
                }
                if is_key_down(KeyCode::LeftSuper) || is_key_down(KeyCode::RightSuper) {
                    held.push(Modifier::Win);
                }
                if !held.is_empty() {
                    self.custom_mods = held;
                }
                return;
            }
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

/// macroquad key -> Win32 virtual-key, for the picker's custom-combo capture.
/// Modifiers are deliberately absent: they are read separately so that holding
/// Ctrl and pressing C captures "Ctrl+C", not "Ctrl".
const KEY_MAP: [(KeyCode, u16); 44] = [
    (KeyCode::A, 0x41), (KeyCode::B, 0x42), (KeyCode::C, 0x43), (KeyCode::D, 0x44),
    (KeyCode::E, 0x45), (KeyCode::F, 0x46), (KeyCode::G, 0x47), (KeyCode::H, 0x48),
    (KeyCode::I, 0x49), (KeyCode::J, 0x4A), (KeyCode::K, 0x4B), (KeyCode::L, 0x4C),
    (KeyCode::M, 0x4D), (KeyCode::N, 0x4E), (KeyCode::O, 0x4F), (KeyCode::P, 0x50),
    (KeyCode::Q, 0x51), (KeyCode::R, 0x52), (KeyCode::S, 0x53), (KeyCode::T, 0x54),
    (KeyCode::U, 0x55), (KeyCode::V, 0x56), (KeyCode::W, 0x57), (KeyCode::X, 0x58),
    (KeyCode::Y, 0x59), (KeyCode::Z, 0x5A),
    (KeyCode::F1, 0x70), (KeyCode::F2, 0x71), (KeyCode::F3, 0x72), (KeyCode::F4, 0x73),
    (KeyCode::F5, 0x74), (KeyCode::F6, 0x75), (KeyCode::F7, 0x76), (KeyCode::F8, 0x77),
    (KeyCode::F9, 0x78), (KeyCode::F10, 0x79), (KeyCode::F11, 0x7A), (KeyCode::F12, 0x7B),
    (KeyCode::Left, 0x25), (KeyCode::Up, 0x26), (KeyCode::Right, 0x27), (KeyCode::Down, 0x28),
    (KeyCode::Tab, 0x09), (KeyCode::Space, 0x20),
];

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
        app.poll_key_capture();

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
            // Last, so it floats over everything.
            picker::overlay(ui, app);
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
            // Brand mark. Full-colour, so it is tinted white (a no-op multiply)
            // rather than taking a palette colour like the UI glyphs do.
            widgets::icon(
                ui,
                if paused {
                    &icons::MODAK_PAUSED
                } else {
                    &icons::MODAK_ACTIVE
                },
                20.0,
                theme::WHITE,
            );
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
                    Section::Buttons => buttons::section(ui, app),
                    Section::Scroll => scroll::section(ui, app),
                    Section::Gestures => gestures_tab::section(ui, app),
                    Section::Profiles => profiles_tab::section(ui, app),
                    Section::Device => device_tab::section(ui, app),
                });
        });
}
