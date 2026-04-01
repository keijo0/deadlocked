use egui::{DragValue, Ui};

use crate::ui::{
    app::App,
    gui::helpers::{checkbox, color_picker},
};

impl App {
    pub fn hud_settings(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .id_salt("hud")
            .show(ui, |ui| {
                ui.columns(2, |cols| {
                    let left = &mut cols[0];
                    self.hud_left(left);
                    let right = &mut cols[1];
                    self.hud_right(right);
                });

                ui.separator();

                if checkbox(ui, "Keybind List", &mut self.config.hud.keybind_list) {
                    self.send_config();
                }

                if self.config.hud.keybind_list {
                    ui.columns(2, |cols| {
                        let left = &mut cols[0];
                        left.label("Aim");
                        if checkbox(left, "Aimbot", &mut self.config.hud.keybind_aimbot) {
                            self.send_config();
                        }
                        if checkbox(left, "FOV / Smooth", &mut self.config.hud.keybind_fov) {
                            self.send_config();
                        }
                        if checkbox(
                            left,
                            "Trigger Delay",
                            &mut self.config.hud.keybind_trigger_delay,
                        ) {
                            self.send_config();
                        }
                        if checkbox(left, "Triggerbot", &mut self.config.hud.keybind_triggerbot) {
                            self.send_config();
                        }
                        if checkbox(left, "Backtrack", &mut self.config.hud.keybind_backtrack) {
                            self.send_config();
                        }

                        let right = &mut cols[1];
                        right.label("Other");
                        if checkbox(right, "ESP", &mut self.config.hud.keybind_esp) {
                            self.send_config();
                        }
                        if checkbox(right, "Bunnyhop", &mut self.config.hud.keybind_bunnyhop) {
                            self.send_config();
                        }
                        if checkbox(
                            right,
                            "Server Picker",
                            &mut self.config.hud.keybind_server_picker,
                        ) {
                            self.send_config();
                        }
                    });
                }

                ui.separator();

                if color_picker(ui, "Text Color", &mut self.config.hud.text_color) {
                    self.send_config();
                }
                if color_picker(ui, "Crosshair Color", &mut self.config.hud.crosshair_color) {
                    self.send_config();
                }
                if color_picker(
                    ui,
                    "Spectator List Color",
                    &mut self.config.hud.spectator_list_color,
                ) {
                    self.send_config();
                }
                if color_picker(ui, "FOV Arrow Color", &mut self.config.hud.fov_arrow_color) {
                    self.send_config();
                }
            });
    }

    fn hud_left(&mut self, ui: &mut Ui) {
        if ui
            .checkbox(&mut self.config.hud.bomb_timer, "Bomb Timer")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.hud.spectator_list, "Spectator List")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.hud.sniper_crosshair, "Sniper Crosshair")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.hud.dropped_weapons, "Dropped Weapons")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.hud.grenade_trails, "Grenade Trails")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.hud.fov_arrows, "FOV Arrows")
            .changed()
        {
            self.send_config();
        }
    }

    fn hud_right(&mut self, ui: &mut Ui) {
        if ui
            .checkbox(&mut self.config.hud.text_outline, "Text Outline")
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.hud.line_width)
                        .range(0.1..=8.0)
                        .speed(0.02)
                        .max_decimals(1),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Line Width");
        });

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.hud.font_size)
                        .range(1.0..=99.0)
                        .speed(0.2)
                        .max_decimals(1),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Font Size");
        });

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.hud.icon_size)
                        .range(1.0..=99.0)
                        .speed(0.2)
                        .max_decimals(1),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Icon Size");
        });

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.hud.fov_arrow_size)
                        .range(0.0..=50.0)
                        .speed(0.1)
                        .max_decimals(1),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("FOV Arrow Size");
        });

        if ui
            .checkbox(&mut self.config.hud.debug, "Debug Overlay")
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.hud.overlay_refresh_rate)
                        .range(30..=360)
                        .speed(1),
                )
                .on_hover_text("Overlay/UI render refresh rate")
                .changed()
            {
                self.send_config();
            }
            ui.label("Overlay FPS");
        });

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.hud.data_refresh_rate)
                        .range(20..=240)
                        .speed(1),
                )
                .on_hover_text("How often game data is sampled for ESP")
                .changed()
            {
                self.send_config();
            }
            ui.label("Data FPS");
        });
    }
}
