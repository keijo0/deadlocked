use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use glam::{IVec2, Mat4, Vec2, Vec3};
use utils::log;

use crate::{
    config::{AimbotConfig, Config, KeyMode, RcsConfig, TriggerbotConfig},
    constants::cs2::{self, TEAM_CT, TEAM_T},
    cs2::{
        bones::Bones,
        entity::{
            Entity, EntityInfo, GrenadeInfo, planted_c4::PlantedC4, player::Player, weapon::Weapon,
        },
        features::{aimbot::Aimbot, esp_toggle::EspToggle, rcs::Recoil, triggerbot::Triggerbot},
        input::Input,
        offsets::Offsets,
        target::Target,
    },
    data::{Data, PlayerData, BacktrackRecord, SpectatorEntry, PenetrationCrosshairState},
    game::Game,
    math::{angles_from_vector, angles_to_fov, vec2_clamp},
    os::{
        frequency_defense::FrequencyDefense,
        mouse::Mouse,
        process::Process,
        stealth_reader::StealthReader,
    },
    parser::{bvh::Bvh, load_map},
};

pub mod bones;
pub mod entity;
mod features;
mod find_offsets;
mod input;
pub mod key_codes;
mod offsets;
mod schema;
mod target;

/// CS2 runs at 64 ticks per second.
pub const CS2_TICK_RATE: f32 = 64.0;

const WORLD_SCAN_INTERVAL: Duration = Duration::from_millis(50);
const BVH_CHECK_INTERVAL: Duration = Duration::from_secs(1);
const OFFSET_REFRESH_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub struct CS2 {
    is_valid: bool,
    process: Process,
    offsets: Offsets,
    input: Input,
    bvh: Option<Bvh>,
    current_bvh: String,
    target: Target,
    players: Vec<Player>,
    entities: Vec<Entity>,
    recoil: Recoil,
    aim: Aimbot,
    trigger: Triggerbot,
    esp: EspToggle,
    weapon: Weapon,
    planted_c4: Option<PlantedC4>,
    next_world_scan: Instant,
    next_bvh_check: Instant,
    last_offset_update: Instant,
    pending_offsets: Option<Receiver<Option<Offsets>>>,
    frequency_defense: FrequencyDefense,
    stealth_reader: StealthReader,
    advanced_magnet: crate::cs2::features::advanced_magnet::AdvancedMagnet,
}

impl Game for CS2 {
    fn is_valid(&self) -> bool {
        self.is_valid && self.process.is_valid()
    }

    fn setup(&mut self) {
        let Some(process) = Process::open(cs2::PROCESS_NAME) else {
            self.is_valid = false;
            return;
        };
        log::info!("process found, pid: {}", process.pid);
        self.process = process;

        self.offsets = match CS2::find_offsets(&self.process) {
            Some(offsets) => offsets,
            None => {
                self.process = Process::new(-1);
                self.is_valid = false;
                return;
            }
        };
        log::info!("offsets found");
        self.last_offset_update = Instant::now();

        // Reset and reconfigure anti-detection subsystems for the new process.
        self.frequency_defense.reset();
        self.stealth_reader = StealthReader::new(
            self.process.pid,
            self.frequency_defense.shared_counter(),
        );
        self.stealth_reader
            .start_noise_thread(self.process.min, self.process.max);

        self.is_valid = true;
    }

