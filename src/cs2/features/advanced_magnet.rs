use std::collections::{HashMap, HashSet, VecDeque};

use glam::{IVec2, Mat4, Vec2, Vec3};

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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    pub target_history: HashMap<u64, VecDeque<Vec3>>,
    cached_window_size: Option<Vec2>,
    frame_counter: u32,
}

// Scoring weights — must sum to 1.0 so scores stay normalised and tunable
const W_DAMAGE: f32 = 0.30;
const W_HIT:    f32 = 0.30;
const W_THREAT: f32 = 0.20;
const W_FOV:    f32 = 0.15;
const W_DIST:   f32 = 0.05;

const HISTORY_LIMIT:          usize = 12;
const CACHE_REFRESH_INTERVAL: u32   = 64;

// Sticky bonus is relative: only applied when the previous target's raw score
// is within this gap of the best challenger, preventing permanent lock-on.
const STICKY_THRESHOLD: f32 = 0.15;
const STICKY_BONUS:     f32 = 0.08;

#[allow(dead_code)]
impl AdvancedMagnet {
    pub fn new() -> Self {
        Self {
            active: false,
            current_target: None,
            last_target_time: None,
            target_history: HashMap::new(),
            cached_window_size: None,
            frame_counter: 0,
        }
    }

    pub fn update(&mut self, cs2: &mut CS2, config: &AdvancedMagnetConfig, mouse: &mut Mouse) {
        if cs2.players.is_empty() {
            self.active = false;
            self.reset();
            return;
        }
        // Player is Copy; collect to release the borrow on cs2 before passing &CS2 below.
        let players: Vec<Player> = cs2.players.iter().copied().collect();
        self.update_with_data(&players, config, mouse, cs2);
    }

    pub fn update_with_data(
        &mut self,
        players: &[Player],
        config: &AdvancedMagnetConfig,
        mouse: &mut Mouse,
        cs2: &CS2,
    ) {
        self.active = true;
        self.frame_counter = self.frame_counter.wrapping_add(1);

        let Some(local_player) = Player::local_player(cs2) else {
            self.current_target = None;
            return;
        };

        let local_eye = local_player.eye_position(cs2);
        if local_eye == Vec3::ZERO {
            self.current_target = None;
            return;
        }

        let view_angles   = local_player.view_angles(cs2);
        let window_size   = self.get_window_size(cs2);
        let screen_center = window_size * 0.5;
        let view_matrix   = cs2.process.read::<Mat4>(cs2.offsets.direct.view_matrix);

        let previous_target_id = self.current_target.as_ref().map(|t| t.player.pawn);
        let now = std::time::Instant::now();

        let sticky_active = previous_target_id.is_some()
            && self.last_target_time
                .map(|t| now.duration_since(t) <= std::time::Duration::from_millis(180))
                .unwrap_or(false);

        let max_fov      = config.fov.max(0.1);
        let max_distance = config.max_distance.max(1.0);

        // --- candidate collection ---

        struct Candidate {
            info:  TargetInfo,
            score: f32,
        }

        let mut candidates: Vec<Candidate> = Vec::with_capacity(players.len());

        for player in players {
            // 1. validity (cheapest)
            if !player.is_valid(cs2) {
                continue;
            }

            let target_id = player.pawn;

            // 2. bone + position in one pass — avoids reading the same bone twice
            let (target_bone, bone_pos) = match self.select_best_bone_with_pos(player, config, cs2) {
                Some(pair) => pair,
                None => continue,
            };

            // 3. update movement history for valid candidates
            self.push_history_point(target_id, bone_pos);

            // 4. resolve aim position; capture confidence for hit_chance below
            let (aim_pos, prediction_confidence) = if config.prediction {
                let predicted = self.predict_position(target_id, bone_pos, config);
                if predicted.confidence < 0.15 {
                    continue;
                }
                (predicted.position, predicted.confidence)
            } else {
                (bone_pos, 1.0)
            };

            // 5. distance (arithmetic only)
            let distance = local_eye.distance(aim_pos);
            if !distance.is_finite() || distance > max_distance {
                continue;
            }

            // 6. FOV (arithmetic only)
            let fov = self.compute_fov(local_eye, view_angles, aim_pos);
            if fov > max_fov {
                continue;
            }

            // 7. wall cast — most expensive; only reached after all prior gates
            let walls = cs2.count_walls(local_eye, aim_pos).unwrap_or(0);

            // prediction_confidence flows into hit_chance so uncertain predictions
            // are penalised proportionally rather than treated as certain.
            let damage_potential = self.estimate_damage_potential(distance, walls, target_bone, config);
            let hit_chance       = self.estimate_hit_chance(fov, distance, prediction_confidence, walls, config);
            let threat_score     = self.estimate_threat(player, distance, config, cs2, &local_player);

            let score = self.score_target(
                damage_potential,
                hit_chance,
                threat_score,
                fov,
                distance,
                walls,
                config,
            );

            candidates.push(Candidate {
                info: TargetInfo {
                    player: *player,
                    bone: target_bone,
                    position: aim_pos,
                    damage_potential,
                    hit_chance,
                    threat_score,
                    distance,
                    walls,
                },
                score,
            });
        }

        // --- target selection with relative sticky bonus ---

        let best_target = if candidates.is_empty() {
            None
        } else {
            let raw_best = candidates.iter().map(|c| c.score).fold(f32::NEG_INFINITY, f32::max);

            let mut chosen_idx   = 0;
            let mut chosen_score = f32::NEG_INFINITY;

            for (i, candidate) in candidates.iter().enumerate() {
                let mut effective = candidate.score;

                // Sticky bonus only applies when the previous target is within
                // STICKY_THRESHOLD of the best candidate — a clearly superior
                // challenger always wins.
                if sticky_active && previous_target_id == Some(candidate.info.player.pawn) {
                    if raw_best - candidate.score <= STICKY_THRESHOLD {
                        effective += STICKY_BONUS;
                    }
                }

                if effective > chosen_score {
                    chosen_score = effective;
                    chosen_idx   = i;
                }
            }

            Some(candidates.remove(chosen_idx).info)
        };

        self.current_target = best_target;

        if let Some(target) = &self.current_target {
            self.last_target_time = Some(now);

            if let Some(target_screen) = Self::world_to_screen(&target.position, &view_matrix, window_size) {
                let delta    = target_screen - screen_center;
                let movement = self.compute_mouse_delta(delta, config);

                if movement.length_squared() > 0.0 {
                    mouse.move_rel(&movement);
                }
            }
        }

        if self.frame_counter % CACHE_REFRESH_INTERVAL == 0 {
            self.evict_stale_history(players);
        }
    }

