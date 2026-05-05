use egui::{Button, DragValue, Ui};

use crate::{
    config::SpectatorListStyle,
    ui::{
        app::App,
        gui::helpers::{checkbox, color_picker, combo_box},
    },
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
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 1.0);
                        egui::Grid::new("keybind_grid")
                            .num_columns(3)
                            .spacing([8.0, 1.0])
                            .show(ui, |ui| {
                                if checkbox(ui, "Aimbot", &mut self.config.hud.keybind_aimbot) {
                                    self.send_config();
                                }
                                if checkbox(ui, "FOV/Smooth", &mut self.config.hud.keybind_fov) {
                                    self.send_config();
                                }
                                if checkbox(ui, "Triggerbot", &mut self.config.hud.keybind_triggerbot) {
                                    self.send_config();
                                }
                                ui.end_row();
                                if checkbox(ui, "Trg Delay", &mut self.config.hud.keybind_trigger_delay) {
                                    self.send_config();
                                }
                                if checkbox(ui, "Autowall", &mut self.config.hud.keybind_autowall) {
                                    self.send_config();
                                }
                                if checkbox(ui, "Threshold", &mut self.config.hud.keybind_threshold) {
                                    self.send_config();
                                }
                                ui.end_row();
                                if checkbox(ui, "Trg Activate", &mut self.config.hud.keybind_trigger_activate) {
                                    self.send_config();
                                }
                                if checkbox(ui, "Trg Indicator", &mut self.config.hud.keybind_trigger_active_indicator) {
                                    self.send_config();
                                }
                                if checkbox(ui, "Backtrack", &mut self.config.hud.keybind_backtrack) {
                                    self.send_config();
                                }
                                ui.end_row();
                                if checkbox(ui, "ESP", &mut self.config.hud.keybind_esp) {
                                    self.send_config();
                                }
                                if checkbox(ui, "Server Picker", &mut self.config.hud.keybind_server_picker) {
                                    self.send_config();
                                }
                            });
                    });
                }

                ui.separator();

                if color_picker(ui, "Text Color", &mut self.config.hud.text_color) {
                    self.send_config();
                }
                if color_picker(ui, "Crosshair Color", &mut self.config.hud.crosshair_color) {
                    self.send_config();
                }
                if color_picker(ui, "FOV Arrow Color", &mut self.config.hud.fov_arrow_color) {
                    self.send_config();
                }
                if self.config.hud.penetration_crosshair_enabled {
                    if color_picker(
                        ui,
                        "Can Wallbang",
                        &mut self.config.hud.penetration_color_yes,
                    ) {
                        self.send_config();
                    }
                    if color_picker(
                        ui,
                        "Cannot Wallbang",
                        &mut self.config.hud.penetration_color_no,
                    ) {
                        self.send_config();
                    }
                    if color_picker(
                        ui,
                        "Unavailable",
                        &mut self.config.hud.penetration_color_unavailable,
                    ) {
                        self.send_config();
                    }
                }
                if self.config.hud.media_info {
                    if color_picker(ui, "Media Info Color", &mut self.config.hud.media_info_color) {
                        self.send_config();
                    }
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

        if self.config.hud.spectator_list {
            if combo_box(ui, "Style", &mut self.config.hud.spectator_list_style) {
                self.send_config();
            }
            // Scale and Sync to UI apply to both Simple and New styles.
            ui.horizontal_wrapped(|ui| {
                ui.label("Scale:");
                let selected = (self.config.hud.spectator_list_scale - 1.0).abs() < 0.01;
                if ui
                    .add(Button::new("100%").selected(selected).frame(true))
                    .clicked()
                {
                    self.config.hud.spectator_list_scale = 1.0;
                    self.send_config();
                }
                let pct = &mut (self.config.hud.spectator_list_scale * 100.0);
                if ui
                    .add(
                        DragValue::new(pct)
                            .range(10.0..=999.0)
                            .speed(1.0)
                            .max_decimals(0)
                            .suffix("%"),
                    )
                    .changed()
                {
                    self.config.hud.spectator_list_scale = (*pct / 100.0).clamp(0.1, 9.99);
                    self.send_config();
                }
                if ui.button("Sync to UI").clicked() {
                    self.config.hud.spectator_list_scale = self.ui_scale;
                    self.send_config();
                }
            });
            if self.config.hud.spectator_list_style != SpectatorListStyle::Simple {
                ui.horizontal(|ui| {
                    ui.label("X");
                    if ui
                        .add(
                            DragValue::new(&mut self.config.hud.spectator_list_x)
                                .range(-1.0..=3840.0)
                                .speed(1.0),
                        )
                        .on_hover_text("-1 = auto (right side)")
                        .changed()
                    {
                        self.send_config();
                    }
                    ui.label("Y");
                    if ui
                        .add(
                            DragValue::new(&mut self.config.hud.spectator_list_y)
                                .range(-1.0..=2160.0)
                                .speed(1.0),
                        )
                        .on_hover_text("-1 = auto (center)")
                        .changed()
                    {
                        self.send_config();
                    }
                    if ui.small_button("Reset Position").clicked() {
                        self.config.hud.spectator_list_x = -1.0;
                        self.config.hud.spectator_list_y = -1.0;
                        self.send_config();
                    }
                });
            }
        }

        if ui
            .checkbox(&mut self.config.hud.sniper_crosshair, "Sniper Crosshair")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(
                &mut self.config.hud.penetration_crosshair_enabled,
                "Penetration Crosshair",
            )
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

        if self.config.hud.dropped_weapons {
            if ui
                .checkbox(&mut self.config.hud.weapon_box, "Weapon Box")
                .changed()
            {
                self.send_config();
            }
            if self.config.hud.weapon_box {
                if combo_box(ui, "Box Mode", &mut self.config.hud.weapon_box_mode) {
                    self.send_config();
                }

            }
            if ui
                .checkbox(&mut self.config.hud.weapon_esp_use_colors, "Weapon ESP Colors")
                .changed()
            {
                self.send_config();
            }
            ui.horizontal(|ui| {
                let mut opacity = self.config.hud.weapon_esp_background_opacity as f32;
                if ui
                    .add(
                        DragValue::new(&mut opacity)
                            .range(0.0..=255.0)
                            .speed(1.0)
                            .max_decimals(0),
                    )
                    .changed()
                {
                    self.config.hud.weapon_esp_background_opacity = opacity as u8;
                    self.send_config();
                }
                ui.label("Weapon Label BG Opacity");
            });
            ui.horizontal(|ui| {
                if ui
                    .add(
                        DragValue::new(&mut self.config.hud.weapon_esp_max_distance)
                            .range(0.0..=10000.0)
                            .speed(10.0)
                            .max_decimals(0),
                    )
                    .on_hover_text("Max distance to render weapon ESP labels (0 = no limit)")
                    .changed()
                {
                    self.send_config();
                }
                ui.label("Weapon ESP Max Distance");
            });
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

        ui.separator();

        if ui
            .checkbox(&mut self.config.hud.media_info, "Media Info (playerctl)")
            .changed()
        {
            self.send_config();
        }

        if self.config.hud.media_info {
            ui.horizontal_wrapped(|ui| {
                ui.label("Scale:");
                let selected = (self.config.hud.media_info_scale - 1.0).abs() < 0.01;
                if ui
                    .add(Button::new("100%").selected(selected).frame(true))
                    .clicked()
                {
                    self.config.hud.media_info_scale = 1.0;
                    self.send_config();
                }
                let pct = &mut (self.config.hud.media_info_scale * 100.0);
                if ui
                    .add(
                        DragValue::new(pct)
                            .range(10.0..=999.0)
                            .speed(1.0)
                            .max_decimals(0)
                            .suffix("%"),
                    )
                    .changed()
                {
                    self.config.hud.media_info_scale = (*pct / 100.0).clamp(0.1, 9.99);
                    self.send_config();
                }
                if ui.button("Sync to UI").clicked() {
                    self.config.hud.media_info_scale = self.ui_scale;
                    self.send_config();
                }
            });
            ui.horizontal(|ui| {
                ui.label("X");
                if ui
                    .add(
                        DragValue::new(&mut self.config.hud.media_info_x)
                            .range(-1.0..=3840.0)
                            .speed(1.0),
                    )
                    .on_hover_text("-1 = auto (top right)")
                    .changed()
                {
                    self.send_config();
                }
                ui.label("Y");
                if ui
                    .add(
                        DragValue::new(&mut self.config.hud.media_info_y)
                            .range(-1.0..=2160.0)
                            .speed(1.0),
                    )
                    .on_hover_text("-1 = auto (top)")
                    .changed()
                {
                    self.send_config();
                }
                if ui.small_button("Reset Position").clicked() {
                    self.config.hud.media_info_x = -1.0;
                    self.config.hud.media_info_y = -1.0;
                    self.send_config();
                }
            });
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
                    DragValue::new(&mut self.config.hud.gui_font_size)
                        .range(8.0..=24.0)
                        .speed(0.2)
                        .max_decimals(1),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("GUI Font Size");
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

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.hud.fov_arrow_margin)
                        .range(0.0..=400.0)
                        .speed(1.0)
                        .max_decimals(0),
                )
                .on_hover_text("Distance from screen edge for FOV arrows")
                .changed()
            {
                self.send_config();
            }
            ui.label("FOV Arrow Margin");
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
