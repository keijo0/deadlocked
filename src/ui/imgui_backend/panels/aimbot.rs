//! Aimbot, RCS, and Triggerbot settings panel.
use std::sync::Arc;
use imgui::Key;
use strum::IntoEnumIterator as _;
use utils::log;

use crate::{
    config::{Config, KeyMode, WeaponConfig},
    cs2::{bones::Bones, entity::weapon::Weapon, key_codes::KeyCode},
    data::Data,
    ui::color::ColorScheme,
};
use super::c32_to_f4;

pub struct AimbotPanel {
    /// Whether the per-weapon override mode is active.
    weapon_tab: bool,
    /// Which weapon is selected for per-weapon config.
    current_weapon: Weapon,
    /// ID of the keybind currently being recorded, e.g. "aimbot_hotkey".
    listening_id: Option<String>,
}

impl AimbotPanel {
    pub fn new() -> Self {
        Self {
            weapon_tab: false,
            current_weapon: Weapon::Ak47,
            listening_id: None,
        }
    }

    /// Render the panel. Returns `true` when any setting changed.
    pub fn render(&mut self, ui: &imgui::Ui, config: &mut Config, data: &Arc<utils::sync::Mutex<Data>>) -> bool {
        let mut changed = false;

        // ── Weapon selector row ───────────────────────────────────────────
        changed |= ui.checkbox("Per-Weapon Override", &mut self.weapon_tab);

        if self.weapon_tab {
            let weapons: Vec<Weapon> = Weapon::iter().collect();
            let weapon_strings: Vec<String> =
                weapons.iter().map(|w| format!("{:?}", w)).collect();
            let weapon_strs: Vec<&str> =
                weapon_strings.iter().map(|s| s.as_str()).collect();
            let mut weapon_idx =
                weapons.iter().position(|w| *w == self.current_weapon).unwrap_or(0);
            if ui.combo_simple_string("Weapon##weapon_sel", &mut weapon_idx, &weapon_strs) {
                self.current_weapon = weapons[weapon_idx].clone();
            }
            ui.same_line();
            if ui.button("Current##weapon_cur") {
                self.current_weapon = data.lock().weapon.clone();
            }
            ui.same_line();
            let cur_weapon_name = format!("({:?})", data.lock().weapon);
            ui.text_disabled(cur_weapon_name);
        }

        ui.separator();

        // ── Two-column layout: aimbot left | triggerbot+rcs right ─────────
        let avail = ui.content_region_avail()[0];
        let col_w = (avail - 8.0) / 2.0;

        // ─── Left column: Aimbot ──────────────────────────────────────────
        ui.child_window("##ab_left")
            .size([col_w, 0.0])
            .build(|| {
                ui.text("Aimbot");
                ui.separator();

                // Aimbot override toggle (weapon mode only)
                if self.weapon_tab {
                    let wcfg = config.aim.weapons.get_mut(&self.current_weapon).unwrap();
                    if ui.checkbox("Enable Override##ab_ovr", &mut wcfg.aimbot.enable_override) {
                        changed = true;
                    }
                }

                {
                    let wcfg = Self::wcfg(config, self.weapon_tab, &self.current_weapon);
                    if ui.checkbox("Enable Aimbot", &mut wcfg.aimbot.enabled) {
                        changed = true;
                    }
                }

                // Hotkeys — right after the enable toggle
                {
                    // Ensure at least two hotkey slots exist.
                    while config.aim.aimbot_hotkeys.len() < 2 {
                        config.aim.aimbot_hotkeys.push(*config.aim.aimbot_hotkeys.first().unwrap_or(&KeyCode::Mouse5));
                    }
                    let [hk0, hk1, ..] = config.aim.aimbot_hotkeys.as_mut_slice() else { unreachable!() };
                    if keybind_button(ui, "Aimbot Key 1", "aimbot_hk0", hk0, &mut self.listening_id) {
                        changed = true;
                    }
                    if keybind_button(ui, "Aimbot Key 2", "aimbot_hk1", hk1, &mut self.listening_id) {
                        changed = true;
                    }
                }

                {
                    let wcfg = Self::wcfg(config, self.weapon_tab, &self.current_weapon);

                    let modes = ["Hold", "Toggle"];
                    let mut mode_idx = key_mode_idx(&wcfg.aimbot.mode);
                    if ui.combo_simple_string("Mode##ab", &mut mode_idx, &modes) {
                        wcfg.aimbot.mode = idx_to_key_mode(mode_idx);
                        changed = true;
                    }

                    let mut fov = wcfg.aimbot.fov;
                    if ui.slider_config("FOV", 0.1_f32, 360.0_f32)
                        .display_format("%.1f°")
                        .build(&mut fov)
                    {
                        wcfg.aimbot.fov = fov;
                        changed = true;
                    }

                    let mut smooth = wcfg.aimbot.smooth;
                    if ui.slider_config("Smooth", 0.0_f32, 20.0_f32)
                        .display_format("%.1f")
                        .build(&mut smooth)
                    {
                        wcfg.aimbot.smooth = smooth;
                        changed = true;
                    }

                    let mut start_bullet = wcfg.aimbot.start_bullet;
                    if ui.slider_config("Start Bullet", 0_i32, 10_i32)
                        .build(&mut start_bullet)
                    {
                        wcfg.aimbot.start_bullet = start_bullet;
                        changed = true;
                    }

                    if ui.checkbox("Visibility Check", &mut wcfg.aimbot.visibility_check) {
                        changed = true;
                    }
                    if ui.checkbox("Wall / Smoke Check", &mut wcfg.aimbot.smoke_wall_check) {
                        changed = true;
                    }
                    if ui.checkbox("Target Friendlies", &mut wcfg.aimbot.target_friendlies) {
                        changed = true;
                    }
                    if ui.checkbox("Distance-Adjusted FOV", &mut wcfg.aimbot.distance_adjusted_fov)
                    {
                        changed = true;
                    }

                    if wcfg.aimbot.distance_adjusted_fov {
                        let mut fov_min_dist = wcfg.aimbot.fov_min_distance;
                        if ui.slider_config("  Min Dist", 0.0_f32, 3000.0_f32)
                            .display_format("%.0f u")
                            .build(&mut fov_min_dist)
                        {
                            wcfg.aimbot.fov_min_distance = fov_min_dist;
                            changed = true;
                        }
                        let mut fov_max_dist = wcfg.aimbot.fov_max_distance;
                        if ui.slider_config("  Max Dist", 0.0_f32, 3000.0_f32)
                            .display_format("%.0f u")
                            .build(&mut fov_max_dist)
                        {
                            wcfg.aimbot.fov_max_distance = fov_max_dist;
                            changed = true;
                        }
                        let mut fov_at_min = wcfg.aimbot.fov_at_min_distance;
                        if ui.slider_config("  FOV @ Min", 0.1_f32, 360.0_f32)
                            .display_format("%.1f")
                            .build(&mut fov_at_min)
                        {
                            wcfg.aimbot.fov_at_min_distance = fov_at_min;
                            changed = true;
                        }
                        let mut fov_at_max = wcfg.aimbot.fov_at_max_distance;
                        if ui.slider_config("  FOV @ Max", 0.1_f32, 360.0_f32)
                            .display_format("%.1f")
                            .build(&mut fov_at_max)
                        {
                            wcfg.aimbot.fov_at_max_distance = fov_at_max;
                            changed = true;
                        }
                    }
                }

                // ── Bone selector ─────────────────────────────────────────
                ui.separator();
                ui.text("Bones:");
                ui.spacing();

                let scheme = ColorScheme::for_style(&config.accent_style);
                let accent   = c32_to_f4(scheme.accent);
                let accent_b = c32_to_f4(scheme.accent_bright);

                let wcfg = Self::wcfg(config, self.weapon_tab, &self.current_weapon);

                // Single bones
                for bone in [Bones::Head, Bones::Neck, Bones::Hip] {
                    let label = format!("{:?}", bone);
                    let active = wcfg.aimbot.bones.contains(&bone);
                    if toggle_btn(ui, &label, active, accent, accent_b) {
                        if active {
                            wcfg.aimbot.bones.retain(|b| *b != bone);
                        } else {
                            wcfg.aimbot.bones.push(bone);
                        }
                        changed = true;
                    }
                    ui.same_line();
                }
                ui.new_line();

                // Spine group
                {
                    let spine_bones =
                        [Bones::Spine1, Bones::Spine2, Bones::Spine3, Bones::Spine4];
                    let active = spine_bones.iter().any(|b| wcfg.aimbot.bones.contains(b));
                    if toggle_btn(ui, "Spine", active, accent, accent_b) {
                        if active {
                            wcfg.aimbot.bones.retain(|b| !spine_bones.contains(b));
                        } else {
                            for b in spine_bones {
                                if !wcfg.aimbot.bones.contains(&b) {
                                    wcfg.aimbot.bones.push(b);
                                }
                            }
                        }
                        changed = true;
                    }
                    ui.same_line();
                }

                // Merged L/R pairs
                for (label, left, right) in [
                    ("Shoulder", Bones::LeftShoulder, Bones::RightShoulder),
                    ("Elbow", Bones::LeftElbow, Bones::RightElbow),
                    ("Hand", Bones::LeftHand, Bones::RightHand),
                    ("Knee", Bones::LeftKnee, Bones::RightKnee),
                    ("Foot", Bones::LeftFoot, Bones::RightFoot),
                ] {
                    let active = wcfg.aimbot.bones.contains(&left)
                        || wcfg.aimbot.bones.contains(&right);
                    if toggle_btn(ui, label, active, accent, accent_b) {
                        if active {
                            wcfg.aimbot.bones.retain(|b| *b != left && *b != right);
                        } else {
                            if !wcfg.aimbot.bones.contains(&left) {
                                wcfg.aimbot.bones.push(left);
                            }
                            if !wcfg.aimbot.bones.contains(&right) {
                                wcfg.aimbot.bones.push(right);
                            }
                        }
                        changed = true;
                    }
                    ui.same_line();
                }
                ui.new_line();
            });

        ui.same_line();

        // ─── Right column: Triggerbot + RCS ──────────────────────────────
        ui.child_window("##ab_right")
            .size([col_w, 0.0])
            .build(|| {
                // ── Triggerbot ────────────────────────────────────────────
                ui.text("Triggerbot");
                ui.separator();

                if self.weapon_tab {
                    let wcfg = config.aim.weapons.get_mut(&self.current_weapon).unwrap();
                    if ui.checkbox("Enable Override##tb_ovr", &mut wcfg.triggerbot.enable_override)
                    {
                        changed = true;
                    }
                }

                {
                    let wcfg = Self::wcfg(config, self.weapon_tab, &self.current_weapon);

                    if ui.checkbox("Enable Triggerbot", &mut wcfg.triggerbot.enabled) {
                        changed = true;
                    }
                }

                // Hotkey — right after the enable toggle
                if keybind_button(
                    ui,
                    "Trigger Key",
                    "tb_hotkey",
                    &mut config.aim.triggerbot_hotkey,
                    &mut self.listening_id,
                ) {
                    changed = true;
                }

                {
                    let wcfg = Self::wcfg(config, self.weapon_tab, &self.current_weapon);

                    let modes = ["Hold", "Toggle"];
                    let mut mode_idx = key_mode_idx(&wcfg.triggerbot.mode);
                    if ui.combo_simple_string("Mode##tb", &mut mode_idx, &modes) {
                        wcfg.triggerbot.mode = idx_to_key_mode(mode_idx);
                        changed = true;
                    }

                    // Delay range
                    let delay_start = *wcfg.triggerbot.delay.start();
                    let delay_end = *wcfg.triggerbot.delay.end();
                    let mut delay_min = delay_start as i32;
                    let mut delay_max = delay_end as i32;
                    let mut delay_changed = false;
                    delay_changed |= ui
                        .slider_config("Delay Min (ms)", 0_i32, 999_i32)
                        .build(&mut delay_min);
                    delay_changed |= ui
                        .slider_config("Delay Max (ms)", 0_i32, 999_i32)
                        .build(&mut delay_max);
                    if delay_changed {
                        wcfg.triggerbot.delay =
                            (delay_min.max(0) as u64)..=(delay_max.max(delay_min) as u64);
                        changed = true;
                    }

                    let mut shot_dur = wcfg.triggerbot.shot_duration as i32;
                    if ui.slider_config("Shot Duration (ms)", 0_i32, 2000_i32)
                        .build(&mut shot_dur)
                    {
                        wcfg.triggerbot.shot_duration = shot_dur.max(0) as u64;
                        changed = true;
                    }

                    if ui.checkbox("Head Only", &mut wcfg.triggerbot.head_only) {
                        changed = true;
                    }
                    if ui.checkbox("Flash Check", &mut wcfg.triggerbot.flash_check) {
                        changed = true;
                    }
                    if ui.checkbox("Scope Check", &mut wcfg.triggerbot.scope_check) {
                        changed = true;
                    }
                    if ui.checkbox("Velocity Check", &mut wcfg.triggerbot.velocity_check) {
                        changed = true;
                    }
                    if wcfg.triggerbot.velocity_check {
                        let mut vt = wcfg.triggerbot.velocity_threshold;
                        if ui.slider_config("  Velocity Threshold", 0.0_f32, 5000.0_f32)
                            .build(&mut vt)
                        {
                            wcfg.triggerbot.velocity_threshold = vt;
                            changed = true;
                        }
                    }
                }

                ui.separator();

                // Autowall (global)
                {
                    if ui.checkbox("Autowall", &mut config.aim.autowall_enabled) {
                        changed = true;
                    }
                    if config.aim.autowall_enabled {
                        if ui.checkbox("  Safe Mode", &mut config.aim.autowall_safe) {
                            changed = true;
                        }
                        let modes = ["Hold", "Toggle"];
                        let mut aw_mode = key_mode_idx(&config.aim.autowall_mode);
                        if ui.combo_simple_string("  AW Mode", &mut aw_mode, &modes) {
                            config.aim.autowall_mode = idx_to_key_mode(aw_mode);
                            changed = true;
                        }
                        if keybind_button(
                            ui,
                            "  AW Key",
                            "aw_hotkey",
                            &mut config.aim.autowall_hotkey,
                            &mut self.listening_id,
                        ) {
                            changed = true;
                        }
                    }
                }

                // ── Magnet Trigger ────────────────────────────────────────
                ui.separator();
                {
                    let wcfg = Self::wcfg(config, self.weapon_tab, &self.current_weapon);
                    if ui.checkbox("Magnet Trigger", &mut wcfg.triggerbot.magnet_enabled) {
                        changed = true;
                    }
                    if wcfg.triggerbot.magnet_enabled {
                        let mut strength = wcfg.triggerbot.magnet_strength;
                        if ui.slider_config("  Strength", 0.0_f32, 0.50_f32)
                            .display_format("%.2f")
                            .build(&mut strength)
                        {
                            wcfg.triggerbot.magnet_strength = strength;
                            changed = true;
                        }
                        let mut fov = wcfg.triggerbot.magnet_fov;
                        if ui.slider_config("  FOV", 2.0_f32, 25.0_f32)
                            .display_format("%.1f")
                            .build(&mut fov)
                        {
                            wcfg.triggerbot.magnet_fov = fov;
                            changed = true;
                        }
                        let mut smoothing = wcfg.triggerbot.magnet_smoothing;
                        if ui.slider_config("  Smoothing", 0.1_f32, 1.0_f32)
                            .display_format("%.2f")
                            .build(&mut smoothing)
                        {
                            wcfg.triggerbot.magnet_smoothing = smoothing;
                            changed = true;
                        }
                        let mut vscale = wcfg.triggerbot.magnet_vertical_scale;
                        if ui.slider_config("  Vertical Scale", 0.1_f32, 1.0_f32)
                            .display_format("%.2f")
                            .build(&mut vscale)
                        {
                            wcfg.triggerbot.magnet_vertical_scale = vscale;
                            changed = true;
                        }
                    }
                }

                // Advanced Magnet (Low FOV Ragebot)
                ui.separator();
                {
                    let wcfg = Self::wcfg(config, self.weapon_tab, &self.current_weapon);
                    if ui.checkbox("Advanced Magnet (Ragebot)", &mut wcfg.triggerbot.advanced_magnet.enabled) {
                        changed = true;
                    }
                    if wcfg.triggerbot.advanced_magnet.enabled {
                        let modes = ["Hold", "Toggle"];
                        let mut mode_idx = key_mode_idx(&wcfg.triggerbot.advanced_magnet.mode);
                        if ui.combo_simple_string("  Mode##adv", &mut mode_idx, &modes) {
                            wcfg.triggerbot.advanced_magnet.mode = idx_to_key_mode(mode_idx);
                            changed = true;
                        }

                        let mut fov = wcfg.triggerbot.advanced_magnet.fov;
                        if ui.slider_config("  FOV (Low FOV)", 0.5_f32, 10.0_f32)
                            .display_format("%.1f")
                            .build(&mut fov)
                        {
                            wcfg.triggerbot.advanced_magnet.fov = fov;
                            changed = true;
                        }

                        let mut max_dist = wcfg.triggerbot.advanced_magnet.max_distance;
                        if ui.slider_config("  Max Distance", 100.0_f32, 2000.0_f32)
                            .display_format("%.0f")
                            .build(&mut max_dist)
                        {
                            wcfg.triggerbot.advanced_magnet.max_distance = max_dist;
                            changed = true;
                        }

                        let mut aggression = wcfg.triggerbot.advanced_magnet.aggression;
                        if ui.slider_config("  Aggression", 0.0_f32, 1.0_f32)
                            .display_format("%.2f")
                            .build(&mut aggression)
                        {
                            wcfg.triggerbot.advanced_magnet.aggression = aggression;
                            changed = true;
                        }

                        if ui.checkbox("  Prediction", &mut wcfg.triggerbot.advanced_magnet.prediction) {
                            changed = true;
                        }
                        if wcfg.triggerbot.advanced_magnet.prediction {
                            let mut pred_time = wcfg.triggerbot.advanced_magnet.prediction_time;
                            if ui.slider_config("    Prediction Time", 0.01_f32, 0.5_f32)
                                .display_format("%.2f")
                                .build(&mut pred_time)
                            {
                                wcfg.triggerbot.advanced_magnet.prediction_time = pred_time;
                                changed = true;
                            }
                        }

                        if ui.checkbox("  Damage-Based Bones", &mut wcfg.triggerbot.advanced_magnet.damage_based_bones) {
                            changed = true;
                        }

                        let mut hit_chance = wcfg.triggerbot.advanced_magnet.hit_chance_threshold;
                        if ui.slider_config("  Hit Chance Threshold", 0.1_f32, 1.0_f32)
                            .display_format("%.2f")
                            .build(&mut hit_chance)
                        {
                            wcfg.triggerbot.advanced_magnet.hit_chance_threshold = hit_chance;
                            changed = true;
                        }

                        if ui.checkbox("  Threat Scoring", &mut wcfg.triggerbot.advanced_magnet.threat_scoring) {
                            changed = true;
                        }

                        if ui.checkbox("  Instant Snap", &mut wcfg.triggerbot.advanced_magnet.instant_snap) {
                            changed = true;
                        }

                        let mut smooth_factor = wcfg.triggerbot.advanced_magnet.smooth_factor;
                        if ui.slider_config("  Smooth Factor", 0.1_f32, 1.0_f32)
                            .display_format("%.2f")
                            .build(&mut smooth_factor)
                        {
                            wcfg.triggerbot.advanced_magnet.smooth_factor = smooth_factor;
                            changed = true;
                        }

                        let mut vert_mult = wcfg.triggerbot.advanced_magnet.vertical_multiplier;
                        if ui.slider_config("  Vertical Multiplier", 0.5_f32, 2.0_f32)
                            .display_format("%.2f")
                            .build(&mut vert_mult)
                        {
                            wcfg.triggerbot.advanced_magnet.vertical_multiplier = vert_mult;
                            changed = true;
                        }

                        let mut min_damage = wcfg.triggerbot.advanced_magnet.min_damage_threshold;
                        if ui.slider_config("  Min Damage", 1.0_f32, 100.0_f32)
                            .display_format("%.0f")
                            .build(&mut min_damage)
                        {
                            wcfg.triggerbot.advanced_magnet.min_damage_threshold = min_damage;
                            changed = true;
                        }

                        let mut wall_bonus = wcfg.triggerbot.advanced_magnet.wall_penetration_bonus;
                        if ui.slider_config("  Wall Bonus", 1.0_f32, 3.0_f32)
                            .display_format("%.2f")
                            .build(&mut wall_bonus)
                        {
                            wcfg.triggerbot.advanced_magnet.wall_penetration_bonus = wall_bonus;
                            changed = true;
                        }
                    }
                }

                // ── RCS ───────────────────────────────────────────────────
                ui.separator();
                ui.text("RCS");
                ui.separator();

                if self.weapon_tab {
                    let wcfg = config.aim.weapons.get_mut(&self.current_weapon).unwrap();
                    if ui.checkbox("Enable Override##rcs_ovr", &mut wcfg.rcs.enable_override) {
                        changed = true;
                    }
                }

                {
                    let wcfg = Self::wcfg(config, self.weapon_tab, &self.current_weapon);
                    if ui.checkbox("Enable RCS", &mut wcfg.rcs.enabled) {
                        changed = true;
                    }
                    let mut smooth = wcfg.rcs.smooth;
                    if ui.slider_config("RCS Smooth", 0.0_f32, 10.0_f32)
                        .display_format("%.2f")
                        .build(&mut smooth)
                    {
                        wcfg.rcs.smooth = smooth;
                        changed = true;
                    }
                }
            });

        changed
    }

