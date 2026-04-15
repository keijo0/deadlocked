//! Main menu bar with tab navigation.
use std::sync::Arc;

use utils::sync::Mutex;

use crate::{config::Config, data::Data, message::GameStatus};
use super::{AimbotPanel, ConfigPanel, EspPanel, GrenadePanel, HudPanel, MiscPanel};

// Tab labels — used for width pre-calculation to right-align the tab bar.
const TABS: &[&str] = &["Aimbot", "Player", "HUD", "Misc", "Grenades", "Config"];

pub struct MainMenu;

impl MainMenu {
    pub fn new() -> Self {
        Self
    }

    /// Render the full settings UI.
    ///
    /// Returns `true` when the user changed a setting (caller should call
    /// `send_config`).
    pub fn render(
        ui: &imgui::Ui,
        config: &mut Config,
        game_status: GameStatus,
        data: &Arc<Mutex<Data>>,
        esp: &mut EspPanel,
        aimbot: &mut AimbotPanel,
        hud: &mut HudPanel,
        misc: &mut MiscPanel,
        cfg_panel: &mut ConfigPanel,
        grenades: &mut GrenadePanel,
    ) -> bool {
        let mut changed = false;

        let mw = cfg_panel.menu_width;
        let mh = cfg_panel.menu_height;

        // ── Floating window with generous padding ────────────────────────
        let _win_padding = ui.push_style_var(imgui::StyleVar::WindowPadding([6.0, 4.0]));
        let window_token = ui.window("##deadlocked_menu")
            .size([mw, mh], imgui::Condition::Always)
            .position([0.0, 0.0], imgui::Condition::Always)
            .title_bar(false)
            .scrollable(false)
            .movable(false)
            .resizable(false)
            .bring_to_front_on_focus(false)
            .begin();

        let Some(_wt) = window_token else { return changed };

        // ── Header row: status indicator (left) + tab bar (right) ────────
        // Push tab-bar style vars *before* measuring text so widths are correct.
        const TAB_FP_X: f32 = 8.0; // FramePadding.x for tab items
        const TAB_IS_X: f32 = 4.0; // ItemSpacing.x between tab items
        let frame_padding = ui.push_style_var(imgui::StyleVar::FramePadding([TAB_FP_X, 6.0]));
        let item_spacing  = ui.push_style_var(imgui::StyleVar::ItemSpacing([TAB_IS_X, 3.0]));

        // Compute the total width needed for the tab bar so we can right-align it.
        let tab_bar_width: f32 = TABS.iter()
            .map(|t| ui.calc_text_size(t)[0].ceil() + TAB_FP_X * 2.0)
            .sum::<f32>()
            + TAB_IS_X * (TABS.len() as f32 - 1.0)
            + 4.0; // small float-rounding guard

        // Draw status indicator at the current (top-left) cursor position.
        let header_y = ui.cursor_pos()[1];
        let (status_text, status_color) = match game_status {
            GameStatus::Working    => ("HAHAHAHHAHAHAHAHAHAHHAHAHAHAHAHAHAHAHHAHAHAHAHAH niggerhook", [0.2, 0.9, 0.2, 1.0f32]),
            GameStatus::NotStarted => ("you know whats hotter than a femboy?", [0.9, 0.2, 0.2, 1.0f32]),
        };
        ui.text_colored(status_color, status_text);

        // Move cursor to the right edge minus the tab bar width (same header row).
        let right_edge = ui.content_region_max()[0];
        let tab_x = (right_edge - tab_bar_width).max(0.0);
        ui.set_cursor_pos([tab_x, header_y]);

        if let Some(tabs) = ui.tab_bar_with_flags(
            "##tabs",
            imgui::TabBarFlags::NO_TAB_LIST_SCROLLING_BUTTONS,
        ) {
            if let Some(tab) = ui.tab_item("Aimbot") {
                ui.separator();
                changed |= aimbot.render(ui, config, data);
                tab.end();
            }
            if let Some(tab) = ui.tab_item("Player") {
                ui.separator();
                changed |= esp.render(ui, config);
                tab.end();
            }
            if let Some(tab) = ui.tab_item("HUD") {
                ui.separator();
                changed |= hud.render(ui, config);
                tab.end();
            }
            if let Some(tab) = ui.tab_item("Misc") {
                ui.separator();
                changed |= misc.render(ui, config);
                tab.end();
            }
            if let Some(tab) = ui.tab_item("Nades") {
                ui.separator();
                grenades.render(ui, data);
                tab.end();
            }
            if let Some(tab) = ui.tab_item("Config") {
                ui.separator();
                changed |= cfg_panel.render(ui, config);
                tab.end();
            }
            tabs.end();
        }
        frame_padding.pop();
        item_spacing.pop();

        changed
    }
}

impl Default for MainMenu {
    fn default() -> Self {
        Self::new()
    }
}