    fn run(&mut self, config: &Config, mouse: &mut Mouse) {
        // Evict stale memory-cache entries from the previous tick.
        self.process.evict_cache();

        // Evaluate syscall frequency; sleep if suspicious activity is detected.
        let defense_state = self.frequency_defense.evaluate();
        let throttle = self.frequency_defense.throttle_delay();
        if !throttle.is_zero() {
            std::thread::sleep(throttle);
        }

        // Apply a small pseudo-random per-frame jitter to vary the syscall cadence.
        let jitter = self.stealth_reader.frame_jitter();
        if !jitter.is_zero() {
            std::thread::sleep(jitter);
        }

        if defense_state == crate::os::frequency_defense::DefenseState::Throttled {
            return;
        }

        if !self.process.is_valid() {
            self.is_valid = false;
            log::debug!("process is no longer valid");
            return;
        }

        if let Some(receiver) = &self.pending_offsets {
            match receiver.try_recv() {
                Ok(Some(new_offsets)) => {
                    self.offsets = new_offsets;
                    self.last_offset_update = Instant::now();
                    self.pending_offsets = None;
                    log::info!("offsets refreshed");
                }
                Ok(None) => {
                    self.pending_offsets = None;
                    self.is_valid = false;
                    log::warn!("failed to refresh offsets, reconnecting");
                    return;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.pending_offsets = None;
                    self.is_valid = false;
                    log::warn!("offset refresh thread died, reconnecting");
                    return;
                }
            }
        }

        if self.pending_offsets.is_none()
            && self.last_offset_update.elapsed() > OFFSET_REFRESH_INTERVAL
        {
            let pid = self.process.pid;
            let (tx, rx) = mpsc::channel();
            self.pending_offsets = Some(rx);
            std::thread::spawn(move || {
                let process = Process::new(pid);
                let _ = tx.send(CS2::find_offsets(&process));
            });
            log::debug!("offset refresh started in background");
        }

        self.input.update(&self.process, &self.offsets);

        let now = Instant::now();
        let requires_strict_occlusion = self.aimbot_config(config).smoke_wall_check;
        let world_scan_enabled = config.hud.bomb_timer
            || config.hud.dropped_weapons
            || config.hud.grenade_trails
            || requires_strict_occlusion;

        if world_scan_enabled && now >= self.next_world_scan {
            self.cache_entities();
            self.next_world_scan = now + WORLD_SCAN_INTERVAL;
        } else {
            self.cache_players();
            if !world_scan_enabled {
                self.entities.clear();
                self.planted_c4 = None;
            }
        }

        if now >= self.next_bvh_check {
            self.check_bvh();
            self.next_bvh_check = now + BVH_CHECK_INTERVAL;
        }

        self.esp_toggle(config);

        self.rcs(config, mouse);
        self.triggerbot(config, mouse);
        self.magnet_trigger(config, mouse);
        self.advanced_magnet_trigger(config, mouse);

        self.triggerbot_shoot(mouse);

        self.find_target(config);
        self.update_backtrack(config);

        // No-flash and no-smoke features (unsafe)
        self.no_flash(config);
        self.no_smoke(config);

        self.aimbot(config, mouse);
    }

