use egui::{DragValue, Ui};

use crate::ui::{
    app::App,
    gui::helpers::{collapsing_open, color_picker},
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

                collapsing_open(ui, "Colors", |ui| {
                    if color_picker(ui, "Text Color", &mut self.config.hud.text_color) {
                        self.send_config();
                    }

                    if color_picker(ui, "Crosshair Color", &mut self.config.hud.crosshair_color) {
                        self.send_config();
                    }
                });
            });
    }

    fn hud_left(&mut self, ui: &mut Ui) {
        collapsing_open(ui, "HUD", |ui| {
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
                .checkbox(&mut self.config.hud.fov_circle, "FOV Circle")
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

            ui.collapsing("FOV Arrows", |ui| {
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
                    ui.label("Size");
                });

                if color_picker(ui, "Color", &mut self.config.hud.fov_arrow_color) {
                    self.send_config();
                }
            });
        });
    }

    fn hud_right(&mut self, ui: &mut Ui) {
        collapsing_open(ui, "Appearance", |ui| {
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
        });

        ui.collapsing("Advanced", |ui| {
            if ui
                .checkbox(&mut self.config.hud.debug, "Debug Overlay")
                .changed()
            {
                self.send_config();
            }
        });
    }
}
