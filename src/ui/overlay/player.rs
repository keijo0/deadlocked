use egui::{Align2, Color32, FontId, Painter, Stroke, pos2};
use glam::vec3;

use crate::{
    config::{BoxMode, DrawMode},
    cs2::bones::Bones,
    data::{Data, PlayerData},
    math::world_to_screen,
    ui::app::App,
};

impl App {
    pub fn draw_player(&self, painter: &Painter, player: &PlayerData, data: &Data) {
        if self.config.player.visible_only && !player.visible {
            return;
        }

        self.player_box(painter, player, data, None);
        self.skeleton(painter, player, data, None);
    }

    pub fn draw_backtrack(&self, painter: &Painter, player: &PlayerData, data: &Data) {
        if !self.config.player.backtrack_visual {
            return;
        }
        if self.config.player.backtrack_in_esp && !data.esp_active {
            return;
        }
        let Some(history) = data.backtrack_history.get(&player.pawn) else {
            return;
        };
        let Some(record) = history.back() else {
            return;
        };
        let color = self.config.player.backtrack_color;
        let stroke = Stroke::new(self.config.hud.line_width, color);
        for (a, b) in &Bones::CONNECTIONS {
            let Some(&pa) = record.bones.get(a) else { continue };
            let Some(&pb) = record.bones.get(b) else { continue };
            let Some(sa) = world_to_screen(&pa, data) else { continue };
            let Some(sb) = world_to_screen(&pb, data) else { continue };
            painter.line(vec![sa, sb], stroke);
        }
    }