    // --- helpers ---

    fn get_window_size(&mut self, cs2: &CS2) -> Vec2 {
        if self.frame_counter % CACHE_REFRESH_INTERVAL == 0 || self.cached_window_size.is_none() {
            self.cached_window_size = Some(Self::read_window_size(cs2));
        }
        self.cached_window_size.unwrap()
    }

    fn evict_stale_history(&mut self, players: &[Player]) {
        let active_ids: HashSet<u64> = players.iter().map(|p| p.pawn).collect();
        self.target_history.retain(|id, _| active_ids.contains(id));
    }

    fn push_history_point(&mut self, target_id: u64, pos: Vec3) {
        let history = self.target_history.entry(target_id).or_default();
        if history.len() >= HISTORY_LIMIT {
            history.pop_front();
        }
        history.push_back(pos);
    }

    /// Returns `(bone, world_position)` in a single pass so each bone is read
    /// from the process exactly once instead of once for selection + once for use.
    fn select_best_bone_with_pos(
        &self,
        player: &Player,
        config: &AdvancedMagnetConfig,
        cs2: &CS2,
    ) -> Option<(Bones, Vec3)> {
        for &bone in &config.priority_bones {
            let pos = player.bone_position(cs2, bone.u64());
            if pos != Vec3::ZERO {
                return Some((bone, pos));
            }
        }
        // Fallback in case Head was not in priority_bones
        let pos = player.bone_position(cs2, Bones::Head.u64());
        if pos != Vec3::ZERO { Some((Bones::Head, pos)) } else { None }
    }

    fn predict_position(
        &self,
        target_id: u64,
        current_pos: Vec3,
        config: &AdvancedMagnetConfig,
    ) -> PredictedPosition {
        let Some(history) = self.target_history.get(&target_id) else {
            return PredictedPosition { position: current_pos, velocity: Vec3::ZERO, confidence: 0.35 };
        };

        if history.len() < 2 {
            return PredictedPosition { position: current_pos, velocity: Vec3::ZERO, confidence: 0.45 };
        }

        // Linearly-weighted velocity average: recent deltas count more than old ones.
        // history front = oldest, back = newest.
        let mut velocity_sum = Vec3::ZERO;
        let mut weight_sum   = 0.0_f32;

        for (i, (a, b)) in history.iter().zip(history.iter().skip(1)).enumerate() {
            let weight = (i + 1) as f32;
            velocity_sum += (b - a) * weight;
            weight_sum   += weight;
        }

        let avg_velocity  = velocity_sum / weight_sum;
        let smoothing     = config.prediction_time.clamp(0.0, 3.0);
        let predicted_pos = current_pos + avg_velocity * smoothing;

        // Fast-moving targets are harder to predict — confidence decreases with speed
        let speed_stability = 1.0 / (1.0 + avg_velocity.length());
        let confidence = (0.55 + speed_stability * 0.45).clamp(0.0, 1.0);

        PredictedPosition {
            position: predicted_pos,
            velocity: avg_velocity,
            confidence,
        }
    }

