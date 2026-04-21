use std::time::{Duration, Instant};

use glam::Vec2;
use rand::rng;

use crate::{
    config::{Config, KeyMode, safe_limits},
    cs2::{
        CS2,
        bones::Bones,
        entity::{player::Player, weapon_class::WeaponClass},
    },
    math::angles_to_fov,
    os::mouse::Mouse,
};

#[derive(Debug, Default)]
pub struct Triggerbot {
    shot_start: Option<Instant>,
    shot_end: Option<Instant>,
    pub active: bool,
    pub autowall_active: bool,
}

/// Returns a 0.0–1.0 penetration factor for the given weapon class.
pub(crate) fn weapon_penetration(weapon_class: &WeaponClass) -> f32 {
    match weapon_class {
        WeaponClass::Rifle | WeaponClass::Sniper => 1.0,
        WeaponClass::Smg | WeaponClass::Pistol | WeaponClass::Heavy => 0.75,
        WeaponClass::Shotgun => 0.5,
        _ => 0.0,
    }
}

/// Approximate base damage per weapon class.
pub(crate) fn base_damage(weapon_class: &WeaponClass) -> f32 {
    match weapon_class {
        WeaponClass::Sniper => 120.0,
        WeaponClass::Rifle => 30.0,
        WeaponClass::Heavy => 35.0,
        WeaponClass::Smg => 25.0,
        WeaponClass::Pistol => 30.0,
        WeaponClass::Shotgun => 60.0,
        _ => 0.0,
    }
}

const WALL_FALLOFF: f32 = 0.75;

pub(crate) fn calculate_penetration_damage(weapon_class: &WeaponClass, num_walls: u32) -> f32 {
    let penetration = weapon_penetration(weapon_class);
    if penetration == 0.0 {
        return 0.0;
    }
    base_damage(weapon_class) * penetration * WALL_FALLOFF.powi(num_walls as i32)
}

impl CS2 {
    pub fn triggerbot(&mut self, config: &Config, _mouse: &mut Mouse) {
        let hotkey = config.aim.triggerbot_hotkey;
        let autowall_hotkey = config.aim.autowall_hotkey;
        let autowall_mode = &config.aim.autowall_mode;

        let Some(local_player) = Player::local_player(self) else {
            return;
        };

        // Get current triggerbot config (global + per-weapon)
        let aim = &config.aim;
        let current_weapon = local_player.weapon(self);

        let tb = if let Some(wcfg) = aim.weapons.get(&current_weapon) {
            &wcfg.triggerbot
        } else {
            &aim.global.triggerbot
        };

        if !tb.enabled {
            return;
        }

        // Hotkey handling
        match tb.mode {
            KeyMode::Hold => {
                if !self.input.is_key_pressed(hotkey) {
                    return;
                }
            }
            KeyMode::Toggle => {
                if self.input.key_just_pressed(hotkey) {
                    self.trigger.active = !self.trigger.active;
                }
                if !self.trigger.active {
                    return;
                }
            }
        }

        // Autowall toggle handling (global keybind)
        match autowall_mode {
            KeyMode::Hold => {}
            KeyMode::Toggle => {
                if self.input.key_just_pressed(autowall_hotkey) {
                    self.trigger.autowall_active = !self.trigger.autowall_active;
                }
            }
        }

        let autowall_on = !config.parental_lock && config.aim.autowall_enabled && match autowall_mode {
            KeyMode::Hold => self.input.is_key_pressed(autowall_hotkey),
            KeyMode::Toggle => self.trigger.autowall_active,
        };

        let shot_pending = self.trigger.shot_start.is_some() || self.trigger.shot_end.is_some();
        if shot_pending {
            return;
        }

        if tb.flash_check && local_player.is_flashed(self) {
            return;
        }

        if tb.scope_check
            && local_player.weapon_class(self) == WeaponClass::Sniper
            && !local_player.is_scoped(self)
        {
            return;
        }

        if tb.velocity_check && local_player.velocity(self).length() > tb.velocity_threshold {
            return;
        }

        let ffa = self.is_ffa();
        let local_team = local_player.team(self);

        // Target selection
        let mut target_player = local_player.crosshair_entity(self);
        let found_via_crosshair = target_player.is_some();

        if target_player.is_none() && autowall_on {
            let view_angles = local_player.view_angles(self);
            let mut best_fov = f32::MAX;
            let mut best_player = None;

            for &p in &self.players {
                if !ffa && p.team(self) == local_team { continue; }
                if p.health(self) <= 0 { continue; }

                let head = p.bone_position(self, Bones::Head.u64());
                let target_angle = self.angle_to_target(&local_player, &head, &Vec2::ZERO);
                let fov = angles_to_fov(&view_angles, &target_angle);

                let max_fov = 4.0;

                if fov < max_fov && fov < best_fov {
                    best_fov = fov;
                    best_player = Some(p);
                }
            }
            if let Some(p) = best_player {
                target_player = Some(p);
            }
        }

        let Some(player) = target_player else { return; };
        if !ffa && player.team(self) == local_team { return; }

        // Head-only check
        if tb.head_only {
            let head = player.bone_position(self, Bones::Head.u64());
            let target_angle = self.angle_to_target(&local_player, &head, &Vec2::ZERO);
            let view_angles = local_player.view_angles(self);
            let fov = angles_to_fov(&view_angles, &target_angle);

            let head_radius_fov =
                3.5 / (local_player.position(self) - player.position(self)).length().max(1.0) * 100.0;

            if fov > head_radius_fov {
                return;
            }
        }

        // Visibility + autowall damage check
        let target_visible = player.visible(self, &local_player);
        if !target_visible {
            if !autowall_on {
                return;
            }
            if config.aim.autowall_safe {
                return;
            }

            let eye = local_player.eye_position(self);
            let target_pos = player.bone_position(self, Bones::Head.u64());
            let num_walls = self.count_walls(eye, target_pos).unwrap_or(1).max(1);

            let weapon_class = local_player.weapon_class(self);
            let final_damage = calculate_penetration_damage(&weapon_class, num_walls);
            let target_health = player.health(self).max(1) as f32;
            let damage_ratio = final_damage / target_health;

            if damage_ratio < 1.0 {
                return;
            }
        }

        // When magnet is active only shoot if the crosshair landed on the enemy
        // naturally — magnet pulls aim there, the crosshair entity check confirms it.
        if tb.magnet_enabled && !config.parental_lock && !found_via_crosshair {
            return;
        }

        // Shot scheduling — enforce minimum delay under parental lock
        let delay_start = if config.parental_lock {
            tb.delay.start().max(&safe_limits::DELAY_MIN)
        } else {
            tb.delay.start()
        };
        let delay_end = if config.parental_lock {
            tb.delay.end().max(&safe_limits::DELAY_MIN)
        } else {
            tb.delay.end()
        };
        let mean = (*delay_start + *delay_end) as f32 / 2.0;
        let std_dev = (*delay_end - *delay_start) as f32 / 2.0;

        let normal = rand_distr::Normal::new(mean, std_dev).unwrap();
        use rand_distr::Distribution as _;
        let delay = normal.sample(&mut rng()).max(0.0) as u64;

        let now = Instant::now();
        let delay_dur = Duration::from_millis(delay);
        self.trigger.shot_start = Some(now + delay_dur);
        self.trigger.shot_end = Some(now + delay_dur + Duration::from_millis(tb.shot_duration));
    }

