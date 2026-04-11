use std::collections::HashMap;

use glam::{IVec2, Mat4, Vec2, Vec3};
use rand::RngExt;

use crate::{
    config::AdvancedMagnetConfig,
    cs2::{
        CS2,
        bones::Bones,
        entity::player::Player,
    },
    math::angles_to_fov,
    os::mouse::Mouse,
};

#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub player: Player,
    pub bone: Bones,
    pub position: Vec3,
    pub damage_potential: f32,
    pub hit_chance: f32,
    pub threat_score: f32,
    pub distance: f32,
    pub walls: u32,
}

#[derive(Debug, Clone)]
pub struct PredictedPosition {
    pub position: Vec3,
    pub velocity: Vec3,
    pub confidence: f32,
}

#[derive(Debug)]
pub struct AdvancedMagnet {
    pub active: bool,
    pub current_target: Option<TargetInfo>,
    pub last_target_time: Option<std::time::Instant>,
    pub target_history: HashMap<u64, Vec<Vec3>>,
}

impl AdvancedMagnet {
    pub fn new() -> Self {
        Self {
            active: false,
            current_target: None,
            last_target_time: None,
            target_history: HashMap::new(),
        }
    }

    pub fn update(&mut self, cs2: &mut CS2, config: &AdvancedMagnetConfig, mouse: &mut Mouse) {
        let players = cs2.players.clone();
        if players.is_empty() {
            self.active = false;
            self.reset();
            return;
        }
        self.update_with_data(&players, config, mouse, cs2);
    }

    pub fn update_with_data(&mut self, players: &[Player], config: &AdvancedMagnetConfig, mouse: &mut Mouse, cs2: &CS2) {
        self.active = true;

        let Some(local_player) = Player::local_player(cs2) else {
            self.current_target = None;
            return;
        };
        let local_eye = local_player.eye_position(cs2);
        if local_eye == Vec3::ZERO {
            self.current_target = None;
            return;
        }

        let view_angles = local_player.view_angles(cs2);
        let window_size = Self::read_window_size(cs2);
        let screen_center = window_size * 0.5;
        let view_matrix = cs2.process.read::<Mat4>(cs2.offsets.direct.view_matrix);

        let previous_target_id = self.current_target.as_ref().map(|t| t.player.pawn);
        let now = std::time::Instant::now();

        let mut best_target: Option<TargetInfo> = None;
        let mut best_score = f32::MIN;

        let max_fov = config.fov.max(0.1);
        let max_distance = config.max_distance.max(1.0);

        for player in players {
            if !player.is_valid(cs2) {
                continue;
            }

            let target_id = player.pawn;
            let target_bone = self.select_best_bone(player, config, cs2);
            let bone_pos = match self.bone_position(player, target_bone, cs2) {
                Some(pos) => pos,
                None => continue,
            };

            self.push_history_point(target_id, bone_pos);

            let aim_pos = if config.prediction {
                let predicted = self.predict_position(target_id, bone_pos, config);
                if predicted.confidence < 0.15 {
                    continue;
                }
                predicted.position
            } else {
                bone_pos
            };

            let distance = local_eye.distance(aim_pos);
            if !distance.is_finite() || distance > max_distance {
                continue;
            }

            let fov = self.compute_fov(local_eye, view_angles, aim_pos);
            if fov > max_fov {
                continue;
            }

            let walls = cs2.count_walls(local_eye, aim_pos).unwrap_or(0);
            let damage_potential = self.estimate_damage_potential(distance, walls, target_bone, config);
            let hit_chance = self.estimate_hit_chance(fov, distance, 1.0, walls, config);
            let threat_score = self.estimate_threat(player, distance, config, cs2, &local_player);

            let sticky_bonus = if previous_target_id == Some(target_id)
                && self.last_target_time
                    .map(|t| now.duration_since(t) <= std::time::Duration::from_millis(180))
                    .unwrap_or(false)
            {
                0.2
            } else {
                0.0
            };

            let target_score = self.score_target(
                damage_potential,
                hit_chance,
                threat_score,
                fov,
                distance,
                sticky_bonus,
                walls,
                config,
            );

            if target_score > best_score {
                best_score = target_score;
                best_target = Some(TargetInfo {
                    player: *player,
                    bone: target_bone,
                    position: aim_pos,
                    damage_potential,
                    hit_chance,
                    threat_score,
                    distance,
                    walls,
                });
            }
        }

        self.current_target = best_target;

        if let Some(target) = &self.current_target {
            self.last_target_time = Some(now);

            if let Some(target_screen) = Self::world_to_screen(&target.position, &view_matrix, window_size) {
                let delta = target_screen - screen_center;
                let movement = self.compute_mouse_delta(delta, config);

                if movement.length_squared() > 0.0 {
                    mouse.move_rel(&movement);
                }
            }
        }
    }

