//! Buttons tab: per-profile remapping of the back / forward / middle buttons.

use eframe::egui;

impl super::App {
    pub(crate) fn buttons_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Button remapping");
        ui.label("Remap the side and middle buttons. Choose a profile to edit its mappings.");
        ui.add_space(6.0);

        let names = self.profile_names();
        let current = names
            .get(self.selected_profile)
            .cloned()
            .unwrap_or_default();
        egui::ComboBox::from_label("Profile")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_profile, i, n);
                }
            });
        ui.separator();

        let idx = self.selected_profile;
        let mut changed = false;
        if let Some(profile) = self.profile_at_mut(idx) {
            egui::Grid::new("buttons_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Back (side, lower)");
                    changed |= super::action_editor(ui, &mut profile.buttons.back, "btn_back");
                    ui.end_row();

                    ui.label("Forward (side, upper)");
                    changed |= super::action_editor(ui, &mut profile.buttons.forward, "btn_fwd");
                    ui.end_row();

                    ui.label("Middle (wheel click)");
                    changed |= super::action_editor(ui, &mut profile.buttons.middle, "btn_mid");
                    ui.end_row();
                });
        } else {
            ui.label("No such profile.");
        }

        if changed {
            self.commit_config();
        }
    }
}
