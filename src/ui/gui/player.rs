use egui::{DragValue, Ui};

use crate::ui::{
    app::App,
    gui::helpers::{checkbox, checkbox_hover, color_picker, combo_box, keybind, scroll},
};

impl App {
    pub fn player_settings(&mut self, ui: &mut Ui) {
        scroll(ui, "player", |ui| {
            ui.columns(2, |cols| {
                let left = &mut cols[0];
                self.player_left(left);
                let right = &mut cols[1];
                self.player_right(right);
            });

            ui.separator();

            if color_picker(ui, "Box (visible)", &mut self.config.player.box_visible_color) {
                self.send_config();
            }
            if color_picker(ui, "Box (invisible)", &mut self.config.player.box_invisible_color) {
                self.send_config();
            }
            if color_picker(ui, "Skeleton", &mut self.config.player.skeleton_color) {
                self.send_config();
            }

            ui.separator();

            if checkbox(ui, "Backtrack Visual", &mut self.config.player.backtrack_visual) {
                self.send_config();
            }
            if self.config.player.backtrack_visual {
                if checkbox_hover(
                    ui,
                    "  Backtrack follows ESP",
                    "Hide backtrack ghosts when ESP is toggled off",
                    &mut self.config.player.backtrack_in_esp,
                ) {
                    self.send_config();
                }
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            DragValue::new(&mut self.config.player.backtrack_ms)
                                .range(15..=200)
                                .speed(1.0)
                                .suffix("ms"),
                        )
                        .changed()
                    {
                        self.send_config();
                    }
                    ui.label("Backtrack Duration")
                        .on_hover_text("How far back in time to show ghost positions (15ms ≈ 1 tick at 64Hz)");
                });
                if color_picker(ui, "Backtrack Color", &mut self.config.player.backtrack_color) {
                    self.send_config();
                }
            }

            ui.separator();
        });
    }

    fn player_left(&mut self, ui: &mut Ui) {
        if checkbox(ui, "Enable", &mut self.config.player.enabled) {
            self.send_config();
        }
        if keybind(ui, "esp_hotkey", "ESP Hotkey", &mut self.config.player.esp_hotkey) {
            self.send_config();
        }
        if checkbox_hover(
            ui,
            "Show Friendlies",
            "Only active in custom game modes (workshop/custom maps)",
            &mut self.config.player.show_friendlies,
        ) {
            self.send_config();
        }
        if combo_box(ui, "Box", &mut self.config.player.draw_box) {
            self.send_config();
        }
        if combo_box(ui, "Box Mode", &mut self.config.player.box_mode) {
            self.send_config();
        }
        if combo_box(ui, "Skeleton", &mut self.config.player.draw_skeleton) {
            self.send_config();
        }
        if checkbox_hover(
            ui,
            "Visible Only",
            "Only show visible players",
            &mut self.config.player.visible_only,
        ) {
            self.send_config();
        }
    }

    fn player_right(&mut self, ui: &mut Ui) {
        if ui.checkbox(&mut self.config.player.health_bar, "Health Bar").changed() {
            self.send_config();
        }
        if ui.checkbox(&mut self.config.player.health_text, "Health Text").changed() {
            self.send_config();
        }
        if ui.checkbox(&mut self.config.player.armor_text, "Armor Text").changed() {
            self.send_config();
        }
        if ui.checkbox(&mut self.config.player.player_name, "Player Name").changed() {
            self.send_config();
        }
        if ui.checkbox(&mut self.config.player.weapon_icon, "Weapon Name").changed() {
            self.send_config();
        }
        if ui.checkbox(&mut self.config.player.tags, "Show Tags").changed() {
            self.send_config();
        }
    }
}
