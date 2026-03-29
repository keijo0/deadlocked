use glam::Vec3;
use serde::Serialize;

use crate::cs2::{CS2, entity::player::Player};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Smoke {
    controller: u64,
}

impl Smoke {
    pub fn new(controller: u64) -> Self {
        Self { controller }
    }

    pub fn info(&self, cs2: &CS2) -> SmokeInfo {
        SmokeInfo {
            entity: self.controller,
            position: Player::entity(self.controller).position(cs2),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SmokeInfo {
    pub entity: u64,
    pub position: Vec3,
}

impl SmokeInfo {
    pub fn grenade(&self) -> super::GrenadeInfo {
        super::GrenadeInfo {
            entity: self.entity,
            position: self.position,
            name: "Smoke",
        }
    }
}
