use glam::vec2;
use rand::RngExt;
use utils::log;

use crate::{
    config::{AimbotConfig, Config, KeyMode},
    cs2::{
        CS2,
        entity::{player::Player, weapon_class::WeaponClass},
    },
    math::{angles_to_fov, vec2_clamp},
    os::mouse::Mouse,
};

#[derive(Debug, Default)]
pub struct Aimbot {
    pub active: bool,
}

impl CS2 {
    pub fn aimbot(&mut self, config: &Config, mouse: &mut Mouse) {
        let hotkeys = config.aim.aimbot_hotkeys.as_slice();
        let config = self.aimbot_config(config);

        if !config.enabled {
            return;
        }

        match config.mode {
            KeyMode::Hold => {
                if !hotkeys.iter().any(|k| self.input.is_key_pressed(*k)) {
                    return;
                }
            }
            KeyMode::Toggle => {
                if hotkeys.iter().any(|k| self.input.key_just_pressed(*k)) {
                    self.aim.active = !self.aim.active;
                }
                if !self.aim.active {
                    return;
                }
            }
        }

        let Some(target) = &self.target.player else {
            return;
        };

        if !target.is_valid(self) {
            return;
        }

        let Some(local_player) = Player::local_player(self) else {
            return;
        };

        let weapon_class = local_player.weapon_class(self);
        let disallowed_weapons = [
            WeaponClass::Unknown,
            WeaponClass::Knife,
            WeaponClass::Grenade,
        ];
        if disallowed_weapons.contains(&weapon_class) {
            return;
        }

        if !self.validate_target(config, &local_player, target) {
            return;
        }

        if local_player.shots_fired(self) < config.start_bullet {
            return;
        }

        let view_angles = local_player.view_angles(self);

        let target_angle = {
            let mut smallest_fov = 360.0;
            let mut smallest_angle = glam::Vec2::ZERO;

            // Generate multiple random candidate positions and pick the one whose aim angle
            // is closest to the current view angles (mouse-relative humanization).
            // This keeps the aim near where the crosshair already is while still randomising
            // the exact point on the body, making the movement appear more human.
            const HUMANIZATION_CANDIDATES: usize = 8;
            let humanize = |pos: glam::Vec3| -> glam::Vec3 {
                if !config.humanization || config.humanization_amount <= 0.0 {
                    return pos;
                }
                let r = config.humanization_amount;
                let mut rng = rand::rng();
                let mut best_pos = pos;
                let mut best_fov = f32::MAX;
                for _ in 0..HUMANIZATION_CANDIDATES {
                    let candidate = pos
                        + glam::Vec3::new(
                            rng.random_range(-r..=r),
                            rng.random_range(-r..=r),
                            rng.random_range(-r..=r),
                        );
                    let candidate_angle =
                        self.angle_to_target(&local_player, &candidate, &self.target.previous_aim_punch);
                    let fov = angles_to_fov(&view_angles, &candidate_angle);
                    if fov < best_fov {
                        best_fov = fov;
                        best_pos = candidate;
                    }
                }
                best_pos
            };

            for bone in &config.bones {
                let bone_pos = target.bone_position(self, bone.u64());
                let angle = self.angle_to_target(
                    &local_player,
                    &humanize(bone_pos),
                    &self.target.previous_aim_punch,
                );
                let fov = angles_to_fov(&view_angles, &angle);
                if fov < smallest_fov {
                    smallest_fov = fov;
                    smallest_angle = angle;
                }
            }

            smallest_angle
        };

        if angles_to_fov(&view_angles, &target_angle)
            > config.calculate_fov(self.target.distance)
        {
            return;
        }

        let mut aim_angles = view_angles - target_angle;
        if aim_angles.y < -180.0 {
            aim_angles.y += 360.0
        }
        vec2_clamp(&mut aim_angles);

        let sensitivity = self.get_sensitivity() * local_player.fov_multiplier(self);

        let mouse_angles = vec2(
            aim_angles.y / sensitivity * 50.0,
            -aim_angles.x / sensitivity * 50.0,
        ) / (config.smooth + 1.0).clamp(1.0, 20.0);

        log::debug!(
            "aimbot mouse movement: {:.2}/{:.2}",
            mouse_angles.x,
            mouse_angles.y
        );
        mouse.move_rel(&mouse_angles);
    }

    /// Performs a unified target validation check combining flash, visibility,
    /// and smoke/wall obstruction checks. Returns `true` if the target is valid
    /// (not blocked by any of the enabled checks), `false` otherwise.
    pub fn validate_target(
        &self,
        config: &AimbotConfig,
        local_player: &Player,
        target: &Player,
    ) -> bool {
        if config.flash_check && local_player.is_flashed(self) {
            return false;
        }

        if config.visibility_check && !target.visible(self, local_player) {
            return false;
        }

        if config.smoke_wall_check {
            let target_position = target.bone_position(self, self.target.bone_index);
            if !self.is_path_clear(local_player.eye_position(self), target_position) {
                return false;
            }
        }

        true
    }
}
