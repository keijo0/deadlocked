use egui::{Align, Button, DragValue, Ui};

use crate::{
    config::{
        AppState, BASE_PATH, CONFIG_PATH, available_configs, delete_config, parse_config,
        write_app_state, write_config,
    },
    ui::{app::App, color::AccentStyle, grenades::read_grenades},
};
use strum::IntoEnumIterator;

impl App {
    pub fn config_settings(&mut self, ui: &mut Ui) {
        // Top row: new config creation + UI scale presets
        ui.horizontal_wrapped(|ui| {
            ui.label("New:");
            let te = ui.text_edit_singleline(&mut self.new_config_name);
            if (te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || (ui.button("Add").clicked() && !self.new_config_name.is_empty())
            {
                if !self.new_config_name.ends_with(".toml") {
                    self.new_config_name.push_str(".toml");
                }
                let path = CONFIG_PATH.join(&self.new_config_name);
                write_config(&self.config, &path);
                self.new_config_name.clear();
                self.current_config = path;
                self.available_configs = available_configs();
            }

            ui.separator();

            // UI scale preset buttons inline
            ui.label("Scale:");
            for (label, scale) in [
                ("75%", 0.75_f32),
                ("100%", 1.0),
                ("125%", 1.25),
            ] {
                let selected = (self.ui_scale - scale).abs() < 0.01;
                if ui
                    .add(Button::new(label).selected(selected).frame(true))
                    .clicked()
                {
                    self.ui_scale = scale;
                    write_app_state(&AppState { ui_scale: self.ui_scale });
                }
            }

            // Custom scale drag
            let pct = &mut (self.ui_scale * 100.0);
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
                self.ui_scale = (*pct / 100.0).clamp(0.1, 9.99);
                write_app_state(&AppState { ui_scale: self.ui_scale });
            }
        });

        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                self.available_configs = available_configs();
                self.grenades = read_grenades();
            }
            if ui.button("Open Folder").clicked() {
                if let Err(e) = std::process::Command::new("xdg-open")
                    .arg(BASE_PATH.as_os_str())
                    .status()
                {
                    utils::log::error!("xdg-open failed: {e}");
                }
            }
        });

        ui.separator();

        // Theme selector
        ui.horizontal(|ui| {
            ui.label("Theme:");
            let prev_style = self.config.accent_style.clone();
            egui::ComboBox::new("accent_style_combo", "")
                .selected_text(format!("{:?}", self.config.accent_style))
                .show_ui(ui, |ui| {
                    for style in AccentStyle::iter() {
                        let text = format!("{:?}", style);
                        ui.selectable_value(&mut self.config.accent_style, style, text);
                    }
                });
            if prev_style != self.config.accent_style {
                self.send_config();
            }
        });

        // Config list
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .id_salt("config_list")
            .show(ui, |ui| {
                self.config_list(ui);
            });
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
                    if ui.button("X").clicked() {
                        delete = Some(config.clone());
                    }
                });
            });
        }

        if let Some(config_path) = clicked_config {
            self.config = parse_config(&config_path);
            self.current_config = config_path;
            self.send_config();
            // The render loop detects the new accent_style and re-applies the
            // full color scheme on the very next frame.
        }

        if let Some(config) = delete {
            delete_config(&config);
            self.available_configs = available_configs();
            self.current_config = self.available_configs[0].clone();
            self.config = parse_config(&self.current_config);
        }
    }
}
