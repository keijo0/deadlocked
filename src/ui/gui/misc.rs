use egui::Ui;

use crate::ui::{
    app::App,
};

impl App {
    pub fn misc_settings(&mut self, ui: &mut Ui) {
        ui.separator();

        self.antiafk_settings(ui);

        ui.separator();

        egui::CollapsingHeader::new("Server Picker")
            .default_open(false)
            .show(ui, |ui| {
                self.server_picker_settings(ui);
            });
    }
}
