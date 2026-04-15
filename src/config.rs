use std::{
    collections::HashMap,
    fs::read_to_string,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    sync::LazyLock,
    time::Duration,
};

use egui::Color32;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use utils::log;

use crate::{
    cs2::{bones::Bones, entity::weapon::Weapon, key_codes::KeyCode},
    ui::color::{AccentStyle, Colors},
};

// 15.625ms per iteration = 64 Hz (1000ms / 15.625ms)
pub const LOOP_DURATION: Duration = Duration::from_micros(15_625);
pub const SLEEP_DURATION: Duration = Duration::from_secs(5);
pub const DEFAULT_CONFIG_NAME: &str = "deadlocked.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub aim: AimConfig,
    pub player: PlayerConfig,
    pub hud: HudConfig,
    pub misc: UnsafeConfig,
    pub accent_style: AccentStyle,
    pub parental_lock: bool,
}

/// Hard caps enforced when `parental_lock` is enabled.
pub mod safe_limits {
    pub const FOV_MAX:        f32 = 2.5;
    pub const SMOOTH_MIN:     f32 = 8.0;
    pub const START_BULLET:   i32 = 1;
    pub const DELAY_MIN:      u64 = 80;
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aim: AimConfig::default(),
            player: PlayerConfig::default(),
            hud: HudConfig::default(),
            misc: UnsafeConfig::default(),
            accent_style: AccentStyle::Default,
            parental_lock: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponConfig {
    pub aimbot: AimbotConfig,
    pub rcs: RcsConfig,
    pub triggerbot: TriggerbotConfig,
}

impl WeaponConfig {
    pub fn enabled(enabled: bool) -> Self {
        let aimbot = AimbotConfig {
            enable_override: enabled,
            ..Default::default()
        };
        Self {
            aimbot,
            rcs: RcsConfig::default(),
            triggerbot: TriggerbotConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AimbotConfig {
    pub enable_override: bool,
    pub enabled: bool,
    pub mode: KeyMode,
    pub target_friendlies: bool,
    pub distance_adjusted_fov: bool,
    pub start_bullet: i32,
    pub visibility_check: bool,
    pub smoke_wall_check: bool,
    pub flash_check: bool,
    pub fov: f32,
    pub fov_min_distance: f32,
    pub fov_max_distance: f32,
    pub fov_at_min_distance: f32,
    pub fov_at_max_distance: f32,
    pub smooth: f32,
    pub bones: Vec<Bones>,
    pub humanization: bool,
    pub humanization_amount: f32,
}

impl Default for AimbotConfig {
    fn default() -> Self {
        Self {
            enable_override: false,
            enabled: true,
            mode: KeyMode::Hold,
            target_friendlies: false,
            distance_adjusted_fov: true,
            start_bullet: 0,
            visibility_check: true,
            smoke_wall_check: true,
            flash_check: true,
            fov: 2.5,
            fov_min_distance: 200.0,
            fov_max_distance: 1000.0,
            fov_at_min_distance: 5.0,
            fov_at_max_distance: 1.5,
            smooth: 5.0,
            bones: vec![
                Bones::Head,
                Bones::Neck,
                Bones::Spine4,
                Bones::Spine3,
                Bones::Spine2,
                Bones::Spine1,
                Bones::Hip,
            ],
            humanization: true,
            humanization_amount: 1.11,
        }
    }
}

impl AimbotConfig {
    /// Returns an interpolated FOV value for the given target `distance`.
    ///
    /// * Below `fov_min_distance` the FOV is clamped to `fov_at_min_distance`.
    /// * Above `fov_max_distance` the FOV is clamped to `fov_at_max_distance`.
    /// * Between the two thresholds the FOV is linearly interpolated.
    ///
    /// If `distance_adjusted_fov` is disabled the base `fov` field is returned
    /// unchanged, preserving flat-FOV behaviour for users who have not opted in.
    pub fn calculate_fov(&self, distance: f32) -> f32 {
        if !self.distance_adjusted_fov {
            return self.fov;
        }
        if self.fov_max_distance <= self.fov_min_distance {
            return self.fov;
        }
        let t = ((distance - self.fov_min_distance)
            / (self.fov_max_distance - self.fov_min_distance))
            .clamp(0.0, 1.0);
        self.fov_at_min_distance + t * (self.fov_at_max_distance - self.fov_at_min_distance)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RcsConfig {
    pub enable_override: bool,
    pub enabled: bool,
    pub smooth: f32,
}

impl Default for RcsConfig {
    fn default() -> Self {
        Self {
            enable_override: false,
            enabled: false,
            smooth: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumIter)]
pub enum KeyMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TriggerbotConfig {
    pub enable_override: bool,
    pub enabled: bool,
    pub delay: RangeInclusive<u64>,
    pub shot_duration: u64,
    pub mode: KeyMode,
    pub flash_check: bool,
    pub scope_check: bool,
    pub velocity_check: bool,
    pub velocity_threshold: f32,
    pub head_only: bool,
    pub magnet_enabled: bool,
    pub magnet_strength: f32,
    pub magnet_fov: f32,
    pub magnet_smoothing: f32,
    pub magnet_vertical_scale: f32,
    pub advanced_magnet: AdvancedMagnetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedMagnetConfig {
    pub enabled: bool,
    pub mode: KeyMode,
    pub fov: f32,
    pub max_distance: f32,
    pub aggression: f32,
    pub prediction: bool,
    pub prediction_time: f32,
    pub damage_based_bones: bool,
    pub hit_chance_threshold: f32,
    pub threat_scoring: bool,
    pub instant_snap: bool,
    pub smooth_factor: f32,
    pub vertical_multiplier: f32,
    pub priority_bones: Vec<Bones>,
    pub min_damage_threshold: f32,
    pub wall_penetration_bonus: f32,
}

impl Default for AdvancedMagnetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: KeyMode::Hold,
            fov: 2.0,
            max_distance: 800.0,
            aggression: 0.8,
            prediction: true,
            prediction_time: 0.1,
            damage_based_bones: true,
            hit_chance_threshold: 0.6,
            threat_scoring: true,
            instant_snap: false,
            smooth_factor: 0.3,
            vertical_multiplier: 1.2,
            priority_bones: vec![Bones::Head, Bones::Neck, Bones::Spine3],
            min_damage_threshold: 20.0,
            wall_penetration_bonus: 1.5,
        }
    }
}

impl Default for TriggerbotConfig {
    fn default() -> Self {
        Self {
            enable_override: false,
            enabled: false,
            delay: 100..=200,
            shot_duration: 200,
            mode: KeyMode::Hold,
            flash_check: true,
            scope_check: true,
            velocity_check: true,
            velocity_threshold: 100.0,
            head_only: false,
            magnet_enabled: false,
            magnet_strength: 0.22,
            magnet_fov: 14.0,
            magnet_smoothing: 0.80,
            magnet_vertical_scale: 0.6,
            advanced_magnet: AdvancedMagnetConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AimConfig {
    pub aimbot_hotkeys: Vec<KeyCode>,
    pub triggerbot_hotkey: KeyCode,
    pub autowall_hotkey: KeyCode,
    pub autowall_mode: KeyMode,
    pub autowall_enabled: bool,
    pub autowall_safe: bool,
    pub global: WeaponConfig,
    pub weapons: HashMap<Weapon, WeaponConfig>,
}

impl Default for AimConfig {
    fn default() -> Self {
        let mut weapons = HashMap::new();
        for weapon in Weapon::iter() {
            weapons.insert(weapon, WeaponConfig::default());
        }

        Self {
            aimbot_hotkeys: vec![KeyCode::Mouse5],
            triggerbot_hotkey: KeyCode::Mouse4,
            autowall_hotkey: KeyCode::Mouse4,
            autowall_mode: KeyMode::Toggle,
            autowall_enabled: false,
            autowall_safe: false,
            global: WeaponConfig::enabled(true),
            weapons,
        }
    }
}

#[derive(Debug, Clone, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum DrawMode {
    None,
    Health,
    Color,
}

#[derive(Debug, Clone, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum BoxMode {
    Gap,
    Full,
}

#[derive(Debug, Clone, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum WeaponBoxMode {
    Full,
    Gap,
}

#[derive(Debug, Clone, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum SpectatorListStyle {
    Simple,
    New,
    Interium,
}

#[derive(Debug, Clone, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum KeybindListStyle {
    Simple,
    New,
    Interium,
}

impl Default for KeybindListStyle {
    fn default() -> Self {
        Self::New
    }
}

#[derive(Debug, Clone, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum MediaInfoStyle {
    Simple,
    New,
    Interium,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerConfig {
    pub enabled: bool,
    pub esp_hotkey: KeyCode,
    pub show_friendlies: bool,
    pub draw_box: DrawMode,
    pub box_mode: BoxMode,
    pub box_visible_color: Color32,
    pub box_invisible_color: Color32,
    pub draw_skeleton: DrawMode,
    pub skeleton_color: Color32,
    pub health_bar: bool,
    pub health_text: bool,
    pub armor_text: bool,
    pub player_name: bool,
    pub weapon_icon: bool,
    pub tags: bool,
    pub visible_only: bool,
    pub backtrack_visual: bool,
    pub backtrack_in_esp: bool,
    pub backtrack_ms: u32,
    pub backtrack_color: Color32,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            esp_hotkey: KeyCode::X,
            show_friendlies: false,
            draw_box: DrawMode::Color,
            box_mode: BoxMode::Gap,
            box_visible_color: Color32::WHITE,
            box_invisible_color: Color32::RED,
            draw_skeleton: DrawMode::Health,
            skeleton_color: Color32::WHITE,
            health_bar: true,
            health_text: true,
            armor_text: true,
            player_name: true,
            weapon_icon: true,
            tags: true,
            visible_only: false,
            backtrack_visual: false,
            backtrack_in_esp: true,
            backtrack_ms: 187,
            backtrack_color: Color32::from_rgba_unmultiplied(255, 165, 0, 180),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HudConfig {
    pub bomb_timer: bool,
    pub spectator_list: bool,
    pub spectator_list_style: SpectatorListStyle,
    pub spectator_list_x: f32,
    pub spectator_list_y: f32,
    pub spectator_list_scale: f32,
    pub sniper_crosshair: bool,
    pub sniper_crosshair_override: bool,
    pub crosshair_color: Color32,
    pub sniper_crosshair_h: f32,
    pub sniper_crosshair_v: f32,
    pub sniper_crosshair_gap: f32,
    pub sniper_crosshair_thickness: f32,
    pub dropped_weapons: bool,
    pub weapon_box: bool,
    pub weapon_box_mode: WeaponBoxMode,
    pub weapon_box_max_distance: f32,

    pub weapon_esp_max_distance: f32,
    pub keybind_list: bool,
    pub keybind_list_style: KeybindListStyle,
    pub keybind_list_x: f32,
    pub keybind_list_y: f32,
    pub keybind_list_scale: f32,
    pub keybind_list_simple_backdrop: bool,
    pub keybind_aimbot: bool,
    pub keybind_fov: bool,
    pub keybind_trigger_delay: bool,
    pub keybind_triggerbot: bool,
    pub keybind_autowall: bool,

    pub keybind_trigger_activate: bool,
    pub keybind_trigger_active_indicator: bool,
    pub keybind_esp: bool,
    pub keybind_server_picker: bool,
    pub keybind_backtrack: bool,
    pub keybind_ping: bool,
    pub grenade_trails: bool,
    pub fov_arrow_size: f32,
    pub fov_arrows: bool,
    pub fov_arrow_color: Color32,
    pub fov_arrow_margin: f32,
    pub spectator_list_color: Color32,
    pub spectator_list_simple_backdrop: bool,
    pub text_outline: bool,
    pub text_color: Color32,
    pub line_width: f32,
    pub font_size: f32,
    pub icon_size: f32,
    pub gui_font_size: f32,
    pub overlay_refresh_rate: u64,
    pub data_refresh_rate: u64,
    pub debug: bool,
    pub weapon_esp_use_colors: bool,
    pub weapon_esp_background_opacity: u8,
    pub penetration_crosshair_enabled: bool,
    pub penetration_color_yes: Color32,
    pub penetration_color_no: Color32,
    pub penetration_color_unavailable: Color32,
    pub media_info: bool,
    pub media_info_x: f32,
    pub media_info_y: f32,
    pub media_info_color: Color32,
    pub media_info_scale: f32,
    pub media_info_style: MediaInfoStyle,
    pub media_info_simple_backdrop: bool,
    pub watermark: bool,
    pub watermark_weather: bool,
    pub watermark_x: f32,
    pub watermark_y: f32,
    pub watermark_color: Color32,
    pub watermark_scale: f32,
}

impl Default for HudConfig {
    fn default() -> Self {
        Self {
            bomb_timer: true,
            spectator_list: false,
            spectator_list_style: SpectatorListStyle::New,
            spectator_list_x: -1.0,
            spectator_list_y: -1.0,
            spectator_list_scale: 1.0,
            sniper_crosshair: true,
            sniper_crosshair_override: false,
            crosshair_color: Color32::WHITE,
            sniper_crosshair_h: 10.0,
            sniper_crosshair_v: 10.0,
            sniper_crosshair_gap: 4.0,
            sniper_crosshair_thickness: 2.0,
            dropped_weapons: true,
            weapon_box: false,
            weapon_box_mode: WeaponBoxMode::Full,
            weapon_box_max_distance: 0.0,

            weapon_esp_max_distance: 2500.0,
            keybind_list: false,
            keybind_list_style: KeybindListStyle::New,
            keybind_list_x: -1.0,
            keybind_list_y: -1.0,
            keybind_list_scale: 1.0,
            keybind_list_simple_backdrop: false,
            keybind_aimbot: true,
            keybind_fov: true,
            keybind_trigger_delay: true,
            keybind_triggerbot: true,
            keybind_autowall: true,

            keybind_trigger_activate: false,
            keybind_trigger_active_indicator: true,
            keybind_esp: true,
            keybind_server_picker: true,
            keybind_backtrack: true,
            keybind_ping: true,
            grenade_trails: true,
            fov_arrow_size: 15.0,
            fov_arrows: true,
            fov_arrow_color: Color32::WHITE,
            fov_arrow_margin: 50.0,
            spectator_list_color: Color32::from_rgb(132, 125, 209),
            spectator_list_simple_backdrop: false,
            text_outline: true,
            text_color: Colors::TEXT,
            line_width: 2.0,
            font_size: 14.0,
            icon_size: 20.0,
            gui_font_size: 13.0,
            overlay_refresh_rate: 280,
            data_refresh_rate: 280,
            debug: false,
            weapon_esp_use_colors: true,
            weapon_esp_background_opacity: 130,
            penetration_crosshair_enabled: false,
            penetration_color_yes: Color32::from_rgb(0, 200, 0),
            penetration_color_no: Color32::from_rgb(220, 0, 0),
            penetration_color_unavailable: Color32::GRAY,
            media_info: false,
            media_info_x: -1.0,
            media_info_y: -1.0,
            media_info_color: Color32::from_rgba_unmultiplied(200, 200, 200, 180),
            media_info_scale: 1.0,
            media_info_style: MediaInfoStyle::New,
            media_info_simple_backdrop: false,
            watermark: false,
            watermark_weather: false,
            watermark_x: -1.0,
            watermark_y: -1.0,
            watermark_color: Color32::from_rgba_unmultiplied(200, 200, 200, 200),
            watermark_scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AntiAfk {
    pub enabled: bool,
    pub interval_min: f32,
    pub interval_max: f32,
}

impl Default for AntiAfk {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_min: 5.0,
            interval_max: 10.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UnsafeConfig {
    pub antiafk: AntiAfk,
    pub no_flash: bool,
    pub max_flash_alpha: f32,
    pub no_smoke: bool,
}

impl Default for UnsafeConfig {
    fn default() -> Self {
        Self {
            antiafk: AntiAfk::default(),
            no_flash: false,
            max_flash_alpha: 127.0,
            no_smoke: false,
        }
    }
}

pub static BASE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = std::env::var_os("XDG_CONFIG_HOME")
        .and_then(|p| {
            if p.is_empty() {
                None
            } else {
                Some(PathBuf::from(p))
            }
        })
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|base| base.join("deadlocked"))
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
        });
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path
});

pub static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = BASE_PATH.join("configs");
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path
});

pub fn parse_config(path: &Path) -> Config {
    if !path.exists() || path.is_dir() {
        return Config::default();
    }

    let Ok(config_string) = read_to_string(path) else {
        return Config::default();
    };

    let config = toml::from_str(&config_string);
    if config.is_err() {
        log::warn!("config file invalid");
    } else if let Some(file_name) = path.file_name() {
        log::info!("loaded config {:?}", file_name);
    }
    config.unwrap_or_default()
}

pub fn write_config(config: &Config, path: &Path) {
    let out = toml::to_string(&config).unwrap();
    let _ = std::fs::write(path, out);
}

pub fn delete_config(path: &Path) {
    if !path.exists() {
        return;
    }

    if std::fs::remove_file(path).is_ok()
        && let Some(file_name) = path.file_name()
    {
        log::info!("deleted config {:?}", file_name);
    }
}

pub fn available_configs() -> Vec<PathBuf> {
    let mut files = Vec::with_capacity(8);
    let Ok(dir) = std::fs::read_dir::<&Path>(CONFIG_PATH.as_ref()) else {
        return files;
    };

    for path in dir {
        let Ok(file) = path else {
            continue;
        };
        let Ok(file_type) = file.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let file_name = file.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if !file_name.ends_with(".toml") {
            continue;
        }
        files.push(file.path())
    }
    if files.is_empty() {
        let path = CONFIG_PATH.join(DEFAULT_CONFIG_NAME);
        write_config(&Config::default(), &path);
        files.push(path);
    }
    files
}

pub static APP_STATE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| BASE_PATH.join("app_state.toml"));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub ui_scale: f32,
    pub menu_width: f32,
    pub menu_height: f32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            menu_width: 900.0,
            menu_height: 700.0,
        }
    }
}

pub fn read_app_state() -> AppState {
    let Ok(s) = std::fs::read_to_string(APP_STATE_PATH.as_path()) else {
        return AppState::default();
    };
    toml::from_str(&s).unwrap_or_default()
}

pub fn write_app_state(state: &AppState) {
    if let Ok(s) = toml::to_string(state) {
        let _ = std::fs::write(APP_STATE_PATH.as_path(), s);
    }
}
