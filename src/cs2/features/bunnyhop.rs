use crate::{
    config::Config,
    cs2::{CS2, entity::player::Player},
    os::mouse::Mouse,
};

impl CS2 {
    pub fn bunnyhop(&mut self, config: &Config, mouse: &mut Mouse) {
        // If bhop is disabled or hotkey is not held, ensure space is released and bail out.
        if !config.misc.bunnyhop || !self.input.is_key_pressed(config.misc.bunnyhop_hotkey) {
            if self.bhop_space_pressed {
                mouse.space_release();
                self.bhop_space_pressed = false;
            }
            return;
        }

        let Some(local_player) = Player::local_player(self) else {
            return;
        };

        let want_pressed = !local_player.is_in_air(self);

        // Only write to /dev/uinput when the desired state changes.
        if want_pressed && !self.bhop_space_pressed {
            mouse.space_press();
            self.bhop_space_pressed = true;
        } else if !want_pressed && self.bhop_space_pressed {
            mouse.space_release();
            self.bhop_space_pressed = false;
        }
    }
}
