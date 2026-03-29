use egui::{Align2, Color32, Painter, Shape, Stroke, pos2};

use crate::{
    config::KeyMode, cs2::entity::weapon_class::WeaponClass, data::Data, math::world_to_screen,
    ui::app::App,
};

impl App {
    pub fn overlay_debug(&self, painter: &Painter, data: &Data) {
        if self.config.hud.debug {
            painter.line(
                vec![pos2(0.0, 0.0), pos2(data.window_size.x, data.window_size.y)],
                Stroke::new(self.config.hud.line_width, Color32::WHITE),
            );
            painter.line(
                vec![pos2(data.window_size.x, 0.0), pos2(0.0, data.window_size.y)],
                Stroke::new(self.config.hud.line_width, Color32::WHITE),
            );
        }
    }

    pub fn draw_spectator_list(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.spectator_list {
            return;
        }

        let text = if data.spectators.is_empty() {
            "Spectators:".to_string()
        } else {
            let names = data.spectators.join("\n  ");
            format!("Spectators:\n  {names}")
        };

        let padding = 10.0;
        let pos = pos2(padding, padding);

        self.text_sized(
            painter,
            text,
            pos,
            Align2::LEFT_TOP,
            Some(self.config.hud.spectator_list_color),
            self.config.hud.font_size,
        );
    }

    pub fn draw_bomb_timer(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.bomb_timer || !data.bomb.planted {
            return;
        }

        if let Some(pos) = world_to_screen(&data.bomb.position, data) {
            self.text(
                painter,
                format!("{:.3}", data.bomb.timer),
                pos,
                Align2::CENTER_CENTER,
                None,
            );
            if data.bomb.being_defused {
                self.text(
                    painter,
                    format!("defusing {:.3}", data.bomb.defuse_remain_time),
                    pos2(pos.x, pos.y + self.config.hud.font_size),
                    Align2::CENTER_CENTER,
                    None,
                );
            }
        }

        let fraction = (data.bomb.timer / 40.0).clamp(0.0, 1.0);
        let color = self.health_color((fraction * 100.0) as i32, 255);
        painter.line(
            vec![
                pos2(0.0, data.window_size.y),
                pos2(data.window_size.x * fraction, data.window_size.y),
            ],
            Stroke::new(self.config.hud.line_width * 2.0, color),
        );
    }

    pub fn draw_fov_circle(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.fov_circle || !data.in_game {
            return;
        }

        let weapon_config = self.aimbot_config(&data.weapon);

        if !weapon_config.enabled || (weapon_config.mode == KeyMode::Toggle && !data.aimbot_active)
        {
            return;
        }

        let aim_fov = weapon_config.fov;

        if weapon_config.distance_adjusted_fov {
            self.draw_distance_scaled_fov_circle(painter, data, aim_fov, 125.0, Color32::GREEN);
            self.draw_distance_scaled_fov_circle(painter, data, aim_fov, 250.0, Color32::YELLOW);
            self.draw_distance_scaled_fov_circle(painter, data, aim_fov, 500.0, Color32::RED);
        } else {
            self.draw_simple_fov_circle(painter, data, aim_fov, Color32::WHITE);
        }
    }

    pub fn draw_keybind_list(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.keybind_list {
            return;
        }

        let position = pos2(10.0, data.window_size.y / 2.0);
        let hotkeys_str = self
            .config
            .aim
            .aimbot_hotkeys
            .iter()
            .map(|k| format!("{:?}", k))
            .collect::<Vec<_>>()
            .join(", ");
        self.text(
            painter,
            format!("Aimbot: {}", hotkeys_str),
            position,
            Align2::LEFT_TOP,
            None,
        );
        self.text(
            painter,
            format!("RCS: {:?}", self.config.aim.triggerbot_hotkey),
            position + egui::vec2(0.0, self.config.hud.font_size),
            Align2::LEFT_TOP,
            None,
        );
    }

    fn get_current_fov(&self) -> f32 {
        crate::constants::cs2::DEFAULT_FOV as f32
    }

    fn calculate_fov_radius(&self, data: &Data, target_fov: f32) -> f32 {
        let current_fov = self.get_current_fov();
        let screen_width = data.window_size.x;

        let current_fov_tan = (current_fov.to_radians() / 2.0).tan();
        if current_fov_tan == 0.0 {
            return 0.0;
        }

        let target_fov_tan = (target_fov.to_radians() / 2.0).tan();
        (target_fov_tan / current_fov_tan) * (screen_width / 2.0)
    }

    fn draw_fov_circle_impl(&self, painter: &Painter, data: &Data, radius: f32, color: Color32) {
        let center = pos2(data.window_size.x / 2.0, data.window_size.y / 2.0);
        let stroke = Stroke::new(self.config.hud.line_width, color);
        painter.circle_stroke(center, radius, stroke);
    }

    fn get_distance_fov_scale(&self, distance: f32) -> f32 {
        (5.0 - (distance / 125.0)).max(1.0)
    }

