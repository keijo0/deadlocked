//! Config / profile management panel.
use strum::IntoEnumIterator as _;

use crate::{
    config::{
        AppState, BASE_PATH, CONFIG_PATH, available_configs, delete_config, parse_config,
        read_app_state, write_app_state, write_config, Config, DEFAULT_CONFIG_NAME,
    },
    ui::color::AccentStyle,
};

pub struct ConfigPanel {
    new_config_name: String,
    available_configs: Vec<std::path::PathBuf>,
    pub current_config: std::path::PathBuf,
    ui_scale: f32,
    pub menu_width: f32,
    pub menu_height: f32,
    /// Set to `true` when the user commits a W/H change via the Config
    /// tab.  Consumed by `gui/mod.rs` to call `request_inner_size` exactly
    /// once, then cleared.  Must NOT be set from the `Resized` event path or
    /// it would create a resize feedback loop.
    pub resize_requested: bool,
}

impl ConfigPanel {
    pub fn new() -> Self {
        let current = CONFIG_PATH.join(DEFAULT_CONFIG_NAME);
        let app_state = read_app_state();
        Self {
            new_config_name: String::new(),
            available_configs: available_configs(),
            current_config: current,
            ui_scale: app_state.ui_scale,
            menu_width: app_state.menu_width,
            menu_height: app_state.menu_height,
            resize_requested: false,
        }
    }

    /// Returns the currently configured UI scale.
    pub fn ui_scale(&self) -> f32 {
        self.ui_scale
    }

    /// Save menu geometry to disk.
    pub(crate) fn persist_app_state(&self) {
        write_app_state(&AppState {
            ui_scale: self.ui_scale,
            menu_width: self.menu_width,
            menu_height: self.menu_height,
        });
    }

    /// Returns `true` when the active config changed (caller must `send_config`).
    pub fn render(&mut self, ui: &imgui::Ui, config: &mut Config) -> bool {
        let mut changed = false;

        // ── New config creation ───────────────────────────────────────────
        ui.text("New config:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        ui.input_text("##new_cfg", &mut self.new_config_name).build();
        ui.same_line();
        if ui.button("Add") && !self.new_config_name.is_empty() {
            let mut name = self.new_config_name.clone();
            if !name.ends_with(".toml") {
                name.push_str(".toml");
            }
            let path = CONFIG_PATH.join(&name);
            write_config(config, &path);
            self.current_config = path;
            self.available_configs = available_configs();
            self.new_config_name.clear();
        }
        ui.same_line();
        if ui.button("Refresh") {
            self.available_configs = available_configs();
        }
        ui.same_line();
        if ui.button("Open Folder") {
            let _ = std::process::Command::new("xdg-open")
                .arg(BASE_PATH.as_os_str())
                .status();
        }

        ui.separator();

        // ── UI scale ──────────────────────────────────────────────────────
        ui.text("UI Scale:");
        ui.same_line();
        for (label, scale) in [("50%", 0.50_f32), ("75%", 0.75_f32), ("100%", 1.0), ("125%", 1.25), ("150%", 1.5)] {
            let selected = (self.ui_scale - scale).abs() < 0.01;
            if selected {
                ui.text(format!("[{}]", label));
            } else if ui.small_button(label) {
                self.ui_scale = scale;
                self.persist_app_state();
            }
            ui.same_line();
        }
        ui.new_line();
        let mut scale_pct = self.ui_scale * 100.0;
        ui.set_next_item_width(120.0);
        if ui.slider_config("##ui_scale", 50.0_f32, 400.0_f32)
            .display_format("%.0f%%")
            .build(&mut scale_pct)
        {
            self.ui_scale = (scale_pct / 100.0).clamp(0.5, 4.0);
            self.persist_app_state();
        }

        ui.separator();

        // ── Theme / Accent style ──────────────────────────────────────────
        let style_strings: Vec<String> =
            AccentStyle::iter().map(|s| format!("{:?}", s)).collect();
        let style_strs: Vec<&str> = style_strings.iter().map(|s| s.as_str()).collect();
        let styles: Vec<AccentStyle> = AccentStyle::iter().collect();
        let mut style_idx: usize = styles
            .iter()
            .position(|s| *s == config.accent_style)
            .unwrap_or(0);
        if ui.combo_simple_string("Theme##accent", &mut style_idx, &style_strs) {
            config.accent_style = styles[style_idx];
            changed = true;
        }

        ui.separator();

        // ── Menu size & position ──────────────────────────────────────────
        ui.text("Menu Size:");
        ui.set_next_item_width(120.0);
        if ui.input_float("W##menu_w", &mut self.menu_width)
            .enter_returns_true(true)
            .build()
        {
            self.menu_width = self.menu_width.clamp(400.0, 1600.0);
            self.persist_app_state();
            self.resize_requested = true;
        }
        ui.same_line();
        ui.set_next_item_width(120.0);
        if ui.input_float("H##menu_h", &mut self.menu_height)
            .enter_returns_true(true)
            .build()
        {
            self.menu_height = self.menu_height.clamp(200.0, 1200.0);
            self.persist_app_state();
            self.resize_requested = true;
        }

        ui.separator();

        // ── Config list ───────────────────────────────────────────────────
        let configs: Vec<std::path::PathBuf> = self.available_configs.clone();
        let mut clicked = None;
        let mut to_delete = None;

        for cfg_path in &configs {
            let label = cfg_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");

            let selected = *cfg_path == self.current_config;
            if ui.selectable_config(label).selected(selected).build() {
                clicked = Some(cfg_path.clone());
            }
            ui.same_line();
            let del_label = format!("X##{}", label);
            if ui.small_button(&del_label) {
                to_delete = Some(cfg_path.clone());
            }
        }

        if let Some(path) = clicked {
            *config = parse_config(&path);
            self.current_config = path;
            changed = true;
        }

        if let Some(path) = to_delete {
            delete_config(&path);
            self.available_configs = available_configs();
            if !self.available_configs.is_empty() {
                self.current_config = self.available_configs[0].clone();
                *config = parse_config(&self.current_config);
                changed = true;
            }
        }

        changed
    }
}

impl Default for ConfigPanel {
    fn default() -> Self {
        Self::new()
    }
}
