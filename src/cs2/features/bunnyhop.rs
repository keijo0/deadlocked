use crate::{
    config::Config,
    cs2::{CS2, entity::player::Player},
    os::mouse::Mouse,
};

impl CS2 {
    pub fn bunnyhop(&mut self, config: &Config, mouse: &mut Mouse) {
        if !config.misc.bunnyhop {
            return;
        }

        if !self.input.is_key_pressed(config.misc.bunnyhop_hotkey) {
            return;
        }

        let Some(local_player) = Player::local_player(self) else {
            return;
        };

        if !local_player.is_in_air(self) {
            mouse.space_press();
        } else {
            mouse.space_release();
        }
    }
}