    // --- helpers ---

    fn push_history_point(&mut self, target_id: u64, pos: Vec3) {
        let history = self.target_history.entry(target_id).or_default();
        history.push(pos);
        const HISTORY_LIMIT: usize = 12;
        if history.len() > HISTORY_LIMIT {
            let overflow = history.len() - HISTORY_LIMIT;
            history.drain(0..overflow);
        }
    }

    fn predict_position(&self, target_id: u64, current_pos: Vec3, config: &AdvancedMagnetConfig) -> PredictedPosition {
        let Some(history) = self.target_history.get(&target_id) else {
            return PredictedPosition {
                position: current_pos,
                velocity: Vec3::ZERO,
                confidence: 0.35,
            };
        };
        if history.len() < 2 {
            return PredictedPosition {
                position: current_pos,
                velocity: Vec3::ZERO,
                confidence: 0.45,
            };
        }
        let mut velocity_sum = Vec3::ZERO;
        let mut samples = 0.0;
        for window in history.windows(2) {
            velocity_sum += window[1] - window[0];
            samples += 1.0;
        }
        let avg_velocity = if samples > 0.0 {
            velocity_sum / samples
        } else {
            Vec3::ZERO
        };
        let smoothing = config.prediction_time.clamp(0.0, 3.0);
        let predicted_pos = current_pos + avg_velocity * smoothing;
        let speed_stability = 1.0 / (1.0 + avg_velocity.length());
        let confidence = (0.55 + speed_stability * 0.45).clamp(0.0, 1.0);
        PredictedPosition {
            position: predicted_pos,
            velocity: avg_velocity,
            confidence,
        }
    }

    fn compute_fov(&self, local_eye: Vec3, view_angles: Vec2, target_pos: Vec3) -> f32 {
        let delta = target_pos - local_eye;
        let aim_angles = crate::math::angles_from_vector(&delta);
        angles_to_fov(&view_angles, &aim_angles)
    }

    fn score_target(&self, damage_potential: f32, hit_chance: f32, threat_score: f32, fov: f32, distance: f32, sticky_bonus: f32, walls: u32, config: &AdvancedMagnetConfig) -> f32 {
        let fov_score = 1.0 / (1.0 + fov.max(0.0));
        let dist_score = 1.0 / (1.0 + (distance / 1200.0));
        let wall_penalty = walls as f32 * 0.25;
        let aggression = config.aggression.clamp(0.0, 1.0);
        (damage_potential * 0.30 * aggression)
            + (hit_chance * 0.35)
            + (threat_score * 0.20)
            + (fov_score * 0.25)
            + (dist_score * 0.10)
            + sticky_bonus
            - wall_penalty
    }

    fn estimate_damage_potential(&self, distance: f32, walls: u32, bone: Bones, config: &AdvancedMagnetConfig) -> f32 {
        let bone_multiplier = match bone {
            Bones::Head => 1.0,
            Bones::Neck => 0.85,
            _ => 0.55,
        };
        let distance_falloff = (1.0 - (distance / config.max_distance.max(1.0))).clamp(0.15, 1.0);
        let wall_falloff = (1.0 - walls as f32 * 0.20 / config.wall_penetration_bonus.max(0.01)).clamp(0.05, 1.0);
        bone_multiplier * distance_falloff * wall_falloff
    }