    /// Standalone magnet aim-pull. Runs independently of triggerbot — active
    /// whenever `magnet_enabled` is true, regardless of triggerbot state.
    pub fn magnet_trigger(&mut self, config: &Config, mouse: &mut Mouse) {
        let aim = &config.aim;
        let Some(local_player) = Player::local_player(self) else { return; };
        let current_weapon = local_player.weapon(self);

        let tb = if let Some(wcfg) = aim.weapons.get(&current_weapon) {
            &wcfg.triggerbot
        } else {
            &aim.global.triggerbot
        };

        if !tb.magnet_enabled {
            return;
        }

        let ffa = self.is_ffa();
        let local_team = local_player.team(self);
        let view_angles = local_player.view_angles(self);

        let mut best_fov = f32::MAX;
        let mut best_player = None;

        for &p in &self.players {
            if !ffa && p.team(self) == local_team { continue; }
            if p.health(self) <= 0 { continue; }

            let head = p.bone_position(self, Bones::Head.u64());
            let target_angle = self.angle_to_target(&local_player, &head, &Vec2::ZERO);
            let fov = angles_to_fov(&view_angles, &target_angle);

            if fov < tb.magnet_fov && fov < best_fov {
                best_fov = fov;
                best_player = Some(p);
            }
        }

        let Some(player) = best_player else { return; };

        let head = player.bone_position(self, Bones::Head.u64());
        let target_angle = self.angle_to_target(&local_player, &head, &Vec2::ZERO);

        let mut delta = target_angle - view_angles;
        delta.y = ((delta.y + 180.0) % 360.0) - 180.0;
        delta.x = delta.x.clamp(-89.0, 89.0);

        let strength = tb.magnet_strength * tb.magnet_smoothing;
        let mut pull = delta * strength;
        pull.y *= tb.magnet_vertical_scale;

        let dx = pull.x * 1.15;
        let dy = pull.y * 0.95;

        if dx.abs() > 0.1 || dy.abs() > 0.1 {
            mouse.move_rel(&Vec2::new(dx, dy));
        }
    }

    pub fn triggerbot_shoot(&mut self, mouse: &mut Mouse) {
        let now = Instant::now();

        if let Some(shot_time) = self.trigger.shot_start
            && now >= shot_time
        {
            mouse.left_press();
            self.trigger.shot_start = None;
        }

        if let Some(shot_end) = self.trigger.shot_end
            && now >= shot_end
        {
            mouse.left_release();
            self.trigger.shot_end = None;
        }
    }
}
