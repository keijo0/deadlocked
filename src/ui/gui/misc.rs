use egui::{DragValue, Ui};

use crate::ui::{
    app::App,
    gui::helpers::{checkbox, drag, keybind},
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

        if checkbox(
            ui,
            "Enable Backtrack",
            &mut self.config.aim.global.aimbot.backtrack,
        ) {
            self.send_config();
        }

        if drag(
            ui,
            "Backtrack Ticks",
            DragValue::new(&mut self.config.aim.global.aimbot.backtrack_ticks)
                .range(1..=32)
                .speed(0.1),
        ) {
            self.send_config();
        }

        ui.separator();

        self.antiafk_settings(ui);
    }
}
