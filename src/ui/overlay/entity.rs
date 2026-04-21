use std::time::Instant;

use egui::{Align2, Color32, Painter, Pos2, Stroke, pos2};
use glam::Vec3;

use crate::{
    cs2::entity::{
        EntityInfo, GrenadeInfo, inferno::InfernoInfo, molotov::MolotovInfo,
        smoke::SmokeInfo, weapon::Weapon,
    },
    data::Data,
    math::world_to_screen,
    ui::{app::App, overlay::convex_hull, trail::Trail},
};

impl App {
    pub fn draw_entity(&self, painter: &Painter, entity: &EntityInfo, data: &Data) {
        match entity {
            EntityInfo::Weapon { weapon, position } => {
                if !self.config.hud.dropped_weapons {
                    return;
                }
                let Some(pos) = world_to_screen(position, data) else {
                    return;
                };
                let font_size = self.config.hud.font_size;
                // Scale box with distance: at 800 units scale=1.0, halves at 1600, doubles at 400.
                let distance = (data.local_player.position - *position).length();
                let dist_scale = (800.0_f32 / distance.max(50.0)).clamp(0.1, 5.0);
                use crate::cs2::entity::weapon_class::WeaponClass;
                let (base_half_w, base_half_h) = match weapon.weapon_class() {
                    WeaponClass::Sniper => (font_size * 3.5, font_size * 0.9),
                    WeaponClass::Rifle | WeaponClass::Heavy => (font_size * 3.0, font_size * 0.8),
                    WeaponClass::Smg | WeaponClass::Shotgun => (font_size * 2.2, font_size * 0.7),
                    _ => (font_size * 1.2, font_size * 0.6),
                };
                let half_w = base_half_w * dist_scale;
                let half_h = base_half_h * dist_scale;
                let tl = pos2(pos.x - half_w, pos.y - half_h);
                let br = pos2(pos.x + half_w, pos.y + half_h);
                if self.config.hud.weapon_box
                    && (self.config.hud.weapon_box_max_distance <= 0.0
                        || distance <= self.config.hud.weapon_box_max_distance)
                    && (self.config.hud.weapon_esp_max_distance <= 0.0
                        || distance <= self.config.hud.weapon_esp_max_distance)
                {
                    let color = self.config.hud.text_color;
                    let lw = self.config.hud.line_width;
                    let stroke = Stroke::new(lw, color);
                    let outline_stroke = if self.config.hud.text_outline {
                        Some(Stroke::new(
                            lw + 2.0,
                            Color32::from_rgba_unmultiplied(0, 0, 0, color.a()),
                        ))
                    } else {
                        None
                    };

                    use crate::config::WeaponBoxMode;
                    match self.config.hud.weapon_box_mode {
                        WeaponBoxMode::Full => {
                            let rect = egui::Rect::from_min_max(tl, br);
                            if let Some(os) = outline_stroke {
                                painter.rect(rect, 0, Color32::TRANSPARENT, os, egui::StrokeKind::Middle);
                            }
                            painter.rect(rect, 0, Color32::TRANSPARENT, stroke, egui::StrokeKind::Middle);
                        }
                        WeaponBoxMode::Gap => {
                            let tr = pos2(br.x, tl.y);
                            let bl = pos2(tl.x, br.y);
                            let corner = (half_w / 2.0).max(3.0);
                            let corners: [Vec<egui::Pos2>; 4] = [
                                vec![pos2(tl.x + corner, tl.y), tl, pos2(tl.x, tl.y + corner)],
                                vec![pos2(tr.x - corner, tr.y), tr, pos2(tr.x, tr.y + corner)],
                                vec![pos2(bl.x + corner, bl.y), bl, pos2(bl.x, bl.y - corner)],
                                vec![pos2(br.x - corner, br.y), br, pos2(br.x, br.y - corner)],
                            ];
                            if let Some(os) = outline_stroke {
                                for c in &corners {
                                    painter.line(c.clone(), os);
                                }
                            }
                            for c in corners {
                                painter.line(c, stroke);
                            }
                        }
                    }
                }
                self.draw_weapon_esp(painter, weapon, pos, tl.y, position, data, dist_scale);
            }
            EntityInfo::Inferno(inferno) => self.inferno(painter, data, inferno),
            EntityInfo::Smoke(smoke) => self.smoke(painter, data, smoke),
            EntityInfo::Molotov(molotov) => self.molotov(painter, data, molotov),
            EntityInfo::Flashbang(info) => {
                self.draw_grenade(painter, data, info, Color32::WHITE)
            }
            EntityInfo::HeGrenade(info) => {
                self.draw_grenade(painter, data, info, Color32::DARK_GRAY)
            }
            EntityInfo::Decoy(info) => {
                self.draw_grenade(painter, data, info, Color32::PURPLE)
            }
        };
    }

