use crate::{config::Config, cs2::CS2};

#[derive(Debug)]
pub struct EspToggle {
    pub active: bool,
}

impl Default for EspToggle {
    fn default() -> Self {
        Self { active: true }
    }
}

impl CS2 {
    pub fn esp_toggle(&mut self, config: &Config) {
        let hotkey = config.player.esp_hotkey;

        if self.input.key_just_pressed(hotkey) {
            self.esp.active = !self.esp.active;
        }
    }

    pub fn esp_enabled(&self, config: &Config) -> bool {
        config.player.enabled && self.esp.active
    }
}
