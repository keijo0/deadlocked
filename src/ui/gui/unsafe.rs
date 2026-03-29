use egui::{DragValue, Ui};

use crate::ui::{
    app::App,
    gui::helpers::{checkbox, checkbox_hover, collapsing_open, drag, keybind},
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

            if checkbox_hover(
                ui,
                "Show Backtrack",
                "Visualise stored backtrack positions on the overlay",
                &mut self.config.player.show_backtrack,
            ) {
                self.send_config();
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

        collapsing_open(ui, "Visual", |ui| {
            if drag(
                ui,
                "FOV Override (0 = disabled)",
                DragValue::new(&mut self.config.misc.desired_fov)
                    .range(0..=130)
                    .speed(1),
            ) {
                self.send_config();
            }
        });
    }
}