    fn draw_weapon_esp(
        &self,
        painter: &Painter,
        weapon: &Weapon,
        screen_pos: Pos2,
        box_top_y: f32,
        world_pos: &Vec3,
        data: &Data,
        dist_scale: f32,
    ) {
        // Skip rendering if weapon is beyond the configured max distance.
        let distance = (data.local_player.position - *world_pos).length();
        if self.config.hud.weapon_esp_max_distance > 0.0
            && distance > self.config.hud.weapon_esp_max_distance
        {
            return;
        }

        // Respect the same max-distance gate as the weapon box
        if self.config.hud.weapon_box_max_distance > 0.0
            && distance > self.config.hud.weapon_box_max_distance
        {
            return;
        }

        // Distance-based font scaling
        let scale = if distance < 500.0 {
            1.2
        } else if distance < 1500.0 {
            1.0
        } else {
            0.8
        };
        let font_size = self.config.hud.font_size * scale;

        // Weapon class color coding (or fall back to config text color)
        let color = if self.config.hud.weapon_esp_use_colors {
            weapon.esp_color()
        } else {
            self.config.hud.text_color
        };

        let text = format!("{weapon}");

        // Shrink the gap between the label and the box at long distances so
        // labels don't float far above tiny boxes.
        let label_offset = 2.0 * dist_scale.min(1.0);
        let text_pos = pos2(screen_pos.x, box_top_y - label_offset);

        // Measure the text to draw a background box
        let galley = painter.layout_no_wrap(text.clone(), egui::FontId::proportional(font_size), color);
        let text_size = galley.rect.size();

        // With Align2::CENTER_BOTTOM the text's bottom is at text_pos;
        // build the rect directly from that anchor point.
        let text_rect = egui::Rect::from_min_max(
            pos2(text_pos.x - text_size.x / 2.0, text_pos.y - text_size.y),
            pos2(text_pos.x + text_size.x / 2.0, text_pos.y),
        );
        let bg_rect = text_rect.expand(4.0);

        // Semi-transparent dark background for contrast
        painter.rect_filled(
            bg_rect,
            2.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, self.config.hud.weapon_esp_background_opacity),
        );

        // Draw the weapon name with outline support
        self.text_sized(painter, text, text_pos, Align2::CENTER_BOTTOM, Some(color), font_size);
    }

    fn draw_grenade(
        &self,
        painter: &Painter,
        data: &Data,
        info: &GrenadeInfo,
        trail_color: Color32,
    ) {
        if !self.config.hud.grenade_trails {
            return;
        }
        let Some(position) = world_to_screen(&info.position, data) else {
            return;
        };
        self.text(painter, info.name, position, Align2::CENTER_CENTER, None);

        if !self.config.hud.grenade_trails {
            return;
        }

        let stroke = Stroke::new(self.config.hud.line_width, trail_color);
        let Some(trail) = self.trails.get(&info.entity) else {
            return;
        };
        for window in trail.trail.windows(2) {
            if let [v1, v2] = window {
                use crate::math::world_to_screen;

                let Some(v1) = world_to_screen(v1, data) else {
                    continue;
                };
                let Some(v2) = world_to_screen(v2, data) else {
                    continue;
                };
                painter.line_segment([v1, v2], stroke);
            }
        }
    }

    fn inferno(&self, painter: &Painter, data: &Data, inferno: &InfernoInfo) {
        use egui::Shape;

        if !self.config.hud.grenade_trails {
            return;
        }
        let hull: Vec<Pos2> = convex_hull(&inferno.hull)
            .iter()
            .filter_map(|p| {
                use crate::math::world_to_screen;

                let p = p + (p - inferno.position).clamp_length(60.0, 60.0);
                world_to_screen(&p, data)
            })
            .collect();
        if hull.len() < 3 {
            return;
        }

        let shape = Shape::convex_polygon(
            hull,
            Color32::from_rgba_unmultiplied(255, 0, 0, 127),
            Stroke::NONE,
        );
        painter.add(shape);

        self.draw_grenade(painter, data, &inferno.grenade(), Color32::TRANSPARENT);
    }

    fn smoke(&self, painter: &Painter, data: &Data, smoke: &SmokeInfo) {
        if !self.config.hud.grenade_trails {
            return;
        }
        self.draw_grenade(
            painter,
            data,
            &smoke.grenade(),
            Color32::LIGHT_GRAY,
        );
    }

    fn molotov(&self, painter: &Painter, data: &Data, molotov: &MolotovInfo) {
        if !self.config.hud.grenade_trails {
            return;
        }
        if molotov.is_incendiary {
            self.draw_grenade(
                painter,
                data,
                &molotov.grenade(),
                Color32::ORANGE,
            );
        } else {
            self.draw_grenade(
                painter,
                data,
                &molotov.grenade(),
                Color32::RED,
            );
        }
    }

    pub fn update_trails(&mut self) {
        let data = self.data.lock();
        for entity in &data.entities {
            let (entity, position) = match entity {
                EntityInfo::Inferno(info) => (info.entity, info.position),
                EntityInfo::Smoke(info) => (info.entity, info.position),
                EntityInfo::Molotov(info) => (info.entity, info.position),
                EntityInfo::Flashbang(info) | EntityInfo::HeGrenade(info) => {
                    (info.entity, info.position)
                }
                _ => continue,
            };
            if let Some(trail) = self.trails.get_mut(&entity) {
                if (position - trail.trail.last().unwrap()).length() < 1.0 {
                    continue;
                }
                trail.trail.push(position);
                trail.last_update = Instant::now();
            } else {
                self.trails.insert(
                    entity,
                    Trail {
                        trail: vec![position],
                        last_update: Instant::now(),
                    },
                );
            }
        }

        let now = Instant::now();
        self.trails
            .retain(|_k, trail| now.duration_since(trail.last_update) < Trail::MAX_AGE);
    }
}
