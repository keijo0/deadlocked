//! HUD settings panel.
use crate::config::{Config, KeybindListStyle, MediaInfoStyle, SpectatorListStyle, WeaponBoxMode};
use super::color_edit;

pub struct HudPanel;

impl HudPanel {
    pub fn new() -> Self {
        Self
    }

    /// Returns `true` when any setting changed.
    pub fn render(&self, ui: &imgui::Ui, config: &mut Config) -> bool {
        let mut changed = false;
        let h = &mut config.hud;

        // ── Two-column layout ─────────────────────────────────────────────
        let avail = ui.content_region_avail()[0];
        let col_w = (avail - 8.0) / 2.0;

        ui.child_window("##hud_left")
            .size([col_w, 0.0])
            .build(|| {
                changed |= ui.checkbox("Bomb Timer", &mut h.bomb_timer);
                changed |= ui.checkbox("Spectator List", &mut h.spectator_list);

                if h.spectator_list {
                    let styles = ["Simple", "New", "Interwebz"];
                    let mut style_idx: usize = match h.spectator_list_style {
                        SpectatorListStyle::Simple => 0,
                        SpectatorListStyle::New => 1,
                        SpectatorListStyle::Interium => 2,
                    };
                    if ui.combo_simple_string("  SL Style", &mut style_idx, &styles) {
                        h.spectator_list_style = match style_idx {
                            0 => SpectatorListStyle::Simple,
                            2 => SpectatorListStyle::Interium,
                            _ => SpectatorListStyle::New,
                        };
                        changed = true;
                    }
                    let mut sl_scale_pct = h.spectator_list_scale * 100.0;
                    if ui.slider_config("  SL Scale %", 10.0_f32, 300.0_f32)
                        .display_format("%.0f%%")
                        .build(&mut sl_scale_pct)
                    {
                        h.spectator_list_scale = (sl_scale_pct / 100.0).clamp(0.1, 3.0);
                        changed = true;
                    }
                    changed |= color_edit(ui, "##sl_col", &mut h.spectator_list_color);
                    ui.same_line();
                    ui.text("SL Color");
                    if h.spectator_list_style == SpectatorListStyle::Simple {
                        changed |= ui.checkbox("  SL Backdrop", &mut h.spectator_list_simple_backdrop);
                    }
                    if ui.slider_config("  SL X", -1.0_f32, 3840.0_f32)
                        .display_format("%.0f")
                        .build(&mut h.spectator_list_x)
                    {
                        changed = true;
                    }
                    if ui.slider_config("  SL Y", -1.0_f32, 2160.0_f32)
                        .display_format("%.0f")
                        .build(&mut h.spectator_list_y)
                    {
                        changed = true;
                    }
                    if ui.button("  Reset SL Pos") {
                        h.spectator_list_x = -1.0;
                        h.spectator_list_y = -1.0;
                        changed = true;
                    }
                }

                // Sniper crosshair + inline color
                changed |= color_edit(ui, "##xhair_col", &mut h.crosshair_color);
                ui.same_line();
                changed |= ui.checkbox("Sniper Crosshair", &mut h.sniper_crosshair);
                if h.sniper_crosshair {
                    if ui.slider_config("  XH Horizontal", 1.0_f32, 100.0_f32)
                        .display_format("%.0f")
                        .build(&mut h.sniper_crosshair_h)
                    {
                        changed = true;
                    }
                    if ui.slider_config("  XH Vertical", 1.0_f32, 100.0_f32)
                        .display_format("%.0f")
                        .build(&mut h.sniper_crosshair_v)
                    {
                        changed = true;
                    }
                    if ui.slider_config("  XH Gap", 0.0_f32, 30.0_f32)
                        .display_format("%.0f")
                        .build(&mut h.sniper_crosshair_gap)
                    {
                        changed = true;
                    }
                    if ui.slider_config("  XH Thickness", 0.5_f32, 10.0_f32)
                        .display_format("%.1f")
                        .build(&mut h.sniper_crosshair_thickness)
                    {
                        changed = true;
                    }
                }

                // Penetration crosshair + inline colors
                if h.penetration_crosshair_enabled {
                    changed |= color_edit(ui, "##pen_yes", &mut h.penetration_color_yes);
                    ui.same_line();
                    changed |= color_edit(ui, "##pen_no", &mut h.penetration_color_no);
                    ui.same_line();
                    changed |= color_edit(ui, "##pen_na", &mut h.penetration_color_unavailable);
                    ui.same_line();
                }
                changed |= ui.checkbox("Penetration Crosshair", &mut h.penetration_crosshair_enabled);

                changed |= ui.checkbox("Dropped Weapons", &mut h.dropped_weapons);

                if h.dropped_weapons {
                    changed |= ui.checkbox("  Weapon Box", &mut h.weapon_box);
                    if h.weapon_box {
                        let wbox_modes = ["Full", "Gap"];
                        let mut wbm_idx: usize = match h.weapon_box_mode {
                            WeaponBoxMode::Full => 0,
                            WeaponBoxMode::Gap => 1,
                        };
                        if ui.combo_simple_string("  Box Mode##wb", &mut wbm_idx, &wbox_modes) {
                            h.weapon_box_mode = match wbm_idx {
                                1 => WeaponBoxMode::Gap,
                                _ => WeaponBoxMode::Full,
                            };
                            changed = true;
                        }
                    }
                    changed |= ui.checkbox("  Weapon ESP Colors", &mut h.weapon_esp_use_colors);
                    let mut bg_opacity = h.weapon_esp_background_opacity as i32;
                    if ui.slider_config("  Label BG Opacity", 0_i32, 255_i32)
                        .build(&mut bg_opacity)
                    {
                        h.weapon_esp_background_opacity = bg_opacity.clamp(0, 255) as u8;
                        changed = true;
                    }
                    let mut max_dist = h.weapon_esp_max_distance;
                    if ui.slider_config("  Max Distance", 0.0_f32, 10000.0_f32)
                        .display_format("%.0f u")
                        .build(&mut max_dist)
                    {
                        h.weapon_esp_max_distance = max_dist;
                        changed = true;
                    }
                }

                changed |= ui.checkbox("Grenade Trails", &mut h.grenade_trails);

                // FOV arrows + inline color
                changed |= color_edit(ui, "##fov_col", &mut h.fov_arrow_color);
                ui.same_line();
                changed |= ui.checkbox("FOV Arrows", &mut h.fov_arrows);

                changed |= ui.checkbox("Keybind List", &mut h.keybind_list);

                if h.keybind_list {
                    let kb_styles = ["Simple", "New", "Interwebz"];
                    let mut kb_style_idx: usize = match h.keybind_list_style {
                        KeybindListStyle::Simple => 0,
                        KeybindListStyle::New => 1,
                        KeybindListStyle::Interium => 2,
                    };
                    if ui.combo_simple_string("  KB Style", &mut kb_style_idx, &kb_styles) {
                        h.keybind_list_style = match kb_style_idx {
                            0 => KeybindListStyle::Simple,
                            2 => KeybindListStyle::Interium,
                            _ => KeybindListStyle::New,
                        };
                        changed = true;
                    }
                    let mut kb_scale_pct = h.keybind_list_scale * 100.0;
                    if ui.slider_config("  KB Scale %", 10.0_f32, 300.0_f32)
                        .display_format("%.0f%%")
                        .build(&mut kb_scale_pct)
                    {
                        h.keybind_list_scale = (kb_scale_pct / 100.0).clamp(0.1, 3.0);
                        changed = true;
                    }
                    if h.keybind_list_style == KeybindListStyle::Simple {
                        changed |= ui.checkbox("  KB Backdrop", &mut h.keybind_list_simple_backdrop);
                    }
                    if h.keybind_list_style != KeybindListStyle::Simple {
                        if ui.slider_config("  KB X", -1.0_f32, 3840.0_f32)
                            .display_format("%.0f")
                            .build(&mut h.keybind_list_x)
                        {
                            changed = true;
                        }
                        if ui.slider_config("  KB Y", -1.0_f32, 2160.0_f32)
                            .display_format("%.0f")
                            .build(&mut h.keybind_list_y)
                        {
                            changed = true;
                        }
                        if ui.button("  Reset KB Pos") {
                            h.keybind_list_x = -1.0;
                            h.keybind_list_y = -1.0;
                            changed = true;
                        }
                    }

                    for (label, val) in [
                        ("  Aimbot",           &mut h.keybind_aimbot as *mut bool),
                        ("  FOV/Smooth",       &mut h.keybind_fov as *mut bool),
                        ("  Triggerbot",       &mut h.keybind_triggerbot as *mut bool),
                        ("  Trg Delay",        &mut h.keybind_trigger_delay as *mut bool),
                        ("  Autowall",         &mut h.keybind_autowall as *mut bool),
                        ("  Trg Activate",     &mut h.keybind_trigger_activate as *mut bool),
                        ("  Trg Indicator",    &mut h.keybind_trigger_active_indicator as *mut bool),
                        ("  Backtrack",        &mut h.keybind_backtrack as *mut bool),
                        ("  ESP",              &mut h.keybind_esp as *mut bool),
                        ("  Server Picker",    &mut h.keybind_server_picker as *mut bool),
                        ("  Ping",             &mut h.keybind_ping as *mut bool),
                    ] {
                        if ui.checkbox(label, unsafe { &mut *val }) {
                            changed = true;
                        }
                    }
                }

                // Media info + inline color
                if h.media_info {
                    changed |= color_edit(ui, "##mi_col", &mut h.media_info_color);
                    ui.same_line();
                }
                changed |= ui.checkbox("Media Info", &mut h.media_info);
                if h.media_info {
                    let mi_styles = ["Simple", "New", "Interwebz"];
                    let mut mi_style_idx: usize = match h.media_info_style {
                        MediaInfoStyle::Simple => 0,
                        MediaInfoStyle::New => 1,
                        MediaInfoStyle::Interium => 2,
                    };
                    if ui.combo_simple_string("  MI Style", &mut mi_style_idx, &mi_styles) {
                        h.media_info_style = match mi_style_idx {
                            0 => MediaInfoStyle::Simple,
                            2 => MediaInfoStyle::Interium,
                            _ => MediaInfoStyle::New,
                        };
                        changed = true;
                    }
                    if h.media_info_style == MediaInfoStyle::Simple {
                        changed |= ui.checkbox("  MI Backdrop", &mut h.media_info_simple_backdrop);
                    }
                    let mut mi_scale_pct = h.media_info_scale * 100.0;
                    if ui.slider_config("  MI Scale %", 10.0_f32, 300.0_f32)
                        .display_format("%.0f%%")
                        .build(&mut mi_scale_pct)
                    {
                        h.media_info_scale = (mi_scale_pct / 100.0).clamp(0.1, 3.0);
                        changed = true;
                    }
                    if ui.slider_config("  MI X", -1.0_f32, 3840.0_f32)
                        .display_format("%.0f")
                        .build(&mut h.media_info_x)
                    {
                        changed = true;
                    }
                    if ui.slider_config("  MI Y", -1.0_f32, 2160.0_f32)
                        .display_format("%.0f")
                        .build(&mut h.media_info_y)
                    {
                        changed = true;
                    }
                    if ui.button("  Reset MI Pos") {
                        h.media_info_x = -1.0;
                        h.media_info_y = -1.0;
                        changed = true;
                    }
                }

                // Watermark
                if h.watermark {
                    changed |= color_edit(ui, "##wm_col", &mut h.watermark_color);
                    ui.same_line();
                }
                changed |= ui.checkbox("Watermark", &mut h.watermark);
                if h.watermark {
                    changed |= ui.checkbox("  Weather Info", &mut h.watermark_weather);
                    let mut wm_scale_pct = h.watermark_scale * 100.0;
                    if ui.slider_config("  WM Scale %", 10.0_f32, 300.0_f32)
                        .display_format("%.0f%%")
                        .build(&mut wm_scale_pct)
                    {
                        h.watermark_scale = (wm_scale_pct / 100.0).clamp(0.1, 3.0);
                        changed = true;
                    }
                    if ui.slider_config("  WM X", -1.0_f32, 3840.0_f32)
                        .display_format("%.0f")
                        .build(&mut h.watermark_x)
                    {
                        changed = true;
                    }
                    if ui.slider_config("  WM Y", -1.0_f32, 2160.0_f32)
                        .display_format("%.0f")
                        .build(&mut h.watermark_y)
                    {
                        changed = true;
                    }
                    if ui.button("  Reset WM Pos") {
                        h.watermark_x = -1.0;
                        h.watermark_y = -1.0;
                        changed = true;
                    }
                }

                changed |= ui.checkbox("Debug", &mut h.debug);
            });

        ui.same_line();

        ui.child_window("##hud_right")
            .size([col_w, 0.0])
            .build(|| {
                let mut font_size = h.font_size;
                if ui.slider_config("Font Size", 1.0_f32, 99.0_f32)
                    .display_format("%.1f")
                    .build(&mut font_size)
                {
                    h.font_size = font_size;
                    changed = true;
                }

                let mut icon_size = h.icon_size;
                if ui.slider_config("Icon Size", 1.0_f32, 99.0_f32)
                    .display_format("%.1f")
                    .build(&mut icon_size)
                {
                    h.icon_size = icon_size;
                    changed = true;
                }

                let mut gui_font = h.gui_font_size;
                if ui.slider_config("GUI Font Size", 8.0_f32, 24.0_f32)
                    .display_format("%.1f")
                    .build(&mut gui_font)
                {
                    h.gui_font_size = gui_font;
                    changed = true;
                }

                let mut lw = h.line_width;
                if ui.slider_config("Line Width", 0.1_f32, 8.0_f32)
                    .display_format("%.1f")
                    .build(&mut lw)
                {
                    h.line_width = lw;
                    changed = true;
                }

                let mut arrow_size = h.fov_arrow_size;
                if ui.slider_config("Arrow Size", 0.0_f32, 50.0_f32)
                    .display_format("%.1f")
                    .build(&mut arrow_size)
                {
                    h.fov_arrow_size = arrow_size;
                    changed = true;
                }

                let mut arrow_margin = h.fov_arrow_margin;
                if ui.slider_config("Arrow Margin", 0.0_f32, 500.0_f32)
                    .display_format("%.0f")
                    .build(&mut arrow_margin)
                {
                    h.fov_arrow_margin = arrow_margin;
                    changed = true;
                }

                let mut overlay_rate = h.overlay_refresh_rate as i32;
                if ui.slider_config("Overlay FPS", 30_i32, 360_i32)
                    .build(&mut overlay_rate)
                {
                    h.overlay_refresh_rate = overlay_rate.clamp(30, 360) as u64;
                    changed = true;
                }

                let mut data_rate = h.data_refresh_rate as i32;
                if ui.slider_config("Data FPS", 20_i32, 240_i32)
                    .build(&mut data_rate)
                {
                    h.data_refresh_rate = data_rate.clamp(20, 240) as u64;
                    changed = true;
                }

                // Text outline + inline text color
                changed |= color_edit(ui, "##text_col", &mut h.text_color);
                ui.same_line();
                changed |= ui.checkbox("Text Outline", &mut h.text_outline);
            });

        changed
    }
}

impl Default for HudPanel {
    fn default() -> Self {
        Self::new()
    }
}