    fn estimate_hit_chance(&self, fov: f32, distance: f32, confidence: f32, walls: u32, config: &AdvancedMagnetConfig) -> f32 {
        let fov_factor = (1.0 - (fov / config.fov.max(0.1))).clamp(0.0, 1.0);
        let distance_factor = (1.0 - (distance / config.max_distance.max(1.0))).clamp(0.15, 1.0);
        let wall_factor = (1.0 - walls as f32 * 0.18).clamp(0.05, 1.0);
        fov_factor * distance_factor * confidence * wall_factor
    }

    fn estimate_threat(&self, player: &Player, distance: f32, _config: &AdvancedMagnetConfig, cs2: &CS2, local_player: &Player) -> f32 {
        let proximity = (1.0 - (distance / 2500.0)).clamp(0.0, 1.0);
        let visible_bonus = if player.visible(cs2, local_player) { 0.3 } else { 0.0 };
        let scoped_bonus = if player.is_scoped(cs2) { 0.15 } else { 0.0 };
        (proximity + visible_bonus + scoped_bonus).clamp(0.0, 1.0)
    }

    fn select_best_bone(&self, player: &Player, config: &AdvancedMagnetConfig, cs2: &CS2) -> Bones {
        for &bone in &config.priority_bones {
            if player.bone_position(cs2, bone.u64()) != Vec3::ZERO {
                return bone;
            }
        }
        Bones::Head
    }

    fn compute_mouse_delta(&self, screen_delta: Vec2, config: &AdvancedMagnetConfig) -> Vec2 {
        if config.instant_snap {
            return screen_delta;
        }
        let smoothing = (1.0 / config.smooth_factor.clamp(0.01, 1.0)).max(1.0);
        let base = Vec2::new(
            screen_delta.x,
            screen_delta.y * config.vertical_multiplier,
        );
        base / smoothing
    }

    fn read_window_size(cs2: &CS2) -> Vec2 {
        let sdl_window = cs2.process.read::<u64>(cs2.offsets.direct.sdl_window);
        if sdl_window == 0 {
            return Vec2::new(1920.0, 1080.0);
        }
        let size = cs2.process.read::<IVec2>(sdl_window + 0x18 + 0x08).as_vec2();
        if size.x < 1.0 || size.y < 1.0 {
            Vec2::new(1920.0, 1080.0)
        } else {
            size
        }
    }

    fn world_to_screen(world: &Vec3, vm: &Mat4, window_size: Vec2) -> Option<Vec2> {
        let mut screen = Vec2::new(
            vm.x_axis.x * world.x + vm.x_axis.y * world.y + vm.x_axis.z * world.z + vm.x_axis.w,
            vm.y_axis.x * world.x + vm.y_axis.y * world.y + vm.y_axis.z * world.z + vm.y_axis.w,
        );
        let w = vm.w_axis.x * world.x + vm.w_axis.y * world.y + vm.w_axis.z * world.z + vm.w_axis.w;
        if w < 0.0001 {
            return None;
        }
        screen /= w;
        let half = window_size * 0.5;
        screen.x = half.x + 0.5 * screen.x * window_size.x + 0.5;
        screen.y = half.y - 0.5 * screen.y * window_size.y + 0.5;
        if screen.x < 0.0 || screen.x > window_size.x || screen.y < 0.0 || screen.y > window_size.y {
            return None;
        }
        Some(screen)
    }

    fn bone_position(&self, player: &Player, bone: Bones, cs2: &CS2) -> Option<Vec3> {
        let pos = player.bone_position(cs2, bone.u64());
        if pos != Vec3::ZERO { Some(pos) } else { None }
    }

    pub fn reset(&mut self) {
        self.current_target = None;
        self.last_target_time = None;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn get_current_target(&self) -> Option<&TargetInfo> {
        self.current_target.as_ref()
    }
}

impl Default for AdvancedMagnet {
    fn default() -> Self {
        Self::new()
    }
}
