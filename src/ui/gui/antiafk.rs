use egui::{DragValue, Ui};

use crate::ui::{app::App, gui::helpers::{checkbox, drag}};

impl App {
    pub fn antiafk_settings(&mut self, ui: &mut Ui) {
        if checkbox(ui, "Anti-AFK", &mut self.config.misc.antiafk.enabled) {
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
    }
}
