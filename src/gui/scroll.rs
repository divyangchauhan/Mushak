//! Scroll tab: SmartShift (0x2110) + hi-res wheel (0x2121) settings.

use crate::config::SmartShiftMode;
use eframe::egui;

impl super::App {
    pub(crate) fn scroll_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scroll wheel");
        ui.add_space(6.0);

        let commit_device = {
            let d = &mut self.draft.device;
            let mut commit = false;

            ui.label("SmartShift mode");
            for (mode, label) in [
                (SmartShiftMode::AlwaysRatchet, "Always ratchet (clicky, stepped)"),
                (SmartShiftMode::AlwaysFreespin, "Always freespin (smooth, no steps)"),
                (SmartShiftMode::SmartShift, "SmartShift (ratchet, spins free when flicked)"),
            ] {
                if ui.radio_value(&mut d.smartshift, mode, label).changed() {
                    commit = true;
                }
            }

            ui.add_enabled_ui(d.smartshift == SmartShiftMode::SmartShift, |ui| {
                let r = ui.add(
                    egui::Slider::new(&mut d.smartshift_threshold, 1..=30)
                        .text("Flick sensitivity (lower = easier to free-spin)"),
                );
                if r.drag_stopped() || r.lost_focus() {
                    commit = true;
                }
            });

            ui.separator();
            if ui
                .checkbox(&mut d.hires_scroll, "High-resolution (smooth) scrolling")
                .changed()
            {
                commit = true;
            }
            if ui
                .checkbox(&mut d.invert_scroll, "Invert scroll direction")
                .changed()
            {
                commit = true;
            }
            commit
        };

        if commit_device {
            self.commit_and_apply_device();
        }
    }
}
