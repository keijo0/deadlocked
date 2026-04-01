use egui::{Align, Button, Ui};

use crate::{
    config::{
        BASE_PATH, CONFIG_PATH, available_configs, delete_config, parse_config,
        write_config,
    },
    ui::{app::App, color::Colors, grenades::read_grenades},
};

impl App {
    pub fn config_settings(&mut self, ui: &mut Ui) {
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .id_salt("config_left")
                .show(left, |left| {
                    self.config_list(left);
                });

            let right = &mut cols[1];
            self.config_controls(right);
        });
    }

    fn config_controls(&mut self, ui: &mut Ui) {
        if ui.button("Refresh").clicked() {
            self.available_configs = available_configs();
            self.grenades = read_grenades();
        }

        ui.horizontal(|ui| {
            if ui.button("+").clicked() && !self.new_config_name.is_empty() {
                if !self.new_config_name.ends_with(".toml") {
                    self.new_config_name.push_str(".toml");
                }
                let path = CONFIG_PATH.join(&self.new_config_name);
                write_config(&self.config, &path);
                self.new_config_name.clear();
                self.current_config = path;
                self.available_configs = available_configs();
            }
            ui.text_edit_singleline(&mut self.new_config_name);
        });

        ui.separator();

        if ui.button("Config Folder").clicked() {
            std::process::Command::new("xdg-open")
                .arg(BASE_PATH.as_os_str())
                .status()
                .unwrap();
        }
    }

    fn config_list(&mut self, ui: &mut Ui) {
        let mut clicked_config = None;
        let mut delete = None;

        for config in &self.available_configs {
            ui.horizontal(|ui| {
                if ui
                    .add(Button::selectable(
                        *config == self.current_config,
                        config.file_name().unwrap().to_str().unwrap(),
                    ))
                    .clicked()
                {
                    clicked_config = Some(config.clone());
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("\u{f0a7a}").clicked() {
                        delete = Some(config.clone());
                    }
                });
            });
        }

        if let Some(config_path) = clicked_config {
            self.config = parse_config(&config_path);
            self.current_config = config_path;
            self.send_config();
            ui.ctx().global_style_mut(|style| {
                style.visuals.selection.bg_fill = Colors::PURPLE
            });
        }

        if let Some(config) = delete {
            delete_config(&config);
            self.available_configs = available_configs();
            self.current_config = self.available_configs[0].clone();
            self.config = parse_config(&self.current_config);
        }
    }
}