    fn compute_fov(&self, local_eye: Vec3, view_angles: Vec2, target_pos: Vec3) -> f32 {
        let delta      = target_pos - local_eye;
        let aim_angles = crate::math::angles_from_vector(&delta);
        angles_to_fov(&view_angles, &aim_angles)
    }

    /// Normalised score in roughly [0, 1].
    /// Weights sum to 1.0. `aggression` uniformly scales both offensive terms
    /// (damage + hit) so its effect is consistent and predictable.
    fn score_target(
        &self,
        damage_potential: f32,
        hit_chance: f32,
        threat_score: f32,
        fov: f32,
        distance: f32,
        walls: u32,
        config: &AdvancedMagnetConfig,
    ) -> f32 {
        let fov_score    = 1.0 / (1.0 + fov.max(0.0));
        let dist_score   = 1.0 / (1.0 + (distance / 1200.0));
        let wall_penalty = walls as f32 * 0.25;
        let aggression   = config.aggression.clamp(0.0, 1.0);

        // aggression scales both offensive terms equally
        let offensive = (damage_potential * W_DAMAGE + hit_chance * W_HIT) * aggression;

        offensive
            + threat_score * W_THREAT
            + fov_score    * W_FOV
            + dist_score   * W_DIST
            - wall_penalty
    }

    fn estimate_damage_potential(
        &self,
        distance: f32,
        walls: u32,
        bone: Bones,
        config: &AdvancedMagnetConfig,
    ) -> f32 {
        let bone_multiplier = match bone {
            Bones::Head => 1.0,
            Bones::Neck => 0.85,
            _           => 0.55,
        };
        let distance_falloff = (1.0 - (distance / config.max_distance.max(1.0))).clamp(0.15, 1.0);
        let wall_falloff     = (1.0 - walls as f32 * 0.20 / config.wall_penetration_bonus.max(0.01)).clamp(0.05, 1.0);
        bone_multiplier * distance_falloff * wall_falloff
    }

    /// `confidence` is 1.0 for non-predicted targets and the prediction
    /// confidence otherwise, so uncertain predictions reduce hit_chance
    /// proportionally rather than being treated as certain.
    fn estimate_hit_chance(
        &self,
        fov: f32,
        distance: f32,
        confidence: f32,
        walls: u32,
        config: &AdvancedMagnetConfig,
    ) -> f32 {
        let fov_factor      = (1.0 - (fov / config.fov.max(0.1))).clamp(0.0, 1.0);
        let distance_factor = (1.0 - (distance / config.max_distance.max(1.0))).clamp(0.15, 1.0);
        let wall_factor     = (1.0 - walls as f32 * 0.18).clamp(0.05, 1.0);
        fov_factor * distance_factor * confidence * wall_factor
    }

    fn estimate_threat(
        &self,
        player: &Player,
        distance: f32,
        _config: &AdvancedMagnetConfig,
        cs2: &CS2,
        local_player: &Player,
    ) -> f32 {
        let proximity     = (1.0 - (distance / 2500.0)).clamp(0.0, 1.0);
        let visible_bonus = if player.visible(cs2, local_player) { 0.3 } else { 0.0 };
        let scoped_bonus  = if player.is_scoped(cs2) { 0.15 } else { 0.0 };
        (proximity + visible_bonus + scoped_bonus).clamp(0.0, 1.0)
    }

    fn compute_mouse_delta(&self, screen_delta: Vec2, config: &AdvancedMagnetConfig) -> Vec2 {
        if config.instant_snap {
            return screen_delta;
        }
        let smoothing = (1.0 / config.smooth_factor.clamp(0.01, 1.0)).max(1.0);
        Vec2::new(
            screen_delta.x,
            screen_delta.y * config.vertical_multiplier,
        ) / smoothing
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

    /// Correct NDC → screen pixel conversion.
    /// After perspective divide: NDC x ∈ [-1, 1] left→right,
    ///                           NDC y ∈ [-1, 1] bottom→top (flip for screen space).
    fn world_to_screen(world: &Vec3, vm: &Mat4, window_size: Vec2) -> Option<Vec2> {
        let clip_x = vm.x_axis.x * world.x + vm.x_axis.y * world.y + vm.x_axis.z * world.z + vm.x_axis.w;
        let clip_y = vm.y_axis.x * world.x + vm.y_axis.y * world.y + vm.y_axis.z * world.z + vm.y_axis.w;
        let w      = vm.w_axis.x * world.x + vm.w_axis.y * world.y + vm.w_axis.z * world.z + vm.w_axis.w;

        if w < 0.0001 {
            return None;
        }

        let half  = window_size * 0.5;
        let screen = Vec2::new(
            (clip_x / w + 1.0) * half.x,  // NDC [-1,1] → [0, width]
            (1.0 - clip_y / w) * half.y,  // NDC [-1,1] → [0, height], Y flipped
        );

        if screen.x < 0.0 || screen.x > window_size.x
        || screen.y < 0.0 || screen.y > window_size.y {
            return None;
        }

        Some(screen)
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