    fn draw_simple_fov_circle(
        &self,
        painter: &Painter,
        data: &Data,
        target_fov: f32,
        color: Color32,
    ) {
        let radius = self.calculate_fov_radius(data, target_fov);
        self.draw_fov_circle_impl(painter, data, radius, color);
    }

    fn draw_distance_scaled_fov_circle(
        &self,
        painter: &Painter,
        data: &Data,
        base_aim_fov: f32,
        distance: f32,
        color: Color32,
    ) {
        let scale = self.get_distance_fov_scale(distance);
        let target_fov = base_aim_fov * scale;

        let radius = self.calculate_fov_radius(data, target_fov);
        self.draw_fov_circle_impl(painter, data, radius, color);
    }

    pub fn draw_sniper_crosshair(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.sniper_crosshair
            || WeaponClass::from_string(data.weapon.as_ref()) != WeaponClass::Sniper
        {
            return;
        }

        painter.line(
            vec![
                pos2(data.window_size.x / 2.0, data.window_size.y / 2.0 - 8.0),
                pos2(data.window_size.x / 2.0, data.window_size.y / 2.0 + 8.0),
            ],
            Stroke::new(self.config.hud.line_width, self.config.hud.crosshair_color),
        );
        painter.line(
            vec![
                pos2(data.window_size.x / 2.0 - 10.0, data.window_size.y / 2.0),
                pos2(data.window_size.x / 2.0 + 10.0, data.window_size.y / 2.0),
            ],
            Stroke::new(self.config.hud.line_width, self.config.hud.crosshair_color),
        );
    }

    pub fn draw_fov_arrows(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.fov_arrows
            || self.config.hud.fov_arrow_size <= 0.0
            || !data.in_game
            || !data.esp_active
            || !self.config.player.enabled
        {
            return;
        }

        let center_x = data.window_size.x / 2.0;
        let center_y = data.window_size.y / 2.0;
        // Distance from screen edge where arrow centers are placed
        const MARGIN: f32 = 50.0;
        let arrow_size = self.config.hud.fov_arrow_size;

        let half_w = data.window_size.x / 2.0 - MARGIN;
        let half_h = data.window_size.y / 2.0 - MARGIN;

        let vm = &data.view_matrix;

        let arrow_color = self.config.hud.fov_arrow_color;

        for player in &data.players {
            if self.config.player.visible_only && !player.visible {
                continue;
            }

            // Only draw arrows for players that are off-screen
            if world_to_screen(&player.position, data).is_some() {
                continue;
            }

            let pos = player.position;
            let clip_x = vm.x_axis.x * pos.x
                + vm.x_axis.y * pos.y
                + vm.x_axis.z * pos.z
                + vm.x_axis.w;
            let clip_y = vm.y_axis.x * pos.x
                + vm.y_axis.y * pos.y
                + vm.y_axis.z * pos.z
                + vm.y_axis.w;
            let w = vm.w_axis.x * pos.x
                + vm.w_axis.y * pos.y
                + vm.w_axis.z * pos.z
                + vm.w_axis.w;

            if w.abs() < 0.0001 {
                continue;
            }

            // Compute screen-space direction from center toward the enemy.
            // Screen x maps from clip_x/w; screen y is inverted (clip_y/w → downward).
            // When w < 0 (enemy behind camera) negate to flip to the correct side.
            let sign = if w > 0.0 { 1.0_f32 } else { -1.0_f32 };
            let dir_x = sign * (clip_x / w);
            let dir_y = sign * -(clip_y / w);

            let len = (dir_x * dir_x + dir_y * dir_y).sqrt();
            if len < 0.001 {
                continue;
            }
            let dir_x = dir_x / len;
            let dir_y = dir_y / len;

            // Intersect the direction ray with the MARGIN-inset rectangle.
            // t is always positive: half_dim / |dir_component|.
            let edge_t = |dir_component: f32, half_dim: f32| -> f32 {
                if dir_component.abs() > 0.001 {
                    half_dim / dir_component.abs()
                } else {
                    f32::MAX
                }
            };

            let t = edge_t(dir_x, half_w).min(edge_t(dir_y, half_h));
            let arrow_x = center_x + dir_x * t;
            let arrow_y = center_y + dir_y * t;

            // Perpendicular direction for the arrow wings
            let perp_x = -dir_y;
            let perp_y = dir_x;

            let tip = pos2(
                arrow_x + dir_x * arrow_size,
                arrow_y + dir_y * arrow_size,
            );
            let base_left = pos2(
                arrow_x - dir_x * arrow_size * 0.5 + perp_x * arrow_size * 0.7,
                arrow_y - dir_y * arrow_size * 0.5 + perp_y * arrow_size * 0.7,
            );
            let base_right = pos2(
                arrow_x - dir_x * arrow_size * 0.5 - perp_x * arrow_size * 0.7,
                arrow_y - dir_y * arrow_size * 0.5 - perp_y * arrow_size * 0.7,
            );

            painter.add(Shape::convex_polygon(
                vec![tip, base_left, base_right],
                arrow_color,
                Stroke::NONE,
            ));
        }
    }
}