    fn wcfg<'a>(
        config: &'a mut Config,
        weapon_tab: bool,
        current_weapon: &Weapon,
    ) -> &'a mut WeaponConfig {
        if weapon_tab {
            config.aim.weapons.get_mut(current_weapon).unwrap()
        } else {
            &mut config.aim.global
        }
    }
}

// ── Bone toggle button ────────────────────────────────────────────────────────

/// Small framed button that renders highlighted when `active` and normal otherwise.
fn toggle_btn(ui: &imgui::Ui, label: &str, active: bool, accent: [f32; 4], accent_bright: [f32; 4]) -> bool {
    if active {
        let _c = ui.push_style_color(imgui::StyleColor::Button, accent);
        let _c2 = ui.push_style_color(imgui::StyleColor::ButtonHovered, accent_bright);
        ui.button(label)
    } else {
        ui.button(label)
    }
}

// ── Imgui keybind button ──────────────────────────────────────────────────────

/// Renders `[KeyName]  Label`.  Clicking activates recording; the next key or
/// mouse-button press is captured and stored in `keycode`.  Escape cancels.
/// Returns `true` when `keycode` changed.
fn keybind_button(
    ui: &imgui::Ui,
    label: &str,
    id: &str,
    keycode: &mut KeyCode,
    listening_id: &mut Option<String>,
) -> bool {
    let is_listening = listening_id.as_deref() == Some(id);
    
    // Cheat-style key formatting
    let key_text = if is_listening {
        "...".to_string()
    } else {
        format_key_simple(*keycode)
    };
    
    // Simple button styling
    let btn_size = [70.0, 22.0];
    let btn_clicked = ui.button_with_size(&format!("{}##{}", key_text, id), btn_size);
    
    ui.same_line();
    ui.text(label);
    
    if btn_clicked {
        if is_listening {
            *listening_id = None;
        } else {
            *listening_id = Some(id.to_string());
        }
    }

    if !is_listening {
        return false;
    }

    // Cancel on Escape
    if ui.is_key_pressed_no_repeat(Key::Escape) {
        *listening_id = None;
        return false;
    }

    // Mouse buttons
    let mouse_map = [
        (imgui::MouseButton::Left, KeyCode::MouseLeft),
        (imgui::MouseButton::Right, KeyCode::MouseRight),
        (imgui::MouseButton::Middle, KeyCode::MouseMiddle),
        (imgui::MouseButton::Extra1, KeyCode::Mouse4),
        (imgui::MouseButton::Extra2, KeyCode::Mouse5),
    ];
    
    for (btn, kc) in mouse_map {
        if ui.is_mouse_clicked(btn) {
            *keycode = kc;
            *listening_id = None;
            return true;
        }
    }

    // Fallback for Mouse4/Mouse5 — imgui_winit_support may not
    // forward Back/Forward button clicks, so check held state.
    if ui.is_mouse_down(imgui::MouseButton::Extra1) {
        *keycode = KeyCode::Mouse4;
        *listening_id = None;
        return true;
    }
    if ui.is_mouse_down(imgui::MouseButton::Extra2) {
        *keycode = KeyCode::Mouse5;
        *listening_id = None;
        return true;
    }

    // Mouse wheel
    let wheel = ui.io().mouse_wheel;
    if wheel > 0.0 {
        *keycode = KeyCode::MouseWheelUp;
        *listening_id = None;
        return true;
    }
    if wheel < 0.0 {
        *keycode = KeyCode::MouseWheelDown;
        *listening_id = None;
        return true;
    }

    // Keyboard keys
    let key_map: &[(Key, KeyCode)] = &[
        (Key::A, KeyCode::A), (Key::B, KeyCode::B), (Key::C, KeyCode::C),
        (Key::D, KeyCode::D), (Key::E, KeyCode::E), (Key::F, KeyCode::F),
        (Key::G, KeyCode::G), (Key::H, KeyCode::H), (Key::I, KeyCode::I),
        (Key::J, KeyCode::J), (Key::K, KeyCode::K), (Key::L, KeyCode::L),
        (Key::M, KeyCode::M), (Key::N, KeyCode::N), (Key::O, KeyCode::O),
        (Key::P, KeyCode::P), (Key::Q, KeyCode::Q), (Key::R, KeyCode::R),
        (Key::S, KeyCode::S), (Key::T, KeyCode::T), (Key::U, KeyCode::U),
        (Key::V, KeyCode::V), (Key::W, KeyCode::W), (Key::X, KeyCode::X),
        (Key::Y, KeyCode::Y), (Key::Z, KeyCode::Z),
        (Key::Alpha0, KeyCode::Num0), (Key::Alpha1, KeyCode::Num1),
        (Key::Alpha2, KeyCode::Num2), (Key::Alpha3, KeyCode::Num3),
        (Key::Alpha4, KeyCode::Num4), (Key::Alpha5, KeyCode::Num5),
        (Key::Alpha6, KeyCode::Num6), (Key::Alpha7, KeyCode::Num7),
        (Key::Alpha8, KeyCode::Num8), (Key::Alpha9, KeyCode::Num9),
        (Key::Space, KeyCode::Space),
        (Key::Backspace, KeyCode::Backspace),
        (Key::Tab, KeyCode::Tab),
        (Key::Insert, KeyCode::Insert),
        (Key::Delete, KeyCode::Delete),
        (Key::Home, KeyCode::Home),
        (Key::End, KeyCode::End),
        (Key::LeftShift, KeyCode::LeftShift),
        (Key::LeftCtrl, KeyCode::LeftControl),
        (Key::LeftAlt, KeyCode::LeftAlt),
    ];
    for &(imgui_key, kc) in key_map {
        if ui.is_key_pressed_no_repeat(imgui_key) {
            *keycode = kc;
            *listening_id = None;
            return true;
        }
    }

    false
}

