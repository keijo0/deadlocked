use glam::Vec3;

use crate::cs2::{entity::player::Player, CS2};

pub fn surfaceproperties_game(key: &str) -> f32 {
    match key {
        "$default" => 0.5,
        "$solidmetal" => 0.27,
        "$metal" => 0.4,
        "$metaldogtags" => 0.4,
        "$metalgrate" => 0.95,
        "$Metal_Box" => 0.5,
        "$metalvent" => 0.6,
        "$metalpanel" => 0.5,
        "$dirt" => 0.6,
        "$tile" => 0.7,
        "$Wood" => 0.9,
        "$Wood_Box" => 0.9,
        "$Wood_Basket" => 0.9,
        "$Wood_Crate" => 0.9,
        "$Wood_Plank" => 0.85,
        "$Wood_Solid" => 0.8,
        "$Wood_Dense" => 0.5,
        "$water" => 0.3,
        "$quicksand" => 0.2,
        "$Wood_Lader" => 0.9,
        "$glass" => 0.99,
        "$glassfloor" => 0.99,
        "$computer" => 0.4,
        "$concrete" => 0.5,
        "$asphalt" => 0.55,
        "$porcelain" => 0.95,
        "$brick" => 0.47,
        "$chainlink" => 0.99,
        "$flesh" => 0.9,
        "$armorflesh" => 0.5,
        "$ice" => 0.75,
        "$carpet" => 0.75,
        "$upholstery" => 0.75,
        "$plaster" => 0.7,
        "$sheetrock" => 0.85,
        "$cardboard" => 3.0,
        "$plastic_barrel" => 0.7,
        "$Plastic_Box" => 0.75,
        "$sand" => 0.3,
        "$rubber" => 0.85,
        "$glassbottle" => 0.99,
        "$pottery" => 0.95,
        "$clay" => 0.95,
        "$metal_barrel" => 0.01,
        "$foliage" => 0.95,
        "$watermelon" => 0.95,
        "$gravel" => 0.4,
        "$snow" => 0.85,
        "$metalvehicle" => 0.5,
        "$metal_sand_barrel" => 0.01,
        "$blockbullets" => 0.01,
        "$potterylarge" => 0.95,
        "$fruit" => 0.9,
        _ => 0.5,
    }
}

impl CS2 {
    /// Simulate bullet penetration and return estimated damage dealt.
    /// Returns 0.0 if the bullet cannot reach the target.
    /// If `auto_wall` is false, returns 0.0 when there are any obstacles.
    pub fn handle_bullet_penetration(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_t: f32,
        target: &Player,
        bone_index: u64,
        auto_wall: bool,
    ) -> f32 {
        let Some(bvh) = &self.bvh else {
            return 0.0;
        };

        let Some(local_player) = Player::local_player(self) else {
            return 0.0;
        };

        let mut hits = bvh.raycast(origin, direction, max_t);
        hits.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let weapon_vdata = local_player.weapon_vdata(self);
        let mut damage = weapon_vdata.damage as f32;

        if hits.is_empty() {
            damage *= weapon_vdata.range_modifier.powf(max_t / 500.0);
        } else if auto_wall {
            let is_front =
                |i: usize| (hits[i].1.v1 - hits[i].1.v0).cross(hits[i].1.v2 - hits[i].1.v0).dot(direction) < 0.0;
            let mut hit = 0;
            let mut prev_back_dist = 0.0;
            while hit < hits.len() {
                let front = hit;

                let air_time = hits[front].0 - prev_back_dist;
                damage *= weapon_vdata.range_modifier.powf(air_time / 500.0);

                while hit < hits.len() && is_front(hit) {
                    hit += 1;
                }
                while hit < hits.len() && !is_front(hit) {
                    hit += 1;
                }
                let back = hit - 1;

                let front_material = hits[front].1.material.as_ref();
                let back_material = hits[back].1.material.as_ref();
                let distance_modifier = if (front_material == "$Wood" || front_material == "$Wood_Panel")
                    && (back_material == "$Wood" || back_material == "$Wood_Panel")
                {
                    3.0
                } else {
                    let front_modifier = surfaceproperties_game(front_material);
                    let back_modifier = surfaceproperties_game(back_material);
                    (2.0 * front_modifier * back_modifier) / (front_modifier + back_modifier)
                };

                let thickness = hits[back].0 - hits[front].0;
                let penetration_modifier = 1.0 / distance_modifier;
                let weapon_penetration_modifier =
                    (3.75 / weapon_vdata.penetration * penetration_modifier * 3.0)
                        + (damage * 0.16);
                let damage_lost =
                    thickness * thickness * penetration_modifier / 24.0 + weapon_penetration_modifier;
                damage -= damage_lost;
                prev_back_dist = hits[back].0;
            }
        } else {
            return 0.0;
        }

        damage = damage.max(0.0);
        damage *= match bone_index {
            0 | 1 | 2 => 1.25,
            3 | 4 | 5 => 1.0,
            6 => weapon_vdata.headshot_multiplier,
            8 | 9 | 10 => 1.0,
            13 | 14 | 15 => 1.0,
            22 => 1.25,
            23 | 24 => 0.75,
            25 => 1.25,
            26 | 27 => 0.75,
            _ => 1.0,
        };

        let armor = target.armor(self) as f32;
        let armored = match bone_index {
            6 => target.has_helmet(self),
            23 | 24 | 26 | 27 => false,
            _ => armor > 0.0,
        };
        if armored {
            let ratio = weapon_vdata.armor_ratio * 0.5;
            let mut hp_dmg = damage * ratio;
            let armor_dmg = (damage - hp_dmg) * 0.5;

            if armor_dmg > armor {
                hp_dmg = damage - armor * 2.0;
            }

            damage = hp_dmg;
        }

        damage.round()
    }
}
