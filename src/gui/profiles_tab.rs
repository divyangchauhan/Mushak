//! Profiles tab: manage per-application profiles.

use crate::config::Profile;
use eframe::egui;
use std::time::{Duration, Instant};

impl super::App {
    pub(crate) fn profiles_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Application profiles");
        ui.label(
            "The default profile applies when no app profile matches the foreground window. \
             App profiles are matched by executable name (e.g. chrome.exe), first match wins.",
        );
        ui.add_space(6.0);

        if ui.button("➕ Add profile").clicked() {
            self.draft.profiles.push(Profile {
                name: "New profile".to_string(),
                match_processes: Vec::new(),
                buttons: Default::default(),
            });
            self.commit_config();
        }
        ui.separator();

        ui.horizontal(|ui| {
            ui.strong("Default profile");
            ui.label("(fallback — edit its buttons on the Buttons tab)");
        });

        let mut changed = false;
        let mut to_delete = None;
        let mut move_up = None;
        let mut move_down = None;

        for i in 0..self.draft.profiles.len() {
            ui.separator();
            let mut pick_here = false;
            {
                let p = &mut self.draft.profiles[i];
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    if ui.text_edit_singleline(&mut p.name).changed() {
                        changed = true;
                    }
                    if ui.button("↑").clicked() {
                        move_up = Some(i);
                    }
                    if ui.button("↓").clicked() {
                        move_down = Some(i);
                    }
                    if ui.button("🗑 Delete").clicked() {
                        to_delete = Some(i);
                    }
                });

                ui.label("Matches processes:");
                let mut remove_proc = None;
                for (j, proc) in p.match_processes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.monospace(proc);
                        if ui.small_button("✕").clicked() {
                            remove_proc = Some(j);
                        }
                    });
                }
                if let Some(j) = remove_proc {
                    p.match_processes.remove(j);
                    changed = true;
                }
                if ui.button("🎯 Pick a window (click it within 4s)").clicked() {
                    pick_here = true;
                }
            }
            if pick_here {
                self.pick_target = i + 1;
                self.pick_deadline = Some(Instant::now() + Duration::from_secs(4));
            }
        }

        if let Some(i) = to_delete {
            self.draft.profiles.remove(i);
            changed = true;
            if self.selected_profile > self.draft.profiles.len() {
                self.selected_profile = 0;
            }
        }
        if let Some(i) = move_up {
            if i > 0 {
                self.draft.profiles.swap(i, i - 1);
                changed = true;
            }
        }
        if let Some(i) = move_down {
            if i + 1 < self.draft.profiles.len() {
                self.draft.profiles.swap(i, i + 1);
                changed = true;
            }
        }
        if changed {
            self.commit_config();
        }

        if let Some(dl) = self.pick_deadline {
            let secs = dl.saturating_duration_since(Instant::now()).as_secs() + 1;
            ui.separator();
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("Click the target window now… {secs}s"),
            );
        }
    }
}
