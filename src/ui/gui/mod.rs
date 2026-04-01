use egui::Ui;
use utils::log;

use crate::{
    config::{WeaponConfig, write_config},
    message::{GameStatus, Message},
    ui::{app::App, color::Colors, gui::aimbot::AimbotTab},
};

pub mod aimbot;
mod antiafk;
mod config;
mod grenade;
mod helpers;
mod hud;
mod player;
mod misc;
mod server_picker;

#[derive(PartialEq)]
pub enum Tab {
    Aimbot,
    Player,
    Hud,
    Grenades,
    Misc,
    ServerPicker,
    Config,
}

impl App {
    pub fn send_config(&self) {
        self.send_message(Message::Config(Box::new(self.config.clone())));
        self.save();
    }

    pub fn send_message(&self, message: Message) {
        if self.channel.send(message).is_err() {
            std::process::exit(1);
        }
    }

    fn save(&self) {
        write_config(&self.config, &self.current_config);
    }

    fn gui(&mut self, ui: &mut Ui) {
        ui.ctx().set_pixels_per_point(self.display_scale);

        egui::Panel::top("topbar")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().button_padding = egui::vec2(5.0, 2.0);
                    for (tab, label) in [
                        (Tab::Aimbot, "Aimbot"),
                        (Tab::Player, "Player"),
                        (Tab::Hud, "Hud"),
                        (Tab::Grenades, "Grenades"),
                        (Tab::Misc, "Misc"),
                        (Tab::ServerPicker, "Server Picker"),
                        (Tab::Config, "Config"),
                    ] {
                        let selected = self.current_tab == tab;
                        if ui
                            .add(egui::Button::new(label).selected(selected).frame(true))
                            .clicked()
                        {
                            self.current_tab = tab;
                        }
                    }
                });
            });

        egui::Panel::bottom("status_bar")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.label(egui::RichText::new(format!("{}", self.game_status)).color(
                    match self.game_status {
                        GameStatus::Working => Colors::GREEN,
                        GameStatus::NotStarted => Colors::YELLOW,
                    },
                ));
            });

        egui::CentralPanel::default().show_inside(ui, |ui| match self.current_tab {
            Tab::Aimbot => self.aimbot_settings(ui),
            Tab::Player => self.player_settings(ui),
            Tab::Hud => self.hud_settings(ui),
            Tab::Grenades => self.grenade_settings(ui),
            Tab::Misc => self.misc_settings(ui),
            Tab::ServerPicker => self.server_picker_settings(ui),
            Tab::Config => self.config_settings(ui),
        });
    }

    fn weapon_config(&mut self) -> &mut WeaponConfig {
        if self.aimbot_tab == AimbotTab::Weapon {
            self.config
                .aim
                .weapons
                .get_mut(&self.aimbot_weapon)
                .unwrap()
        } else {
            &mut self.config.aim.global
        }
    }

    pub fn render(&mut self) {
        let self_ptr = self as *mut Self;

        let gui = self.gui.as_mut().unwrap();

        if let Err(err) = gui.make_current() {
            log::error!("could not make gui window current: {err}");
            return;
        }
        gui.run(|ui| (unsafe { &mut *self_ptr }).gui(ui));
        gui.clear();
        gui.paint();

        if let Err(err) = gui.swap_buffers() {
            log::error!("could not swap gui window buffers: {err}");
            return;
        }

        let overlay = self.overlay.as_mut().unwrap();

        overlay.window().set_cursor_hittest(false).unwrap();
        if let Err(err) = overlay.make_current() {
            log::error!("could not make overlay window current: {err}");
            return;
        }

        overlay.run(move |ui| {
            (unsafe { &mut *self_ptr }).overlay(ui);
        });
        overlay.clear();
        overlay.paint();

        if let Err(err) = overlay.swap_buffers() {
            log::error!("could not swap overlay window buffers: {err}");
        }
    }
}
