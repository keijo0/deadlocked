use egui::{Align2, Color32, FontId, Painter, Shape, Stroke, epaint, pos2};

use crate::{
    config::{KeybindListStyle, MediaInfoStyle, SpectatorListStyle},
    cs2::entity::weapon_class::WeaponClass,
    data::{Data, PenetrationCrosshairState},
    math::world_to_screen,
    ui::{app::App, color::{AccentStyle, ColorScheme, Colors}},
};

/// Draw a 2px gradient accent line across the top of a panel.
/// Colors: left_color → mid_color (left half), mid_color → right_color (right half).
fn draw_gradient_accent_line(
    painter: &Painter,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    left: Color32,
    mid: Color32,
    right: Color32,
) {
    let half = width / 2.0;
    // Left half: left → mid
    let mut mesh = epaint::Mesh::default();
    mesh.colored_vertex(pos2(x, y), left);
    mesh.colored_vertex(pos2(x + half, y), mid);
    mesh.colored_vertex(pos2(x + half, y + height), mid);
    mesh.colored_vertex(pos2(x, y + height), left);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));

    // Right half: mid → right
    let mut mesh2 = epaint::Mesh::default();
    mesh2.colored_vertex(pos2(x + half, y), mid);
    mesh2.colored_vertex(pos2(x + width, y), right);
    mesh2.colored_vertex(pos2(x + width, y + height), right);
    mesh2.colored_vertex(pos2(x + half, y + height), mid);
    mesh2.add_triangle(0, 1, 2);
    mesh2.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh2));
}

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
        match self.config.hud.spectator_list_style {
            SpectatorListStyle::Simple => self.draw_spectator_list_simple(painter, data),
            SpectatorListStyle::New => self.draw_spectator_list_new(painter, data),
            SpectatorListStyle::Interium => self.draw_spectator_list_interium(painter, data),
        }
    }

    fn draw_spectator_list_simple(&self, painter: &Painter, data: &Data) {
        let scale = self.config.hud.spectator_list_scale;
        let font_size = self.config.hud.font_size * scale;
        let color = self.config.hud.spectator_list_color;
        let local_spectators: Vec<&str> = data
            .spectators
            .iter()
            .filter(|e| e.watching_local)
            .map(|e| e.name.as_str())
            .collect();
        let text = if local_spectators.is_empty() {
            "Spectators:".to_string()
        } else {
            let names = local_spectators.join("\n  ");
            format!("Spectators:\n  {names}")
        };
        let x = if self.config.hud.spectator_list_x >= 0.0 {
            self.config.hud.spectator_list_x
        } else {
            10.0
        };
        let y = if self.config.hud.spectator_list_y >= 0.0 {
            self.config.hud.spectator_list_y
        } else {
            10.0
        };
        let pos = pos2(x, y);
        if self.config.hud.spectator_list_simple_backdrop {
            let galley = painter.layout(
                text.clone(),
                FontId::proportional(font_size),
                color,
                f32::INFINITY,
            );
            let padding = 4.0;
            painter.rect_filled(
                egui::Rect::from_min_size(
                    pos2(pos.x - padding, pos.y - padding),
                    egui::vec2(galley.size().x + padding * 2.0, galley.size().y + padding * 2.0),
                ),
                egui::CornerRadius::same(2),
                Color32::from_rgba_unmultiplied(0, 0, 0, 140),
            );
        }
        self.text_with_font(
            painter,
            text,
            pos,
            Align2::LEFT_TOP,
            Some(color),
            FontId::proportional(font_size),
        );
    }

    fn draw_spectator_list_new(&self, painter: &Painter, data: &Data) {
        let scheme = ColorScheme::for_style(&self.config.accent_style);

        let scale = self.config.hud.spectator_list_scale;
        let font_size = self.config.hud.font_size * scale;
        let panel_width = font_size * 12.0;
        let header_height = font_size + 8.0 * scale;
        let row_height = font_size + 4.0 * scale;
        let padding = 8.0 * scale;

        let row_count = data.spectators.len();
        let total_height = header_height + row_height * row_count as f32;

        let screen_w = data.window_size.x;
        let screen_h = data.window_size.y;

        let panel_x = if self.config.hud.spectator_list_x >= 0.0 {
            self.config.hud.spectator_list_x
        } else {
            screen_w - panel_width - 10.0
        };
        let panel_y = if self.config.hud.spectator_list_y >= 0.0 {
            self.config.hud.spectator_list_y
        } else {
            (screen_h - total_height) / 2.0
        };

        // Header bar — uses the scheme's highlight color with slight transparency
        let header_rect = egui::Rect::from_min_size(
            pos2(panel_x, panel_y),
            egui::vec2(panel_width, header_height),
        );
        let header_radius = if data.spectators.is_empty() {
            egui::CornerRadius { nw: 2, ne: 2, sw: 2, se: 2 }
        } else {
            egui::CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 }
        };
        let [hr, hg, hb, _] = scheme.highlight.to_srgba_unmultiplied();
        painter.rect(
            header_rect,
            header_radius,
            Color32::from_rgba_unmultiplied(hr, hg, hb, 220),
            Stroke::new(1.0, scheme.subtext),
            egui::StrokeKind::Middle,
        );
        // Gradient accent line at top of header
        draw_gradient_accent_line(
            painter,
            panel_x, panel_y,
            panel_width, 2.0,
            scheme.accent,
            scheme.accent_bright,
            scheme.complement,
        );
        self.text_with_font(
            painter,
            "SPECTATORS",
            pos2(panel_x + panel_width / 2.0, panel_y + header_height / 2.0),
            Align2::CENTER_CENTER,
            Some(scheme.accent),
            FontId::proportional(font_size * 0.85),
        );

        if data.spectators.is_empty() {
            return;
        }

        // Rows — alternate between backdrop and base colors from the scheme
        let [dr, dg, db, _] = scheme.backdrop.to_srgba_unmultiplied();
        let [lr, lg, lb, _] = scheme.base.to_srgba_unmultiplied();
        let row_dark  = Color32::from_rgba_unmultiplied(dr, dg, db, 200);
        let row_light = Color32::from_rgba_unmultiplied(lr, lg, lb, 200);
        let border = Stroke::new(1.0, scheme.subtext);
        let text_color = Colors::TEXT;
        let watching_local_color = Color32::from_rgba_unmultiplied(255, 70, 70, 255);

        let last = data.spectators.len() - 1;
        for (i, entry) in data.spectators.iter().enumerate() {
            let row_y = panel_y + header_height + i as f32 * row_height;
            let is_last = i == last;
            let radii = if is_last {
                egui::CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 }
            } else {
                egui::CornerRadius::default()
            };
            let row_rect = egui::Rect::from_min_size(
                pos2(panel_x, row_y),
                egui::vec2(panel_width, row_height),
            );
            let bg = if i % 2 == 0 { row_dark } else { row_light };
            painter.rect(row_rect, radii, bg, border, egui::StrokeKind::Middle);

            let color = if entry.watching_local { watching_local_color } else { text_color };
            let label = if !entry.target.is_empty() {
                format!("{} -> {}", entry.name, entry.target)
            } else {
                entry.name.clone()
            };
            self.text_with_font(
                painter,
                label,
                pos2(panel_x + padding, row_y + row_height / 2.0),
                Align2::LEFT_CENTER,
                Some(color),
                FontId::proportional(font_size),
            );
        }
    }

    fn draw_spectator_list_interium(&self, painter: &Painter, data: &Data) {
        let scale = self.config.hud.spectator_list_scale;
        let font_size = self.config.hud.font_size * scale;
        let panel_width = font_size * 13.0;
        let header_height = font_size + 10.0 * scale;
        let row_height = font_size + 6.0 * scale;
        let padding = 6.0 * scale;

        let screen_w = data.window_size.x;
        let screen_h = data.window_size.y;

        let row_count = data.spectators.len().max(1) as f32;
        let total_height = header_height + row_height * row_count;

        let panel_x = if self.config.hud.spectator_list_x >= 0.0 {
            self.config.hud.spectator_list_x
        } else {
            screen_w - panel_width - 10.0
        };
        let panel_y = if self.config.hud.spectator_list_y >= 0.0 {
            self.config.hud.spectator_list_y
        } else {
            (screen_h - total_height) / 2.0
        };

        // Interwebz 2018 color scheme from the Lua script:
        // Outer border: (79, 78, 79), inner fill: (58, 57, 58)
        // Accent/text: (132, 125, 209) — the McolorSL/ScolorSL purple
        let accent = self.config.hud.spectator_list_color;
        let header_fill = Color32::from_rgba_unmultiplied(58, 57, 58, 230);
        let border_color = Color32::from_rgba_unmultiplied(79, 78, 79, 230);
        let border = Stroke::new(1.5, border_color);

        // Parallelogram header (slanted left at bottom, matching Lua polygon offsets)
        let skew = header_height * 0.85;
        let htl = pos2(panel_x + skew, panel_y);
        let htr = pos2(panel_x + panel_width + skew, panel_y);
        let hbr = pos2(panel_x + panel_width, panel_y + header_height);
        let hbl = pos2(panel_x, panel_y + header_height);

        painter.add(Shape::convex_polygon(
            vec![htl, htr, hbr, hbl],
            header_fill,
            border,
        ));

        self.text_with_font(
            painter,
            "SPECTATOR LIST",
            pos2(panel_x + panel_width / 2.0 + skew / 2.0, panel_y + header_height / 2.0),
            Align2::CENTER_CENTER,
            Some(accent),
            FontId::proportional(font_size * 0.85),
        );

        // Row colors: alternating dark (46, 45, 46) and medium (58, 57, 58)
        let row_dark  = Color32::from_rgba_unmultiplied(46, 45, 46, 215);
        let row_mid   = Color32::from_rgba_unmultiplied(58, 57, 58, 215);
        // Watching-local uses the same purple accent as the header text
        let watching_local_color = accent;
        let rows_start_y = panel_y + header_height;

        if data.spectators.is_empty() {
            return;
        }

        let last = data.spectators.len() - 1;
        for (i, entry) in data.spectators.iter().enumerate() {
            let row_y = rows_start_y + i as f32 * row_height;
            let is_last = i == last;
            let radii = if is_last {
                egui::CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 }
            } else {
                egui::CornerRadius::default()
            };
            let row_rect = egui::Rect::from_min_size(
                pos2(panel_x, row_y),
                egui::vec2(panel_width, row_height),
            );
            let bg = if i % 2 == 0 { row_dark } else { row_mid };
            painter.rect(row_rect, radii, bg, border, egui::StrokeKind::Middle);

            let color = if entry.watching_local { watching_local_color } else { Colors::TEXT };
            let label = if !entry.target.is_empty() {
                format!("{} -> {}", entry.name, entry.target)
            } else {
                entry.name.clone()
            };
            self.text_with_font(
                painter,
                label,
                pos2(panel_x + padding, row_y + row_height / 2.0),
                Align2::LEFT_CENTER,
                Some(color),
                FontId::proportional(font_size),
            );
        }
    }

    pub fn draw_bomb_timer(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.bomb_timer || !data.bomb.planted {
            return;
        }

        let font_size = self.config.hud.font_size;
        let font = FontId::proportional(font_size);
        let opacity = self.config.hud.weapon_esp_background_opacity;
        let bg_color = Color32::from_rgba_unmultiplied(0, 0, 0, opacity);
        let text_color = self.config.hud.text_color;

        if let Some(pos) = world_to_screen(&data.bomb.position, data) {
            // Timer text with background
            let timer_str = format!("{:.3}", data.bomb.timer);
            let galley = painter.layout_no_wrap(timer_str.clone(), font.clone(), text_color);
            let sz = galley.rect.size();
            let timer_rect = egui::Rect::from_center_size(pos, sz).expand(4.0);
            painter.rect_filled(timer_rect, 2.0, bg_color);
            self.text(painter, timer_str, pos, Align2::CENTER_CENTER, None);

            if data.bomb.being_defused {
                let defuse_pos = pos2(pos.x, pos.y + font_size);
                let defuse_str = format!("defusing {:.3}", data.bomb.defuse_remain_time);
                let galley2 =
                    painter.layout_no_wrap(defuse_str.clone(), font.clone(), text_color);
                let sz2 = galley2.rect.size();
                let defuse_rect = egui::Rect::from_center_size(defuse_pos, sz2).expand(4.0);
                painter.rect_filled(defuse_rect, 2.0, bg_color);
                self.text(painter, defuse_str, defuse_pos, Align2::CENTER_CENTER, None);
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
        let entries = self.collect_keybind_entries(data);
        match self.config.hud.keybind_list_style {
            KeybindListStyle::Simple => self.draw_keybind_list_simple(painter, data, &entries),
            KeybindListStyle::New => self.draw_keybind_list_new(painter, data, &entries),
            KeybindListStyle::Interium => self.draw_keybind_list_interium(painter, data, &entries),
        }
    }

    /// Collect the active keybind rows as `(name, mode_label)` pairs.
    fn collect_keybind_entries(&self, data: &Data) -> Vec<(String, String)> {
        let mut entries = Vec::new();

        if self.config.hud.keybind_aimbot && data.aimbot_active {
            entries.push(("Aimbot".into(), "[hold]".into()));
        }

        if self.config.hud.keybind_triggerbot && data.triggerbot_active {
            let mode = format!("[{:?}]", self.triggerbot_config(&data.weapon).mode).to_lowercase();
            entries.push(("Triggerbot".into(), mode));
        }

        if self.config.hud.keybind_trigger_activate && data.autowall_active {
            let mode = format!("[{:?}]", self.config.aim.autowall_mode).to_lowercase();
            entries.push(("Autowall".into(), mode));
        }

        if self.config.hud.keybind_esp && data.esp_active {
            entries.push(("ESP".into(), "[toggle]".into()));
        }

        if self.config.hud.keybind_backtrack && self.config.player.backtrack_visual {
            let bt_ms = self.config.player.backtrack_ms;
            entries.push((format!("Backtrack +{}ms", bt_ms), "[always]".into()));
        }

        if self.config.hud.keybind_fov {
            let ab = self.aimbot_config(&data.weapon);
            entries.push((format!("FOV {:.1} / Smooth {:.1}", ab.fov, ab.smooth), "[info]".into()));
        }

        if self.config.hud.keybind_trigger_delay {
            let tb = self.triggerbot_config(&data.weapon);
            entries.push((format!("Delay {}-{}ms", tb.delay.start(), tb.delay.end()), "[info]".into()));
        }

        if self.config.hud.keybind_server_picker && !self.server_regions.is_empty() {
            let enabled: Vec<&str> = self
                .server_regions
                .iter()
                .filter(|r| !r.blocked)
                .map(|r| r.name.as_str())
                .collect();
            if !enabled.is_empty() {
                let label = if enabled.len() == self.server_regions.len() {
                    "QD: all".into()
                } else {
                    format!("QD: {}", enabled.join(", "))
                };
                entries.push((label, "[always]".into()));
            }
        }

        if self.config.hud.keybind_ping {
            let ping = data.ping.max(0);
            entries.push((format!("Ping: {}ms", ping), "[info]".into()));
        }

        entries
    }

    fn draw_keybind_list_simple(&self, painter: &Painter, data: &Data, entries: &[(String, String)]) {
        let scale = self.config.hud.keybind_list_scale;
        let font_size = self.config.hud.font_size * scale;
        let x = if self.config.hud.keybind_list_x >= 0.0 {
            self.config.hud.keybind_list_x
        } else {
            10.0
        };
        let y = if self.config.hud.keybind_list_y >= 0.0 {
            self.config.hud.keybind_list_y
        } else {
            data.window_size.y / 2.0
        };

        let mut text_lines = vec!["Keybinds:".to_string()];
        for (name, mode) in entries {
            text_lines.push(format!("  {} {}", name, mode));
        }
        let text = text_lines.join("\n");

        if self.config.hud.keybind_list_simple_backdrop {
            let galley = painter.layout(
                text.clone(),
                FontId::proportional(font_size),
                Colors::TEXT,
                f32::INFINITY,
            );
            let padding = 4.0;
            painter.rect_filled(
                egui::Rect::from_min_size(
                    pos2(x - padding, y - padding),
                    egui::vec2(galley.size().x + padding * 2.0, galley.size().y + padding * 2.0),
                ),
                egui::CornerRadius::same(2),
                Color32::from_rgba_unmultiplied(0, 0, 0, 140),
            );
        }
        self.text_with_font(
            painter,
            text,
            pos2(x, y),
            Align2::LEFT_TOP,
            Some(Colors::TEXT),
            FontId::proportional(font_size),
        );
    }

    fn draw_keybind_list_new(&self, painter: &Painter, data: &Data, entries: &[(String, String)]) {
        let scheme = ColorScheme::for_style(&self.config.accent_style);

        let scale = self.config.hud.keybind_list_scale;
        let font_size = self.config.hud.font_size * scale;
        let panel_width = font_size * 12.0;
        let header_height = font_size + 8.0 * scale;
        let row_height = font_size + 4.0 * scale;
        let padding = 8.0 * scale;

        let row_count = entries.len().max(1);
        let total_height = header_height + row_height * row_count as f32;

        let screen_w = data.window_size.x;
        let screen_h = data.window_size.y;

        let panel_x = if self.config.hud.keybind_list_x >= 0.0 {
            self.config.hud.keybind_list_x
        } else {
            screen_w - panel_width - 10.0
        };
        let panel_y = if self.config.hud.keybind_list_y >= 0.0 {
            self.config.hud.keybind_list_y
        } else {
            (screen_h - total_height) / 2.0
        };

        // Header
        let header_rect = egui::Rect::from_min_size(
            pos2(panel_x, panel_y),
            egui::vec2(panel_width, header_height),
        );
        let header_radius = if entries.is_empty() {
            egui::CornerRadius { nw: 2, ne: 2, sw: 2, se: 2 }
        } else {
            egui::CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 }
        };
        let [hr, hg, hb, _] = scheme.highlight.to_srgba_unmultiplied();
        painter.rect(
            header_rect,
            header_radius,
            Color32::from_rgba_unmultiplied(hr, hg, hb, 220),
            Stroke::new(1.0, scheme.subtext),
            egui::StrokeKind::Middle,
        );
        // Gradient accent line at top of header
        draw_gradient_accent_line(
            painter,
            panel_x, panel_y,
            panel_width, 2.0,
            scheme.accent,
            scheme.accent_bright,
            scheme.complement,
        );

        self.text_with_font(
            painter,
            "KEYBINDS",
            pos2(panel_x + panel_width / 2.0, panel_y + header_height / 2.0),
            Align2::CENTER_CENTER,
            Some(scheme.accent),
            FontId::proportional(font_size * 0.85),
        );

        if entries.is_empty() {
            return;
        }

        // Rows
        let [dr, dg, db, _] = scheme.backdrop.to_srgba_unmultiplied();
        let [lr, lg, lb, _] = scheme.base.to_srgba_unmultiplied();
        let row_dark  = Color32::from_rgba_unmultiplied(dr, dg, db, 200);
        let row_light = Color32::from_rgba_unmultiplied(lr, lg, lb, 200);
        let border = Stroke::new(1.0, scheme.subtext);

        let last = entries.len() - 1;
        for (i, (name, mode)) in entries.iter().enumerate() {
            let row_y = panel_y + header_height + i as f32 * row_height;
            let is_last = i == last;
            let radii = if is_last {
                egui::CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 }
            } else {
                egui::CornerRadius::default()
            };
            let row_rect = egui::Rect::from_min_size(
                pos2(panel_x, row_y),
                egui::vec2(panel_width, row_height),
            );
            let bg = if i % 2 == 0 { row_dark } else { row_light };
            painter.rect(row_rect, radii, bg, border, egui::StrokeKind::Middle);

            // Name on left
            self.text_with_font(
                painter,
                name,
                pos2(panel_x + padding, row_y + row_height / 2.0),
                Align2::LEFT_CENTER,
                Some(Colors::TEXT),
                FontId::proportional(font_size),
            );
            // Mode on right
            self.text_with_font(
                painter,
                mode,
                pos2(panel_x + panel_width - padding, row_y + row_height / 2.0),
                Align2::RIGHT_CENTER,
                Some(scheme.subtext),
                FontId::proportional(font_size * 0.85),
            );
        }
    }

    fn draw_keybind_list_interium(&self, painter: &Painter, data: &Data, entries: &[(String, String)]) {
        let scale = self.config.hud.keybind_list_scale;
        let font_size = self.config.hud.font_size * scale;
        let panel_width = font_size * 13.0;
        let header_height = font_size + 10.0 * scale;
        let row_height = font_size + 6.0 * scale;
        let padding = 6.0 * scale;

        let screen_w = data.window_size.x;
        let screen_h = data.window_size.y;

        let row_count = entries.len().max(1) as f32;
        let total_height = header_height + row_height * row_count;

        let panel_x = if self.config.hud.keybind_list_x >= 0.0 {
            self.config.hud.keybind_list_x
        } else {
            screen_w - panel_width - 10.0
        };
        let panel_y = if self.config.hud.keybind_list_y >= 0.0 {
            self.config.hud.keybind_list_y
        } else {
            (screen_h - total_height) / 2.0
        };

        let accent = self.config.hud.spectator_list_color;
        let header_fill = Color32::from_rgba_unmultiplied(58, 57, 58, 230);
        let border_color = Color32::from_rgba_unmultiplied(79, 78, 79, 230);
        let border = Stroke::new(1.5, border_color);

        // Parallelogram header
        let skew = header_height * 0.85;
        let htl = pos2(panel_x + skew, panel_y);
        let htr = pos2(panel_x + panel_width + skew, panel_y);
        let hbr = pos2(panel_x + panel_width, panel_y + header_height);
        let hbl = pos2(panel_x, panel_y + header_height);

        painter.add(Shape::convex_polygon(
            vec![htl, htr, hbr, hbl],
            header_fill,
            border,
        ));

        self.text_with_font(
            painter,
            "KEYBINDS",
            pos2(panel_x + panel_width / 2.0 + skew / 2.0, panel_y + header_height / 2.0),
            Align2::CENTER_CENTER,
            Some(accent),
            FontId::proportional(font_size * 0.85),
        );

        // Rows
        let row_dark = Color32::from_rgba_unmultiplied(46, 45, 46, 215);
        let row_mid  = Color32::from_rgba_unmultiplied(58, 57, 58, 215);
        let rows_start_y = panel_y + header_height;

        if entries.is_empty() {
            let row_rect = egui::Rect::from_min_size(
                pos2(panel_x, rows_start_y),
                egui::vec2(panel_width, row_height),
            );
            painter.rect(
                row_rect,
                egui::CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 },
                row_dark,
                border,
                egui::StrokeKind::Middle,
            );
            self.text_with_font(
                painter,
                "none",
                pos2(panel_x + padding, rows_start_y + row_height / 2.0),
                Align2::LEFT_CENTER,
                Some(Color32::from_gray(130)),
                FontId::proportional(font_size),
            );
            return;
        }

        let last = entries.len() - 1;
        for (i, (name, mode)) in entries.iter().enumerate() {
            let row_y = rows_start_y + i as f32 * row_height;
            let is_last = i == last;
            let radii = if is_last {
                egui::CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 }
            } else {
                egui::CornerRadius::default()
            };
            let row_rect = egui::Rect::from_min_size(
                pos2(panel_x, row_y),
                egui::vec2(panel_width, row_height),
            );
            let bg = if i % 2 == 0 { row_dark } else { row_mid };
            painter.rect(row_rect, radii, bg, border, egui::StrokeKind::Middle);

            // Name on left
            self.text_with_font(
                painter,
                name,
                pos2(panel_x + padding, row_y + row_height / 2.0),
                Align2::LEFT_CENTER,
                Some(Colors::TEXT),
                FontId::proportional(font_size),
            );
            // Mode on right
            self.text_with_font(
                painter,
                mode,
                pos2(panel_x + panel_width - padding, row_y + row_height / 2.0),
                Align2::RIGHT_CENTER,
                Some(accent),
                FontId::proportional(font_size * 0.85),
            );
        }
    }

    pub fn draw_sniper_crosshair(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.sniper_crosshair
            || WeaponClass::from_string(data.weapon.as_ref()) != WeaponClass::Sniper
        {
            return;
        }

        let cx = data.window_size.x / 2.0;
        let cy = data.window_size.y / 2.0;
        let h = self.config.hud.sniper_crosshair_h;
        let v = self.config.hud.sniper_crosshair_v;
        let gap = self.config.hud.sniper_crosshair_gap;
        let thickness = self.config.hud.sniper_crosshair_thickness.max(0.5);
        let stroke = Stroke::new(thickness, self.config.hud.crosshair_color);

        painter.line(vec![pos2(cx, cy - gap - v), pos2(cx, cy - gap)], stroke);
        painter.line(vec![pos2(cx, cy + gap), pos2(cx, cy + gap + v)], stroke);
        painter.line(vec![pos2(cx - gap - h, cy), pos2(cx - gap, cy)], stroke);
        painter.line(vec![pos2(cx + gap, cy), pos2(cx + gap + h, cy)], stroke);
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
        let margin = self.config.hud.fov_arrow_margin;
        let arrow_size = self.config.hud.fov_arrow_size;

        // Place arrows on a circle so every arrow is the same distance from
        // the crosshair regardless of direction.  Radius = half the shorter
        // screen edge minus the margin (same mental model as before: larger
        // margin → arrows closer to the crosshair).
        let radius = (data.window_size.x.min(data.window_size.y) * 0.5 - margin).max(10.0);

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

            // NDC direction toward the enemy (clip_y negated: NDC y-up → screen y-down).
            // When w < 0 (enemy behind camera) flip both to point the correct way.
            let sign = if w > 0.0 { 1.0_f32 } else { -1.0_f32 };
            let ndc_x = sign * (clip_x / w);
            let ndc_y = sign * -(clip_y / w);

            // Convert NDC direction to screen-pixel direction so the arrow
            // radius is isotropic (same pixel distance in every direction).
            let px = ndc_x * data.window_size.x * 0.5;
            let py = ndc_y * data.window_size.y * 0.5;
            let plen = (px * px + py * py).sqrt();
            if plen < 0.001 {
                continue;
            }
            let dir_x = px / plen;
            let dir_y = py / plen;

            // Arrow anchor on the circle
            let arrow_x = center_x + dir_x * radius;
            let arrow_y = center_y + dir_y * radius;

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


    pub fn draw_penetration_crosshair(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.penetration_crosshair_enabled || !data.in_game {
            return;
        }

        let color = match &data.penetration_crosshair_state {
            PenetrationCrosshairState::CanWallbang => self.config.hud.penetration_color_yes,
            PenetrationCrosshairState::CannotWallbang => self.config.hud.penetration_color_no,
            PenetrationCrosshairState::Unavailable => self.config.hud.penetration_color_unavailable,
        };

        let center = pos2(data.window_size.x / 2.0, data.window_size.y / 2.0);
        let half = 1.0_f32;
        let outline = 1.0_f32;

        // Draw black outline behind the square.
        let outline_rect = egui::Rect::from_center_size(
            center,
            egui::vec2((half + outline) * 2.0, (half + outline) * 2.0),
        );
        painter.rect_filled(outline_rect, 0.0, Color32::BLACK);

        // Draw the colored square.
        let inner_rect = egui::Rect::from_center_size(center, egui::vec2(half * 2.0, half * 2.0));
        painter.rect_filled(inner_rect, 0.0, color);
    }
    pub fn draw_media_info(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.media_info || self.media_info_text.is_empty() {
            return;
        }
        match self.config.hud.media_info_style {
            MediaInfoStyle::Simple => self.draw_media_info_simple(painter, data),
            MediaInfoStyle::New => self.draw_media_info_new(painter, data),
            MediaInfoStyle::Interium => self.draw_media_info_interium(painter, data),
        }
    }

    fn draw_media_info_simple(&self, painter: &Painter, data: &Data) {
        let scale = self.config.hud.media_info_scale;
        let font_size = self.config.hud.font_size * scale;
        let screen_w = data.window_size.x;
        let text = format!("♪ {}", self.media_info_text);
        let galley = painter.layout_no_wrap(
            text.clone(),
            FontId::proportional(font_size),
            self.config.hud.media_info_color,
        );
        let x = if self.config.hud.media_info_x >= 0.0 {
            self.config.hud.media_info_x
        } else {
            screen_w - galley.size().x - 10.0
        };
        let y = if self.config.hud.media_info_y >= 0.0 {
            self.config.hud.media_info_y
        } else {
            10.0
        };
        if self.config.hud.media_info_simple_backdrop {
            let padding = 4.0;
            painter.rect_filled(
                egui::Rect::from_min_size(
                    pos2(x - padding, y - padding),
                    egui::vec2(galley.size().x + padding * 2.0, galley.size().y + padding * 2.0),
                ),
                egui::CornerRadius::same(2),
                Color32::from_rgba_unmultiplied(0, 0, 0, 140),
            );
        }
        self.text_with_font(
            painter,
            text,
            pos2(x, y),
            Align2::LEFT_TOP,
            Some(self.config.hud.media_info_color),
            FontId::proportional(font_size),
        );
    }

    fn draw_media_info_new(&self, painter: &Painter, data: &Data) {
        let scheme = ColorScheme::for_style(&self.config.accent_style);

        let scale = self.config.hud.media_info_scale;
        let font_size = self.config.hud.font_size * scale;
        let header_height = font_size + 8.0 * scale;
        let row_height = font_size + 4.0 * scale;
        let padding = 8.0 * scale;

        let text_galley = painter.layout_no_wrap(
            self.media_info_text.clone(),
            FontId::proportional(font_size),
            self.config.hud.media_info_color,
        );
        let panel_width = (text_galley.size().x + padding * 2.0).max(font_size * 10.0);

        let screen_w = data.window_size.x;
        let panel_x = if self.config.hud.media_info_x >= 0.0 {
            self.config.hud.media_info_x
        } else {
            screen_w - panel_width - 10.0
        };
        let panel_y = if self.config.hud.media_info_y >= 0.0 {
            self.config.hud.media_info_y
        } else {
            10.0
        };

        let [hr, hg, hb, _] = scheme.highlight.to_srgba_unmultiplied();
        painter.rect(
            egui::Rect::from_min_size(
                pos2(panel_x, panel_y),
                egui::vec2(panel_width, header_height),
            ),
            egui::CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 },
            Color32::from_rgba_unmultiplied(hr, hg, hb, 220),
            Stroke::new(1.0, scheme.subtext),
            egui::StrokeKind::Middle,
        );
        self.text_with_font(
            painter,
            "NOW PLAYING",
            pos2(panel_x + panel_width / 2.0, panel_y + header_height / 2.0),
            Align2::CENTER_CENTER,
            Some(scheme.accent),
            FontId::proportional(font_size * 0.85),
        );

        let row_y = panel_y + header_height;
        let [dr, dg, db, _] = scheme.backdrop.to_srgba_unmultiplied();
        painter.rect(
            egui::Rect::from_min_size(
                pos2(panel_x, row_y),
                egui::vec2(panel_width, row_height),
            ),
            egui::CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 },
            Color32::from_rgba_unmultiplied(dr, dg, db, 200),
            Stroke::new(1.0, scheme.subtext),
            egui::StrokeKind::Middle,
        );
        self.text_with_font(
            painter,
            &self.media_info_text,
            pos2(panel_x + padding, row_y + row_height / 2.0),
            Align2::LEFT_CENTER,
            Some(self.config.hud.media_info_color),
            FontId::proportional(font_size),
        );
    }

    fn draw_media_info_interium(&self, painter: &Painter, data: &Data) {
        let scale = self.config.hud.media_info_scale;
        let font_size = self.config.hud.font_size * scale;
        let header_height = font_size + 10.0 * scale;
        let row_height = font_size + 6.0 * scale;
        let padding = 6.0 * scale;

        let text_galley = painter.layout_no_wrap(
            self.media_info_text.clone(),
            FontId::proportional(font_size),
            self.config.hud.media_info_color,
        );
        let panel_width = (text_galley.size().x + padding * 2.0).max(font_size * 10.0);

        let screen_w = data.window_size.x;
        let panel_x = if self.config.hud.media_info_x >= 0.0 {
            self.config.hud.media_info_x
        } else {
            screen_w - panel_width - 10.0
        };
        let panel_y = if self.config.hud.media_info_y >= 0.0 {
            self.config.hud.media_info_y
        } else {
            10.0
        };

        let accent = self.config.hud.media_info_color;
        let header_fill = Color32::from_rgba_unmultiplied(58, 57, 58, 230);
        let border_color = Color32::from_rgba_unmultiplied(79, 78, 79, 230);
        let border = Stroke::new(1.5, border_color);

        let skew = header_height * 0.85;
        let htl = pos2(panel_x + skew, panel_y);
        let htr = pos2(panel_x + panel_width + skew, panel_y);
        let hbr = pos2(panel_x + panel_width, panel_y + header_height);
        let hbl = pos2(panel_x, panel_y + header_height);

        painter.add(Shape::convex_polygon(
            vec![htl, htr, hbr, hbl],
            header_fill,
            border,
        ));
        self.text_with_font(
            painter,
            "NOW PLAYING",
            pos2(panel_x + panel_width / 2.0 + skew / 2.0, panel_y + header_height / 2.0),
            Align2::CENTER_CENTER,
            Some(accent),
            FontId::proportional(font_size * 0.85),
        );

        let row_y = panel_y + header_height;
        painter.rect(
            egui::Rect::from_min_size(
                pos2(panel_x, row_y),
                egui::vec2(panel_width, row_height),
            ),
            egui::CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 },
            Color32::from_rgba_unmultiplied(46, 45, 46, 215),
            border,
            egui::StrokeKind::Middle,
        );
        self.text_with_font(
            painter,
            &self.media_info_text,
            pos2(panel_x + padding, row_y + row_height / 2.0),
            Align2::LEFT_CENTER,
            Some(accent),
            FontId::proportional(font_size),
        );
    }

    pub fn draw_watermark(&self, painter: &Painter, data: &Data) {
        if !self.config.hud.watermark {
            return;
        }

        // Build the watermark text: "deadlocked | <date time> | <weather>"
        let now = std::time::SystemTime::now();
        let datetime_str = {
            let secs = now
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as nix::libc::time_t;
            let mut tm: nix::libc::tm = unsafe { std::mem::zeroed() };
            unsafe { nix::libc::localtime_r(&secs, &mut tm); }
            let year = tm.tm_year as i64 + 1900;
            let month = tm.tm_mon + 1;
            let day = tm.tm_mday;
            let hours = tm.tm_hour;
            let minutes = tm.tm_min;
            format!("{year}-{month:02}-{day:02} {hours:02}:{minutes:02}")
        };

        let display_text = if !self.watermark_text.is_empty() {
            format!("bosshook420 | {} | {}", datetime_str, self.watermark_text)
        } else {
            format!("bosshook420 | {}", datetime_str)
        };

        let scheme = ColorScheme::for_style(&self.config.accent_style);

        let scale = self.config.hud.watermark_scale;
        let font_size = self.config.hud.font_size * scale;
        let row_height = font_size + 6.0 * scale;
        let padding = 8.0 * scale;

        let text_galley = painter.layout_no_wrap(
            display_text.clone(),
            FontId::proportional(font_size),
            self.config.hud.watermark_color,
        );
        let panel_width = text_galley.size().x + padding * 2.0 + 3.0 * scale;

        let screen_w = data.window_size.x;
        let panel_x = if self.config.hud.watermark_x >= 0.0 {
            self.config.hud.watermark_x
        } else {
            screen_w - panel_width - 10.0
        };
        let panel_y = if self.config.hud.watermark_y >= 0.0 {
            self.config.hud.watermark_y
        } else {
            10.0
        };

        let [br, bg, bb, _] = scheme.base.to_srgba_unmultiplied();
        painter.rect(
            egui::Rect::from_min_size(
                pos2(panel_x, panel_y),
                egui::vec2(panel_width, row_height),
            ),
            egui::CornerRadius::same(2),
            Color32::from_rgba_unmultiplied(br, bg, bb, 210),
            Stroke::new(1.0, scheme.accent),
            egui::StrokeKind::Middle,
        );

        // Accent left bar
        painter.rect_filled(
            egui::Rect::from_min_size(
                pos2(panel_x, panel_y),
                egui::vec2(3.0 * scale, row_height),
            ),
            egui::CornerRadius { nw: 2, ne: 0, sw: 2, se: 0 },
            scheme.accent,
        );

        self.text_with_font(
            painter,
            &display_text,
            pos2(panel_x + 3.0 * scale + padding, panel_y + row_height / 2.0),
            Align2::LEFT_CENTER,
            Some(self.config.hud.watermark_color),
            FontId::proportional(font_size),
        );
    }
}