    fn data(&self, config: &Config, data: &mut Data) {
        data.players.clear();
        data.friendlies.clear();
        data.entities.clear();

        let sdl_window = self.process.read::<u64>(self.offsets.direct.sdl_window);
        if sdl_window == 0 {
            data.window_position = Vec2::ZERO;
            data.window_size = Vec2::ONE;
        } else {
            data.window_position = self.process.read::<IVec2>(sdl_window + 0x18).as_vec2();
            data.window_size = self
                .process
                .read::<IVec2>(sdl_window + 0x18 + 0x08)
                .as_vec2();
        }

        let Some(local_player) = Player::local_player(self) else {
            data.weapon = Weapon::default();
            data.in_game = false;
            return;
        };
        let local_team = local_player.team(self);
        if local_team != TEAM_T && local_team != TEAM_CT {
            data.weapon = Weapon::default();
            data.in_game = false;
            return;
        }

        let local_pawn = local_player.pawn;
        data.spectators.clear();
        for i in 1..=64 {
            if let Some(player) = Player::index(self, i) {
                if let Some(target) = player.spectator_target(self) {
                    let watching_local = target.pawn == local_pawn;
                    let target_name = if !watching_local {
                        // Try to find target name from known players
                        self.players
                            .iter()
                            .find(|p| p.pawn == target.pawn)
                            .map(|p| p.name(self))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    data.spectators.push(SpectatorEntry {
                        name: player.name(self),
                        target: target_name,
                        watching_local,
                    });
                }
            }
        }

        for player in &self.players {
            let player_data = PlayerData {
                steam_id: player.steam_id(self),
                pawn: player.pawn,
                health: player.health(self),
                armor: player.armor(self),
                position: player.position(self),
                eye_pos: Vec3::ZERO, // only populated for local player
                head: player.bone_position(self, Bones::Head.u64()),
                name: player.name(self),
                weapon: player.weapon(self),
                bones: player.all_bones(self),
                has_defuser: player.has_defuser(self),
                has_helmet: player.has_helmet(self),
                has_bomb: player.has_bomb(self),
                visible: player.visible(self, &local_player),
                color: player.color(self),
                rotation: player.rotation(self),
                velocity: Vec3::ZERO,
            };

            if !self.is_ffa() && player.team(self) == local_team {
                data.friendlies.push(player_data);
            } else {
                data.players.push(player_data);
            }
        }

        data.local_player = PlayerData {
            steam_id: local_player.steam_id(self),
            pawn: local_player.pawn,
            health: local_player.health(self),
            armor: local_player.armor(self),
            position: local_player.position(self),
            eye_pos: local_player.eye_position(self),
            head: local_player.bone_position(self, Bones::Head.u64()),
            name: local_player.name(self),
            weapon: local_player.weapon(self),
            bones: local_player.all_bones(self),
            has_defuser: local_player.has_defuser(self),
            has_helmet: local_player.has_helmet(self),
            has_bomb: local_player.has_bomb(self),
            visible: true,
            color: local_player.color(self),
            rotation: local_player.rotation(self),
            velocity: local_player.velocity(self),
        };

        data.entities = self
            .entities
            .iter()
            .map(|e| match e {
                Entity::Weapon { weapon, entity } => EntityInfo::Weapon {
                    weapon: weapon.clone(),
                    position: Player::entity(*entity).position(self),
                },
                Entity::Inferno(inferno) => EntityInfo::Inferno(inferno.info(self)),
                Entity::Smoke(smoke) => EntityInfo::Smoke(smoke.info(self)),
                Entity::Molotov(molotov) => EntityInfo::Molotov(molotov.info(self)),
                Entity::Flashbang(entity) => {
                    EntityInfo::Flashbang(GrenadeInfo::new(*entity, "Flashbang", self))
                }
                Entity::HeGrenade(entity) => {
                    EntityInfo::HeGrenade(GrenadeInfo::new(*entity, "HE Grenade", self))
                }
                Entity::Decoy(entity) => {
                    EntityInfo::Decoy(GrenadeInfo::new(*entity, "Decoy", self))
                }
            })
            .collect();

        data.weapon = local_player.weapon(self);
        data.in_game = true;
        data.is_ffa = self.is_ffa();
        data.is_custom_mode = self.is_custom_game_mode();
        data.map_name = self.current_map();
        data.ping = local_player.ping(self);
        data.aimbot_active = {
            let aimbot_config = self.aimbot_config(config);
            aimbot_config.enabled
                && match aimbot_config.mode {
                    KeyMode::Toggle => self.aim.active,
                    KeyMode::Hold => config
                        .aim
                        .aimbot_hotkeys
                        .iter()
                        .any(|k| self.input.is_key_pressed(*k)),
                }
        };
        data.triggerbot_active = {
            let triggerbot_config = self.triggerbot_config(config);
            triggerbot_config.enabled
                && match triggerbot_config.mode {
                    KeyMode::Toggle => self.trigger.active,
                    KeyMode::Hold => self.input.is_key_pressed(config.aim.triggerbot_hotkey),
                }
        };
        data.autowall_active = data.triggerbot_active && {
            config.aim.autowall_enabled && match config.aim.autowall_mode {
                KeyMode::Hold => self.input.is_key_pressed(config.aim.autowall_hotkey),
                KeyMode::Toggle => self.trigger.autowall_active,
            }
        };
        data.esp_active = self.esp_enabled(config);

        data.view_matrix = self.process.read::<Mat4>(self.offsets.direct.view_matrix);
        data.view_angles = local_player.view_angles(self);

        data.backtrack_history = self.target.backtrack_history.clone();

        // Penetration crosshair: find the nearest enemy near the crosshair and check
        // whether the bullet can penetrate through to them.
        if config.hud.penetration_crosshair_enabled {
            use crate::cs2::features::triggerbot::calculate_penetration_damage;

            let eye = local_player.eye_position(self);
            let view_angles = local_player.view_angles(self);
            let weapon_class = local_player.weapon_class(self);
            let ffa = self.is_ffa();
            let local_team = local_player.team(self);
            let local_pos = local_player.position(self);

            // Scan for the nearest enemy under the crosshair (same FOV-radius formula
            // as the triggerbot autowall scan).
            let mut best_enemy: Option<Player> = None;
            let mut best_fov = f32::MAX;
            for &p in &self.players {
                if !ffa && p.team(self) == local_team {
                    continue;
                }
                let head = p.bone_position(self, Bones::Head.u64());
                let angle = self.angle_to_target(&local_player, &head, &Vec2::ZERO);
                let fov = angles_to_fov(&view_angles, &angle);
                let dist = (local_pos - p.position(self)).length().max(1.0);
                let radius = 3.5 / dist * 100.0;
                if fov < radius && fov < best_fov {
                    best_fov = fov;
                    best_enemy = Some(p);
                }
            }

            data.penetration_crosshair_state = if let Some(enemy) = best_enemy {
                let target_pos = enemy.bone_position(self, Bones::Head.u64());
                match self.count_walls(eye, target_pos) {
                    None => PenetrationCrosshairState::Unavailable,
                    // Clear line of sight — no penetration needed, can always shoot.
                    Some(0) => PenetrationCrosshairState::CanWallbang,
                    Some(walls) => {
                        let damage = calculate_penetration_damage(&weapon_class, walls);
                        let target_health = enemy.health(self).max(1) as f32;
                        if damage / target_health >= 1.0 {
                            PenetrationCrosshairState::CanWallbang
                        } else {
                            PenetrationCrosshairState::CannotWallbang
                        }
                    }
                }
            } else {
                PenetrationCrosshairState::Unavailable
            };
        }
        if let Some(bomb) = &self.planted_c4 {
            data.bomb.planted = bomb.is_planted(self);
            data.bomb.timer = bomb.time_to_explosion(self);
            data.bomb.position = bomb.position(self);
            data.bomb.being_defused = bomb.is_being_defused(self);
            data.bomb.defuse_remain_time = bomb.time_to_defuse(self);
        } else {
            data.bomb.planted = false;
        }
    }
}

impl CS2 {
    pub fn new() -> Self {
        let frequency_defense = FrequencyDefense::new();
        let stealth_reader = StealthReader::new(-1, frequency_defense.shared_counter());
        Self {
            is_valid: false,
            process: Process::new(-1),
            offsets: Offsets::default(),
            input: Input::new(),
            bvh: None,
            current_bvh: String::new(),
            target: Target::default(),
            players: Vec::with_capacity(64),
            entities: Vec::with_capacity(128),
            recoil: Recoil::default(),
            aim: Aimbot::default(),
            trigger: Triggerbot::default(),
            esp: EspToggle::default(),
            weapon: Weapon::default(),
            planted_c4: None,
            next_world_scan: Instant::now(),
            next_bvh_check: Instant::now(),
            last_offset_update: Instant::now(),
            pending_offsets: None,
            frequency_defense,
            stealth_reader,
            advanced_magnet: crate::cs2::features::advanced_magnet::AdvancedMagnet::new(),
        }
    }

