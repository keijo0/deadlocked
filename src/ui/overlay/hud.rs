use egui::{Align2, Color32, Painter, Shape, Stroke, pos2};

use crate::{
    cs2::entity::weapon_class::WeaponClass, data::Data, math::world_to_screen, ui::app::App,
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

    pub fn draw_keybind_list(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.keybind_list {
            return;
        }

        let position = pos2(10.0, data.window_size.y / 2.0);
        let font_size = self.config.hud.font_size;
        let mut line: f32 = 0.0;

        if self.config.hud.keybind_aimbot {
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
                format!(
                    "Aimbot [{}]: {}",
                    hotkeys_str,
                    if data.aimbot_active { "ON" } else { "OFF" }
                ),
                position + egui::vec2(0.0, font_size * line),
                Align2::LEFT_TOP,
                None,
            );
            line += 1.0;
        }

        if self.config.hud.keybind_fov {
            let ab = self.aimbot_config(&data.weapon);
            self.text(
                painter,
                format!("FOV: {:.1}  Smooth: {:.1}", ab.fov, ab.smooth),
                position + egui::vec2(0.0, font_size * line),
                Align2::LEFT_TOP,
                None,
            );
            line += 1.0;
        }

        if self.config.hud.keybind_trigger_delay {
            let tb = self.triggerbot_config(&data.weapon);
            self.text(
                painter,
                format!("Ms: {}-{}", tb.delay.start(), tb.delay.end()),
                position + egui::vec2(0.0, font_size * line),
                Align2::LEFT_TOP,
                None,
            );
            line += 1.0;
        }

        if self.config.hud.keybind_triggerbot {
            self.text(
                painter,
                format!(
                    "Triggerbot [{:?}]: {}",
                    self.config.aim.triggerbot_hotkey,
                    if data.triggerbot_active { "ON" } else { "OFF" }
                ),
                position + egui::vec2(0.0, font_size * line),
                Align2::LEFT_TOP,
                None,
            );
            line += 1.0;
        }

        if self.config.hud.keybind_esp {
            self.text(
                painter,
                format!(
                    "ESP [{:?}]: {}",
                    self.config.player.esp_hotkey,
                    if data.esp_active { "ON" } else { "OFF" }
                ),
                position + egui::vec2(0.0, font_size * line),
                Align2::LEFT_TOP,
                None,
            );
            line += 1.0;
        }

        if self.config.hud.keybind_bunnyhop {
            self.text(
                painter,
                format!(
                    "Bunnyhop [{:?}]: {}",
                    self.config.misc.bunnyhop_hotkey,
                    if self.config.misc.bunnyhop { "ON" } else { "OFF" }
                ),
                position + egui::vec2(0.0, font_size * line),
                Align2::LEFT_TOP,
                None,
            );
            line += 1.0;
        }

        if self.config.hud.keybind_server_picker && !self.server_regions.is_empty() {
            let enabled: Vec<&str> = self
                .server_regions
                .iter()
                .filter(|r| !r.blocked)
                .map(|r| r.name.as_str())
                .collect();

            if enabled.len() == self.server_regions.len() {
                self.text(
                    painter,
                    "QD: all",
                    position + egui::vec2(0.0, font_size * line),
                    Align2::LEFT_TOP,
                    None,
                );
                line += 1.0;
            } else if !enabled.is_empty() {
                self.text(
                    painter,
                    "QD:",
                    position + egui::vec2(0.0, font_size * line),
                    Align2::LEFT_TOP,
                    None,
                );
                line += 1.0;
                for name in &enabled {
                    self.text(
                        painter,
                        name.to_uppercase(),
                        position + egui::vec2(8.0, font_size * line),
                        Align2::LEFT_TOP,
                        None,
                    );
                    line += 1.0;
                }
            }
        }

        if self.config.hud.keybind_backtrack && self.config.aim.global.aimbot.backtrack {
            let bt_ms = self.config.aim.global.aimbot.backtrack_ms;
            let ping = data.ping.max(0) as u32;
            let label = if ping > 0 {
                format!("BT: +{}ms (~{}ms w/ ping)", bt_ms, bt_ms + ping)
            } else {
                format!("BT: +{}ms", bt_ms)
            };
            self.text(
                painter,
                label,
                position + egui::vec2(0.0, font_size * line),
                Align2::LEFT_TOP,
                None,
            );
            line += 1.0;
        }

        if self.config.hud.keybind_ping {
            let label = if data.ping >= 0 {
                format!("Ping: {}ms", data.ping)
            } else {
                "Ping: N/A".to_string()
            };
            self.text(
                painter,
                label,
                position + egui::vec2(0.0, font_size * line),
                Align2::LEFT_TOP,
                None,
            );
            line += 1.0;
        }
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
