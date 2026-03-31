use egui::{DragValue, Ui};

use crate::ui::{
    app::App,
    gui::helpers::{checkbox, collapsing_open, drag, keybind},
};

impl App {
    pub fn misc_settings(&mut self, ui: &mut Ui) {
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

        collapsing_open(ui, "Backtrack", |ui| {
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
        });

        self.antiafk_settings(ui);

        collapsing_open(ui, "Server Picker", |ui| {
            ui.label("Block or unblock game server relays to control matchmaking.");
            if ui.button("Open Server Picker X").clicked() {
                let _ = std::process::Command::new("xdg-open")
                    .arg("https://github.com/FN-FAL113/server-picker-x/releases")
                    .status();
            }
        });

        collapsing_open(ui, "HUD", |ui| {
            if checkbox(
                ui,
                "Keybind List",
                &mut self.config.hud.keybind_list,
            ) {
                self.send_config();
            }
        });
    }
}
