//! Settings window (runs as the `--settings` subprocess). This process owns no
//! hooks, HID handle, or tray — it only edits `config.toml` and reads live
//! device status from the status file the resident process publishes. Keeping
//! the GPU-backed GUI out of the always-resident process is what keeps idle RAM
//! low.

mod buttons;
mod device_tab;
mod gestures_tab;
mod profiles_tab;
mod scroll;

use crate::config::{Action, Config, Modifier};
use crate::statusfile::{self, SharedStatus};
use crate::{conflicts, profiles, state};
use eframe::egui;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Buttons,
    Scroll,
    Gestures,
    Profiles,
    Device,
}

pub struct App {
    tab: Tab,
    /// Editable working copy, persisted to config.toml on change.
    draft: Config,
    /// Selected profile for the Buttons tab (0 = default, 1.. = profiles[n-1]).
    selected_profile: usize,
    /// "Pick a running window" countdown + target profile index.
    pick_deadline: Option<Instant>,
    pick_target: usize,
    own_exe: String,
    /// Cached conflicting-driver scan.
    conflict_names: Vec<String>,
    conflict_next_check: Instant,
    /// Live device status read from the resident process.
    device_view: SharedStatus,
    status_next_check: Instant,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let own_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
            .unwrap_or_default();
        App {
            tab: Tab::Buttons,
            draft: (*state::config()).clone(),
            selected_profile: 0,
            pick_deadline: None,
            pick_target: 0,
            own_exe,
            conflict_names: conflicts::detect(),
            conflict_next_check: Instant::now() + Duration::from_secs(3),
            device_view: statusfile::read(),
            status_next_check: Instant::now() + Duration::from_millis(500),
        }
    }

    /// Persist the draft. The resident process notices the file change and
    /// applies any device-affecting settings.
    pub(crate) fn commit_config(&mut self) {
        if let Err(e) = self.draft.save() {
            tracing::error!("saving config failed: {e:#}");
        }
        state::set_config(self.draft.clone());
    }

    // The tabs distinguish device/gesture edits, but in this process all commits
    // are just a config write; the resident re-applies.
    pub(crate) fn commit_and_apply_device(&mut self) {
        self.commit_config();
    }
    pub(crate) fn commit_and_apply_gesture(&mut self) {
        self.commit_config();
    }

    fn poll_window_pick(&mut self) {
        if let Some(deadline) = self.pick_deadline {
            if Instant::now() >= deadline {
                self.pick_deadline = None;
                if let Some(proc) = profiles::foreground_process_name() {
                    if proc.eq_ignore_ascii_case(&self.own_exe) {
                        return;
                    }
                    if let Some(profile) = self.profile_at_mut(self.pick_target) {
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
            }
        }
    }

    fn profile_at_mut(&mut self, idx: usize) -> Option<&mut crate::config::Profile> {
        if idx == 0 {
            Some(&mut self.draft.default_profile)
        } else {
            self.draft.profiles.get_mut(idx - 1)
        }
    }

    fn profile_names(&self) -> Vec<String> {
        let mut names = vec![self.draft.default_profile.name.clone()];
        for p in &self.draft.profiles {
            names.push(p.name.clone());
        }
        names
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_window_pick();

        let now = Instant::now();
        if now >= self.conflict_next_check {
            self.conflict_names = conflicts::detect();
            self.conflict_next_check = now + Duration::from_secs(3);
        }
        if now >= self.status_next_check {
            self.device_view = statusfile::read();
            self.status_next_check = now + Duration::from_millis(500);
        }

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Buttons, "Buttons");
                ui.selectable_value(&mut self.tab, Tab::Scroll, "Scroll");
                ui.selectable_value(&mut self.tab, Tab::Gestures, "Gestures");
                ui.selectable_value(&mut self.tab, Tab::Profiles, "Profiles");
                ui.selectable_value(&mut self.tab, Tab::Device, "Device");
            });
            ui.add_space(4.0);
        });

        if !self.conflict_names.is_empty() {
            egui::TopBottomPanel::top("conflict").show(ctx, |ui| {
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(230, 130, 40),
                        format!(
                            "⚠ Another Logitech driver is running ({}). It will fight Mushak over \
                             the device — quit it for gestures / SmartShift / DPI to work reliably.",
                            self.conflict_names.join(", ")
                        ),
                    );
                });
                ui.add_space(3.0);
            });
        }

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let d = &self.device_view.device;
                let dot = if d.connected { "●" } else { "○" };
                ui.label(format!(
                    "{dot} {}",
                    d.connection.as_deref().unwrap_or("disconnected")
                ));
                if let Some(p) = d.battery_percent {
                    ui.separator();
                    ui.label(format!("{p}%{}", if d.charging { " (charging)" } else { "" }));
                }
                if self.device_view.paused {
                    ui.separator();
                    ui.colored_label(egui::Color32::YELLOW, "remapping paused");
                }
            });
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                Tab::Buttons => self.buttons_tab(ui),
                Tab::Scroll => self.scroll_tab(ui),
                Tab::Gestures => self.gestures_tab(ui),
                Tab::Profiles => self.profiles_tab(ui),
                Tab::Device => self.device_tab(ui),
            });
        });

        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

// ---- Shared action-editor widget --------------------------------------------

