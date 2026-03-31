use egui::{DragValue, Ui};

use crate::ui::{app::App, gui::helpers::{checkbox, collapsing_open, drag}};

impl App {
    pub fn antiafk_settings(&mut self, ui: &mut Ui) {
        collapsing_open(ui, "Anti-AFK", |ui| {
            if checkbox(ui, "Enable", &mut self.config.misc.antiafk.enabled) {
                self.send_config();
            }

            if checkbox(ui, "Walk Bot", &mut self.config.misc.antiafk.walk_bot) {
                self.send_config();
            }

            if drag(
                ui,
                "Min Interval (s)",
                DragValue::new(&mut self.config.misc.antiafk.interval_min)
                    .range(1.0..=60.0)
                    .speed(0.5),
            ) {
                self.send_config();
            }

            if drag(
                ui,
                "Max Interval (s)",
                DragValue::new(&mut self.config.misc.antiafk.interval_max)
                    .range(1.0..=60.0)
                    .speed(0.5),
            ) {
                self.send_config();
            }

            if drag(
                ui,
                "Walk Duration Min (s)",
                DragValue::new(&mut self.config.misc.antiafk.walk_duration_min)
                    .range(0.05..=5.0)
                    .speed(0.05),
            ) {
                self.send_config();
            }

            if drag(
                ui,
                "Walk Duration Max (s)",
                DragValue::new(&mut self.config.misc.antiafk.walk_duration_max)
                    .range(0.05..=5.0)
                    .speed(0.05),
            ) {
                self.send_config();
            }
        });
    }
}