fn key_mode_idx(mode: &KeyMode) -> usize {
    match mode {
        KeyMode::Hold => 0,
        KeyMode::Toggle => 1,
    }
}

fn idx_to_key_mode(idx: usize) -> KeyMode {
    match idx {
        1 => KeyMode::Toggle,
        _ => KeyMode::Hold,
    }
}

/// Format keycodes for display
fn format_key_simple(keycode: KeyCode) -> String {
    match keycode {
        KeyCode::Num0 => "---".to_string(),
        KeyCode::Num1 => "1".to_string(),
        KeyCode::Num2 => "2".to_string(),
        KeyCode::Num3 => "3".to_string(),
        KeyCode::Num4 => "4".to_string(),
        KeyCode::Num5 => "5".to_string(),
        KeyCode::Num6 => "6".to_string(),
        KeyCode::Num7 => "7".to_string(),
        KeyCode::Num8 => "8".to_string(),
        KeyCode::Num9 => "9".to_string(),
        KeyCode::A => "A".to_string(),
        KeyCode::B => "B".to_string(),
        KeyCode::C => "C".to_string(),
        KeyCode::D => "D".to_string(),
        KeyCode::E => "E".to_string(),
        KeyCode::F => "F".to_string(),
        KeyCode::G => "G".to_string(),
        KeyCode::H => "H".to_string(),
        KeyCode::I => "I".to_string(),
        KeyCode::J => "J".to_string(),
        KeyCode::K => "K".to_string(),
        KeyCode::L => "L".to_string(),
        KeyCode::M => "M".to_string(),
        KeyCode::N => "N".to_string(),
        KeyCode::O => "O".to_string(),
        KeyCode::P => "P".to_string(),
        KeyCode::Q => "Q".to_string(),
        KeyCode::R => "R".to_string(),
        KeyCode::S => "S".to_string(),
        KeyCode::T => "T".to_string(),
        KeyCode::U => "U".to_string(),
        KeyCode::V => "V".to_string(),
        KeyCode::W => "W".to_string(),
        KeyCode::X => "X".to_string(),
        KeyCode::Y => "Y".to_string(),
        KeyCode::Z => "Z".to_string(),
        KeyCode::Space => "SPACE".to_string(),
        KeyCode::Backspace => "BKSP".to_string(),
        KeyCode::Tab => "TAB".to_string(),
        KeyCode::Escape => "ESC".to_string(),
        KeyCode::Insert => "INS".to_string(),
        KeyCode::Delete => "DEL".to_string(),
        KeyCode::Home => "HOME".to_string(),
        KeyCode::End => "END".to_string(),
        KeyCode::LeftShift => "LSHIFT".to_string(),
        KeyCode::LeftAlt => "LALT".to_string(),
        KeyCode::LeftControl => "LCTRL".to_string(),
        KeyCode::MouseLeft => "LMB".to_string(),
        KeyCode::MouseRight => "RMB".to_string(),
        KeyCode::MouseMiddle => "MMB".to_string(),
        KeyCode::Mouse4 => "MB4".to_string(),
        KeyCode::Mouse5 => "MB5".to_string(),
        KeyCode::MouseWheelUp => "MWUP".to_string(),
        KeyCode::MouseWheelDown => "MWDN".to_string(),
    }
}

impl Default for AimbotPanel {
    fn default() -> Self {
        Self::new()
    }
}