fn presets() -> Vec<(&'static str, Action)> {
    use crate::config::vk;
    use Modifier::{Alt, Ctrl, Shift, Win};
    vec![
        ("Pass through", Action::PassThrough),
        ("Disabled", Action::Disabled),
        ("Copy (Ctrl+C)", Action::key(&[Ctrl], vk::letter(b'C'))),
        ("Paste (Ctrl+V)", Action::key(&[Ctrl], vk::letter(b'V'))),
        ("Cut (Ctrl+X)", Action::key(&[Ctrl], vk::letter(b'X'))),
        ("Undo (Ctrl+Z)", Action::key(&[Ctrl], vk::letter(b'Z'))),
        ("Redo (Ctrl+Y)", Action::key(&[Ctrl], vk::letter(b'Y'))),
        ("Close tab (Ctrl+W)", Action::key(&[Ctrl], vk::letter(b'W'))),
        ("New (Ctrl+N)", Action::key(&[Ctrl], vk::letter(b'N'))),
        ("New tab (Ctrl+T)", Action::key(&[Ctrl], vk::letter(b'T'))),
        ("Reopen tab (Ctrl+Shift+T)", Action::key(&[Ctrl, Shift], vk::letter(b'T'))),
        ("Find (Ctrl+F)", Action::key(&[Ctrl], vk::letter(b'F'))),
        ("Back (Alt+Left)", Action::key(&[Alt], vk::LEFT)),
        ("Forward (Alt+Right)", Action::key(&[Alt], vk::RIGHT)),
        ("Browser back", Action::key(&[], vk::BROWSER_BACK)),
        ("Browser forward", Action::key(&[], vk::BROWSER_FORWARD)),
        ("Task View (Win+Tab)", Action::key(&[Win], vk::TAB)),
        ("Desktop left (Win+Ctrl+Left)", Action::key(&[Win, Ctrl], vk::LEFT)),
        ("Desktop right (Win+Ctrl+Right)", Action::key(&[Win, Ctrl], vk::RIGHT)),
        ("Show desktop (Win+D)", Action::key(&[Win], vk::letter(b'D'))),
        ("Volume up", Action::key(&[], vk::VOLUME_UP)),
        ("Volume down", Action::key(&[], vk::VOLUME_DOWN)),
        ("Mute", Action::key(&[], vk::VOLUME_MUTE)),
        ("Play / pause", Action::key(&[], vk::MEDIA_PLAY_PAUSE)),
        ("Next track", Action::key(&[], vk::MEDIA_NEXT)),
        ("Previous track", Action::key(&[], vk::MEDIA_PREV)),
    ]
}

fn key_list() -> Vec<(&'static str, u16)> {
    use crate::config::vk;
    let mut v: Vec<(&'static str, u16)> = vec![
        ("Tab", vk::TAB),
        ("Enter", vk::ENTER),
        ("Esc", vk::ESCAPE),
        ("Left", vk::LEFT),
        ("Right", vk::RIGHT),
        ("Up", vk::UP),
        ("Down", vk::DOWN),
        ("Vol +", vk::VOLUME_UP),
        ("Vol -", vk::VOLUME_DOWN),
        ("Mute", vk::VOLUME_MUTE),
        ("Play/Pause", vk::MEDIA_PLAY_PAUSE),
        ("Next", vk::MEDIA_NEXT),
        ("Prev", vk::MEDIA_PREV),
        ("Media Stop", vk::MEDIA_STOP),
    ];
    const LETTERS: [&str; 26] = [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ];
    for (i, l) in LETTERS.iter().enumerate() {
        v.push((l, 0x41 + i as u16));
    }
    const FKEYS: [&str; 12] = [
        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    ];
    for (i, l) in FKEYS.iter().enumerate() {
        v.push((l, 0x70 + i as u16));
    }
    v
}

/// Render an action editor. Returns true if the action changed.
pub(crate) fn action_editor(ui: &mut egui::Ui, action: &mut Action, id: &str) -> bool {
    let presets = presets();
    let current_label = presets
        .iter()
        .find(|(_, a)| a == action)
        .map(|(l, _)| *l)
        .unwrap_or("Custom…");
    let mut changed = false;

    egui::ComboBox::from_id_salt(id)
        .selected_text(current_label)
        .width(230.0)
        .show_ui(ui, |ui| {
            for (label, act) in &presets {
                if ui
                    .selectable_label(current_label == *label, *label)
                    .clicked()
                {
                    *action = act.clone();
                    changed = true;
                }
            }
            if ui.selectable_label(current_label == "Custom…", "Custom…").clicked() {
                if !matches!(action, Action::Key { .. }) {
                    *action = Action::key(&[], 0x41);
                }
                changed = true;
            }
        });

    if current_label == "Custom…" {
        if let Action::Key { mods, vk } = action {
            changed |= custom_key_editor(ui, mods, vk, id);
        }
    }
    changed
}

fn custom_key_editor(ui: &mut egui::Ui, mods: &mut Vec<Modifier>, vk: &mut u16, id: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for m in [Modifier::Ctrl, Modifier::Alt, Modifier::Shift, Modifier::Win] {
            let mut on = mods.contains(&m);
            if ui.checkbox(&mut on, format!("{m:?}")).changed() {
                if on {
                    if !mods.contains(&m) {
                        mods.push(m);
                    }
                } else {
                    mods.retain(|x| x != &m);
                }
                changed = true;
            }
        }
        let keys = key_list();
        let cur = keys
            .iter()
            .find(|(_, k)| k == vk)
            .map(|(l, _)| *l)
            .unwrap_or("?");
        egui::ComboBox::from_id_salt((id, "key"))
            .selected_text(cur)
            .show_ui(ui, |ui| {
                for (label, code) in &keys {
                    if ui.selectable_label(vk == code, *label).clicked() {
                        *vk = *code;
                        changed = true;
                    }
                }
            });
    });
    changed
}