    fn aimbot_config<'a>(&self, config: &'a Config) -> &'a AimbotConfig {
        if let Some(weapon_config) = config.aim.weapons.get(&self.weapon)
            && weapon_config.aimbot.enable_override
        {
            return &weapon_config.aimbot;
        }
        &config.aim.global.aimbot
    }

    fn rcs_config<'a>(&self, config: &'a Config) -> &'a RcsConfig {
        if let Some(weapon_config) = config.aim.weapons.get(&self.weapon)
            && weapon_config.rcs.enable_override
        {
            return &weapon_config.rcs;
        }
        &config.aim.global.rcs
    }

    fn triggerbot_config<'a>(&self, config: &'a Config) -> &'a TriggerbotConfig {
        if let Some(weapon_config) = config.aim.weapons.get(&self.weapon)
            && weapon_config.triggerbot.enable_override
        {
            return &weapon_config.triggerbot;
        }
        &config.aim.global.triggerbot
    }

    fn angle_to_target(&self, local_player: &Player, position: &Vec3, aim_punch: &Vec2) -> Vec2 {
        let eye_position = local_player.eye_position(self);
        let forward = (position - eye_position).normalize();

        let mut angles = angles_from_vector(&forward) - aim_punch;
        vec2_clamp(&mut angles);

        angles
    }

    fn entity_has_owner(&self, entity: u64) -> bool {
        self.process
            .read::<i32>(entity + self.offsets.controller.owner_entity)
            != -1
    }

    pub(crate) fn is_path_clear(&self, start: Vec3, end: Vec3) -> bool {
        if let Some(bvh) = &self.bvh {
            if !bvh.has_line_of_sight(start, end) {
                return false;
            }
        } else {
            // FALLBACK: If we cannot validate geometry LOS, assume clear to ensure cheat works
            log::debug!("No BVH available, assuming path is clear");
        }

        !self.segment_hits_smoke(start, end)
    }

    /// Returns the number of distinct wall/obstacle clusters between `start` and `end`.
    /// Returns `None` if map geometry data is unavailable.
    pub(crate) fn count_walls(&self, start: Vec3, end: Vec3) -> Option<u32> {
        let bvh = self.bvh.as_ref()?;
        Some(bvh.count_wall_intersections(start, end))
    }

    fn segment_hits_smoke(&self, start: Vec3, end: Vec3) -> bool {
        const SMOKE_RADIUS: f32 = 145.0;
        const SMOKE_RADIUS_SQ: f32 = SMOKE_RADIUS * SMOKE_RADIUS;

        let segment = end - start;
        let segment_len_sq = segment.length_squared();

        for entity in &self.entities {
            let Entity::Smoke(smoke) = entity else {
                continue;
            };

            let center = smoke.info(self).position;
            let t = if segment_len_sq > f32::EPSILON {
                ((center - start).dot(segment) / segment_len_sq).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let closest = start + segment * t;

            if center.distance_squared(closest) <= SMOKE_RADIUS_SQ {
                return true;
            }
        }

        false
    }

    // convars
    fn get_sensitivity(&self) -> f32 {
        self.process.read(self.offsets.convar.sensitivity + 0x58)
    }

    fn is_ffa(&self) -> bool {
        self.process.read::<u8>(self.offsets.convar.ffa + 0x58) == 1
    }

    fn is_custom_game_mode(&self) -> bool {
        let map = self.current_map();
        map.starts_with("workshop/")
            || map.starts_with("custom/")
            || !map.starts_with("de_") && !map.starts_with("cs_")
    }

    fn current_time(&self) -> f32 {
        let global_vars: u64 = self.process.read(self.offsets.direct.global_vars);
        self.process.read(global_vars + 0x30)
    }

    fn current_map(&self) -> String {
        let global_vars: u64 = self.process.read(self.offsets.direct.global_vars);
        self.process
            .read_string(self.process.read(global_vars + 0x198))
    }

    
    pub fn no_flash(&self, config: &Config) {
        let Some(local_player) = Player::local_player(self) else {
            return;
        };

        if config.misc.no_flash {
            local_player.no_flash(self, config.misc.max_flash_alpha);
        }
    }

    pub fn no_smoke(&self, config: &Config) {
        if config.misc.no_smoke {
            for entity in &self.entities {
                if let Entity::Smoke(smoke) = entity {
                    smoke.disable(self);
                }
            }
        }

    }

    pub fn advanced_magnet_trigger(&mut self, config: &Config, mouse: &mut Mouse) {
        // Get weapon and config
        let current_weapon = if let Some(local_player) = Player::local_player(self) {
            local_player.weapon(self)
        } else {
            return;
        };

        let aim = &config.aim;
        let tb = if let Some(wcfg) = aim.weapons.get(&current_weapon) {
            &wcfg.triggerbot
        } else {
            &aim.global.triggerbot
        };

        let advanced_config = tb.advanced_magnet.clone();
        
        if !advanced_config.enabled {
            return;
        }

        // Check if hotkey is pressed
        let hotkey_pressed = match advanced_config.mode {
            KeyMode::Hold => self.input.is_key_pressed(config.aim.triggerbot_hotkey),
            KeyMode::Toggle => {
                if self.input.key_just_pressed(config.aim.triggerbot_hotkey) {
                    self.advanced_magnet.active = !self.advanced_magnet.active;
                }
                self.advanced_magnet.active
            }
        };

        if !hotkey_pressed {
            return;
        }

        // Update advanced magnet system with cloned data to avoid borrowing issues
        let players = self.players.clone();
        let mut magnet = std::mem::take(&mut self.advanced_magnet);
        magnet.update_with_data(&players, &advanced_config, mouse, self);
        self.advanced_magnet = magnet;
    }

    fn update_backtrack(&mut self, config: &Config) {
        if !config.player.backtrack_visual {
            self.target.backtrack_history.clear();
            return;
        }
        let max_ticks = ((config.player.backtrack_ms as f32 / 1000.0) * CS2_TICK_RATE).round() as usize;

        let mut records: Vec<(u64, BacktrackRecord)> = Vec::with_capacity(self.players.len());
        for player in &self.players {
            let record = BacktrackRecord {
                bones: player.all_bones(self),
            };
            records.push((player.pawn, record));
        }

        let current_ids: HashSet<u64> = records.iter().map(|(id, _)| *id).collect();
        self.target.backtrack_history.retain(|id, _| current_ids.contains(id));

        for (steam_id, record) in records {
            let history = self.target.backtrack_history.entry(steam_id).or_default();
            history.push_front(record);
            history.truncate(max_ticks);
        }
    }

    fn check_bvh(&mut self) {
        let current_map = self.current_map();
        if current_map != self.current_bvh {
            self.bvh = load_map(&current_map);
            if self.bvh.is_some() {
                log::info!("loaded bvh for {current_map}");
                self.current_bvh = current_map;
            }
        }
    }
}
