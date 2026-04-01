use std::collections::{HashMap, VecDeque};

use glam::{Mat4, Vec2, Vec3};
use serde::Serialize;

use crate::cs2::{
    bones::Bones,
    entity::{EntityInfo, weapon::Weapon},
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum SoundType {
    Footstep,
    Gunshot,
    Weapon,
}

#[derive(Debug, Clone)]
pub struct BacktrackRecord {
    pub head: Vec3,
    pub bones: HashMap<Bones, Vec3>,
}

#[derive(Debug, Default, Serialize)]
pub struct Data {
    pub in_game: bool,
    pub is_ffa: bool,
    pub is_custom_mode: bool,
    pub weapon: Weapon,
    pub players: Vec<PlayerData>,
    pub friendlies: Vec<PlayerData>,
    pub local_player: PlayerData,
    pub entities: Vec<EntityInfo>,
    pub spectators: Vec<String>,
    pub bomb: BombData,
    pub map_name: String,
    pub view_matrix: Mat4,
    pub view_angles: Vec2,
    pub window_position: Vec2,
    pub window_size: Vec2,
    pub aimbot_active: bool,
    pub triggerbot_active: bool,
    pub esp_active: bool,
    pub ping: i32,
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
    pub sound: Option<SoundType>,
}

#[derive(Debug, Default, Serialize)]
pub struct BombData {
    pub planted: bool,
    pub timer: f32,
    pub being_defused: bool,
    pub position: Vec3,
    pub defuse_remain_time: f32,
}
