use egui::Ui;

use crate::ui::{
    app::App,
    gui::helpers::{collapsing_open, keybind},
};

impl App {
    pub fn unsafe_settings(&mut self, ui: &mut Ui) {
        collapsing_open(ui, "Automation", |ui| {
            if ui
                .checkbox(&mut self.config.misc.bunnyhop, "Bunnyhop")
                .changed()
            {
                self.send_config();
            }

            if keybind(
                ui,
                "bunnyhop_hotkey",
                "Bunnyhop Hotkey",
                &mut self.config.misc.bunnyhop_hotkey,
            ) {
                self.send_config();
            }
        });
    }
}
