use egui::Ui;

use crate::ui::{
    app::App,
    gui::helpers::{checkbox, keybind},
};

impl App {
    pub fn misc_settings(&mut self, ui: &mut Ui) {
        if checkbox(ui, "Bunnyhop", &mut self.config.misc.bunnyhop) {
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

        ui.separator();

        self.antiafk_settings(ui);
    }
}
