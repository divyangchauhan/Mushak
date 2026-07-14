//! Device tab: connection status, battery, firmware, DPI, discovered features.
//! Live values come from the status file the resident process publishes.

use eframe::egui;

impl super::App {
    pub(crate) fn device_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Device");
        ui.add_space(6.0);

        let status = self.device_view.device.clone();
        egui::Grid::new("device_grid")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                ui.label("Connection:");
                ui.label(status.connection.as_deref().unwrap_or("not connected"));
                ui.end_row();

                ui.label("Firmware:");
                ui.label(status.firmware.as_deref().unwrap_or("—"));
                ui.end_row();

                ui.label("Battery:");
                ui.label(match status.battery_percent {
                    Some(p) => format!(
                        "{p}%{}",
                        if status.charging { " (charging)" } else { "" }
                    ),
                    None => "—".to_string(),
                });
                ui.end_row();

                ui.label("Current DPI:");
                ui.label(
                    status
                        .dpi_current
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "—".to_string()),
                );
                ui.end_row();
            });

        ui.separator();

        // DPI slider, bounded by the device's reported list.
        let (min, max) = match (status.dpi_list.first(), status.dpi_list.last()) {
            (Some(&f), Some(&l)) if l > f => (f, l),
            _ => (200, 4000),
        };
        let mut commit = false;
        {
            let d = &mut self.draft.device;
            d.dpi = d.dpi.clamp(min, max);
            let r = ui.add(
                egui::Slider::new(&mut d.dpi, min..=max)
                    .text("DPI")
                    .step_by(50.0),
            );
            if r.drag_stopped() || r.lost_focus() {
                commit = true;
            }
        }
        if commit {
            self.commit_and_apply_device();
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Refresh status").clicked() {
                self.device_view = crate::statusfile::read();
            }
            if ui.button("Re-apply settings").clicked() {
                self.commit_config();
            }
        });

        if !status.features.is_empty() {
            ui.separator();
            ui.collapsing(format!("HID++ features ({})", status.features.len()), |ui| {
                for (id, idx) in &status.features {
                    ui.monospace(format!("{id:#06x}  @ index {idx}"));
                }
            });
        }
    }
}
