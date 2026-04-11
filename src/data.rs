use std::collections::{HashMap, VecDeque};

use glam::{Mat4, Vec2, Vec3};
use serde::Serialize;

use crate::cs2::{
    bones::Bones,
    entity::{EntityInfo, weapon::Weapon},
};

/// Whether the surface directly in front of the player can be wallbanged.
#[derive(Debug, Clone, Default, Serialize)]
pub enum PenetrationCrosshairState {
    /// Can wallbang — damage through walls meets the kill threshold.
    CanWallbang,
    /// Cannot wallbang — no penetration capability or damage too low.
    CannotWallbang,
    /// Map geometry data unavailable or raycast failed (e.g., BVH not loaded).
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SpectatorEntry {
    pub name: String,
    pub target: String,
    pub watching_local: bool,
}

#[derive(Debug, Clone)]
pub struct BacktrackRecord {
    pub bones: HashMap<Bones, Vec3>,
}

#[derive(Debug, Default, Serialize)]
pub struct Data {
    pub in_game: bool,
    /// Raw button bitmask from the engine (m_nButtons / IN_*).
    /// TODO: read from `m_nButtons` once the offset is available; 0 = unknown.
    pub buttons: u64,
    pub is_ffa: bool,
    pub is_custom_mode: bool,
    pub weapon: Weapon,
    pub players: Vec<PlayerData>,
    pub friendlies: Vec<PlayerData>,
    pub local_player: PlayerData,
    pub entities: Vec<EntityInfo>,
    pub spectators: Vec<SpectatorEntry>,
    pub bomb: BombData,
    pub map_name: String,
    pub view_matrix: Mat4,
    pub view_angles: Vec2,
    pub window_position: Vec2,
    pub window_size: Vec2,
    pub aimbot_active: bool,
    pub triggerbot_active: bool,
    pub autowall_active: bool,

    pub esp_active: bool,
    pub ping: i32,
    pub penetration_crosshair_state: PenetrationCrosshairState,
    #[serde(skip)]
    pub backtrack_history: HashMap<u64, VecDeque<BacktrackRecord>>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PlayerData {
    pub steam_id: u64,
    pub pawn: u64,
    pub health: i32,
    pub armor: i32,
    pub position: Vec3,
    /// True eye-level position (origin + m_vecViewOffset).
    pub eye_pos: Vec3,
    pub head: Vec3,
    pub name: String,
    pub weapon: Weapon,
    pub bones: HashMap<Bones, Vec3>,
    pub has_defuser: bool,
    pub has_helmet: bool,
    pub has_bomb: bool,
    pub visible: bool,
    pub color: i32,
    pub rotation: f32,
    pub velocity: Vec3,
}

#[derive(Debug, Default, Serialize)]
pub struct BombData {
    pub planted: bool,
    pub timer: f32,
    pub being_defused: bool,
    pub position: Vec3,
    pub defuse_remain_time: f32,
}
