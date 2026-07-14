//! Gestures tab: thumb-button gesture actions + tap/swipe threshold.

use eframe::egui;

impl super::App {
    pub(crate) fn gestures_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Thumb gestures");
        ui.label("Hold the thumb button and move, or tap it. Requires HID++ (0x1B04).");
        ui.add_space(6.0);

        let mut enable_changed = false;
        let mut changed = false;
        {
            let g = &mut self.draft.gestures;
            if ui
                .checkbox(&mut g.enabled, "Enable thumb-button gestures")
                .changed()
            {
                enable_changed = true;
            }

            ui.add_enabled_ui(g.enabled, |ui| {
                let r = ui.add(
                    egui::Slider::new(&mut g.tap_threshold, 10..=400)
                        .text("Tap vs. swipe threshold (raw units)"),
                );
                if r.drag_stopped() || r.lost_focus() {
                    changed = true;
                }
                ui.separator();
                egui::Grid::new("gesture_grid")
                    .num_columns(2)
                    .spacing([16.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Tap");
                        changed |= super::action_editor(ui, &mut g.tap, "g_tap");
                        ui.end_row();
                        ui.label("Swipe up");
                        changed |= super::action_editor(ui, &mut g.up, "g_up");
                        ui.end_row();
                        ui.label("Swipe down");
                        changed |= super::action_editor(ui, &mut g.down, "g_down");
                        ui.end_row();
                        ui.label("Swipe left");
                        changed |= super::action_editor(ui, &mut g.left, "g_left");
                        ui.end_row();
                        ui.label("Swipe right");
                        changed |= super::action_editor(ui, &mut g.right, "g_right");
                        ui.end_row();
                    });
            });
        }

        if enable_changed {
            self.commit_and_apply_gesture();
        } else if changed {
            self.commit_config();
        }
    }
}
