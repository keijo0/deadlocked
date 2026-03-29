use crate::{
    config::Config,
    cs2::{CS2, entity::player::Player},
};

impl CS2 {
    pub fn fov(&self, config: &Config) {
        let desired_fov = config.misc.desired_fov;
        if desired_fov == 0 {
            return;
        }

        let Some(local_player) = Player::local_player(self) else {
            return;
        };

        local_player.set_fov(self, desired_fov);
    }
}