    fn alpha(color: Color32, alpha: f32) -> Color32 {
        Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            (alpha.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    fn player_box(&self, painter: &Painter, player: &PlayerData, data: &Data, alpha: Option<f32>) {
        use crate::config::DrawMode;

        let alpha = match alpha {
            Some(alpha) => alpha.clamp(0.0, 1.0),
            None => 1.0,
        };

        let health_color =
            self.health_color(player.health, self.config.player.box_visible_color.a());
        let mut color = match &self.config.player.draw_box {
            DrawMode::None => health_color,
            DrawMode::Health => health_color,
            DrawMode::Color => {
                if player.visible {
                    self.config.player.box_visible_color
                } else {
                    self.config.player.box_invisible_color
                }
            }
        };

        color = Self::alpha(color, alpha);

        let stroke = Stroke::new(self.config.hud.line_width, color);
        let icon_font = FontId::monospace(self.config.hud.icon_size);

        let midpoint = (player.position + player.head) / 2.0;
        let height = player.head.z - player.position.z + 24.0;
        let half_height = height / 2.0;
        let top = midpoint + vec3(0.0, 0.0, half_height);
        let bottom = midpoint - vec3(0.0, 0.0, half_height);

        let Some(top) = world_to_screen(&top, data) else {
            return;
        };
        let Some(bottom) = world_to_screen(&bottom, data) else {
            return;
        };
        let half_height = bottom.y - top.y;
        let width = half_height / 2.0;
        let half_width = width / 2.0;
        // corner corner_size length: equal in both axes, proportional to box width
        let corner_size = (half_width / 2.0).max(4.0);

        // Use the midpoint x so the box stays rectangular even at uneven angles.
        let center_x = (top.x + bottom.x) / 2.0;
        let tl = pos2(center_x - half_width, top.y);
        let tr = pos2(center_x + half_width, top.y);
        let bl = pos2(center_x - half_width, bottom.y);
        let br = pos2(center_x + half_width, bottom.y);

        if self.config.player.draw_box != DrawMode::None {
            let outline_stroke = if self.config.hud.text_outline {
                Some(Stroke::new(
                    self.config.hud.line_width + 2.0,
                    Color32::from_rgba_unmultiplied(0, 0, 0, color.a()),
                ))
            } else {
                None
            };

            if self.config.player.box_mode == BoxMode::Gap {
                let corners: [Vec<egui::Pos2>; 4] = [
                    vec![pos2(tl.x + corner_size, tl.y), tl, pos2(tl.x, tl.y + corner_size)],
                    vec![pos2(tr.x - corner_size, tr.y), tr, pos2(tr.x, tr.y + corner_size)],
                    vec![pos2(bl.x + corner_size, bl.y), bl, pos2(bl.x, bl.y - corner_size)],
                    vec![pos2(br.x - corner_size, br.y), br, pos2(br.x, br.y - corner_size)],
                ];
                if let Some(os) = outline_stroke {
                    for corner in &corners {
                        painter.line(corner.clone(), os);
                    }
                }
                for corner in corners {
                    painter.line(corner, stroke);
                }
            } else {
                let rect = egui::Rect::from_min_max(tl, br);
                if let Some(os) = outline_stroke {
                    painter.rect(rect, 0, Color32::TRANSPARENT, os, egui::StrokeKind::Middle);
                }
                painter.rect(rect, 0, Color32::TRANSPARENT, stroke, egui::StrokeKind::Middle);
            }
        }

        // health bar / health text
        let health_x = bl.x - self.config.hud.line_width * 2.0;
        let box_height = bl.y - tl.y;
        let bar_top_y = bl.y - (box_height * player.health as f32 / 100.0);

        if self.config.player.health_bar {
            if self.config.hud.text_outline {
                painter.line(
                    vec![
                        pos2(health_x, bl.y),
                        pos2(health_x, bar_top_y),
                    ],
                    Stroke::new(
                        self.config.hud.line_width + 2.0,
                        Color32::from_rgba_unmultiplied(0, 0, 0, Self::alpha(health_color, alpha).a()),
                    ),
                );
            }
            painter.line(
                vec![
                    pos2(health_x, bl.y),
                    pos2(health_x, bar_top_y),
                ],
                Stroke::new(self.config.hud.line_width, Self::alpha(health_color, alpha)),
            );
        }

        let mut offset = 0.0;
        let font_size = self.config.hud.font_size;
        let text_color = Self::alpha(self.config.hud.text_color, alpha);
        if self.config.player.player_name {
            self.text(
                painter,
                &player.name,
                //pos2(tr.x + corner_size, tr.y),
                pos2(center_x, tr.y),
                Align2::CENTER_BOTTOM,
                Some(text_color),
            );
            offset += font_size;
        }

        if self.config.player.health_text {
            self.text(
                painter,
                player.health.to_string(),
                //pos2(tr.x + corner_size, tr.y),
                pos2(tr.x + corner_size, tr.y),
                Align2::LEFT_TOP,
                Some(Self::alpha(health_color, alpha)),
            );
            offset += font_size;
        }

        if self.config.player.armor_text && player.armor > 0 {
            self.text(
                painter,
                player.armor.to_string(),
                pos2(tr.x + corner_size, tr.y + offset),
                Align2::LEFT_BOTTOM,
                Some(Self::alpha(Color32::from_rgb(100, 149, 237), alpha)),
            );
        }

        let mut weapon_y = bl.y;
        if self.config.player.weapon_icon {
            self.text(
                painter,
                player.weapon.to_string(),
                pos2(bl.x + half_width, weapon_y),
                Align2::CENTER_TOP,
                Some(text_color),
            );
            weapon_y += font_size;
        }
        if self.config.player.tags {
            self.text_with_font(
                painter,
                player.weapon.to_icon(),
                pos2(bl.x + half_width, weapon_y),
                Align2::CENTER_TOP,
                Some(text_color),
                icon_font,
            );
        }
    }

    fn skeleton(&self, painter: &Painter, player: &PlayerData, data: &Data, alpha: Option<f32>) {
        let mut color = match &self.config.player.draw_skeleton {
            DrawMode::None => return,
            DrawMode::Health => {
                self.health_color(player.health, self.config.player.skeleton_color.a())
            }
            DrawMode::Color => self.config.player.skeleton_color,
        };
        if let Some(alpha) = alpha {
            color = Self::alpha(color, alpha);
        }
        let stroke = Stroke::new(self.config.hud.line_width, color);

        for (a, b) in &Bones::CONNECTIONS {
            let Some(a) = player.bones.get(a) else {
                continue;
            };
            let Some(b) = player.bones.get(b) else {
                continue;
            };

            let Some(a) = world_to_screen(a, data) else {
                continue;
            };
            let Some(b) = world_to_screen(b, data) else {
                continue;
            };

            painter.line(vec![a, b], stroke);
        }
    }
}
