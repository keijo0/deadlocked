//! Misc / unsafe settings panel — includes Anti-AFK and Server Picker.
use std::sync::Arc;

use utils::sync::Mutex;

use crate::{
    config::Config,
    server_picker::{
        Continent, FetchResult, ServerRegion, block_region, fetch_servers_async, new_fetch_result,
        unblock_region,
    },
};

pub struct MiscPanel {
    server_regions: Vec<ServerRegion>,
    server_picker_loading: bool,
    server_picker_error: Option<String>,
    server_picker_result: FetchResult,
}

impl MiscPanel {
    pub fn new() -> Self {
        Self {
            server_regions: Vec::new(),
            server_picker_loading: false,
            server_picker_error: None,
            server_picker_result: new_fetch_result(),
        }
    }

    /// Returns `true` when any setting changed.
    pub fn render(&mut self, ui: &imgui::Ui, config: &mut Config) -> bool {
        let mut changed = false;

        // ── Anti-AFK ──────────────────────────────────────────────────────
        ui.text("Anti-AFK");
        ui.separator();

        let afk = &mut config.misc.antiafk;
        changed |= ui.checkbox("Enable Anti-AFK", &mut afk.enabled);

        if afk.enabled {
            let mut imin = afk.interval_min;
            if ui.slider_config("Interval Min (s)", 1.0_f32, 60.0_f32)
                .display_format("%.1f s")
                .build(&mut imin)
            {
                afk.interval_min = imin;
                changed = true;
            }

            let mut imax = afk.interval_max;
            if ui.slider_config("Interval Max (s)", 1.0_f32, 60.0_f32)
                .display_format("%.1f s")
                .build(&mut imax)
            {
                afk.interval_max = imax;
                changed = true;
            }
        }

        ui.separator();

        // ── Server Picker ─────────────────────────────────────────────────
        ui.text("Server Picker");
        ui.separator();

        self.poll_server_picker();

        // Load / Reload button
        let btn_label = if self.server_picker_loading {
            "Loading..."
        } else if self.server_regions.is_empty() {
            "Load Servers"
        } else {
            "Reload Servers"
        };

        if !self.server_picker_loading && ui.button(btn_label) {
            self.server_picker_loading = true;
            self.server_picker_error = None;
            fetch_servers_async(self.server_picker_result.clone());
        }

        if !self.server_regions.is_empty() {
            ui.same_line();
            if ui.button("Unblock All") {
                for region in &mut self.server_regions {
                    if region.blocked {
                        unblock_region(&region.relay_ips);
                        region.blocked = false;
                    }
                }
            }
            ui.same_line();
            if ui.button("Block All") {
                for region in &mut self.server_regions {
                    if !region.blocked {
                        block_region(&region.relay_ips);
                        region.blocked = true;
                    }
                }
            }
        }

        if let Some(err) = &self.server_picker_error.clone() {
            ui.text_colored([1.0, 0.2, 0.2, 1.0], format!("Error: {err}"));
        }

        if self.server_picker_loading {
            ui.text("Loading...");
            return changed;
        }

        if self.server_regions.is_empty() {
            ui.text("Press \"Load Servers\" to fetch relay regions.");
            return changed;
        }

        // Per-continent / per-region table
        let mut to_block: Option<usize> = None;
        let mut to_unblock: Option<usize> = None;
        let mut to_block_continent: Option<Continent> = None;
        let mut to_unblock_continent: Option<Continent> = None;

        let h = ui.content_region_avail()[1].min(260.0).max(60.0);
        ui.child_window("##server_scroll")
            .size([0.0, h])
            .build(|| {
                let mut prev_continent: Option<Continent> = None;
                for (i, region) in self.server_regions.iter().enumerate() {
                    if prev_continent != Some(region.continent) {
                        prev_continent = Some(region.continent);
                        // Continent header
                        ui.text_colored([0.8, 0.8, 1.0, 1.0], region.continent.as_str());
                        ui.same_line();
                        let bc_label = format!("Block All##{:?}", region.continent);
                        let ub_label = format!("Unblock All##{:?}", region.continent);
                        if ui.small_button(&bc_label) {
                            to_block_continent = Some(region.continent);
                        }
                        ui.same_line();
                        if ui.small_button(&ub_label) {
                            to_unblock_continent = Some(region.continent);
                        }
                    }

                    // Region row
                    let desc_col = [0.85, 0.85, 0.85, 1.0];
                    ui.text_colored(desc_col, format!("  {}", region.description));
                    ui.same_line();
                    ui.text_colored([0.6, 0.6, 0.6, 1.0], format!("[{}]", region.name));
                    ui.same_line();
                    if region.blocked {
                        let ub_lbl = format!("Unblock##{i}");
                        if ui.small_button(&ub_lbl) {
                            to_unblock = Some(i);
                        }
                    } else {
                        let b_lbl = format!("Block##{i}");
                        if ui.small_button(&b_lbl) {
                            to_block = Some(i);
                        }
                    }
                }
            });

        // Apply deferred actions
        if let Some(i) = to_block {
            let ips = self.server_regions[i].relay_ips.clone();
            block_region(&ips);
            self.server_regions[i].blocked = true;
        }
        if let Some(i) = to_unblock {
            let ips = self.server_regions[i].relay_ips.clone();
            unblock_region(&ips);
            self.server_regions[i].blocked = false;
        }
        if let Some(cont) = to_block_continent {
            let indices: Vec<usize> = self
                .server_regions
                .iter()
                .enumerate()
                .filter(|(_, r)| r.continent == cont && !r.blocked)
                .map(|(i, _)| i)
                .collect();
            for i in indices {
                block_region(&self.server_regions[i].relay_ips.clone());
                self.server_regions[i].blocked = true;
            }
        }
        if let Some(cont) = to_unblock_continent {
            let indices: Vec<usize> = self
                .server_regions
                .iter()
                .enumerate()
                .filter(|(_, r)| r.continent == cont && r.blocked)
                .map(|(i, _)| i)
                .collect();
            for i in indices {
                unblock_region(&self.server_regions[i].relay_ips.clone());
                self.server_regions[i].blocked = false;
            }
        }

        changed
    }

    /// Poll the async fetch result and merge it into `self.server_regions`.
    fn poll_server_picker(&mut self) {
        let result = self.server_picker_result.lock().take();
        if let Some(result) = result {
            self.server_picker_loading = false;
            match result {
                Ok(mut regions) => {
                    let old_blocked: Vec<ServerRegion> = self
                        .server_regions
                        .iter()
                        .filter(|r| r.blocked)
                        .cloned()
                        .collect();

                    for new_region in &mut regions {
                        if let Some(old) =
                            old_blocked.iter().find(|r| r.name == new_region.name)
                        {
                            unblock_region(&old.relay_ips);
                            block_region(&new_region.relay_ips);
                            new_region.blocked = true;
                        }
                    }
                    for old in &old_blocked {
                        if !regions.iter().any(|r| r.name == old.name) {
                            unblock_region(&old.relay_ips);
                        }
                    }

                    self.server_regions = regions;
                    self.server_picker_error = None;
                }
                Err(e) => {
                    self.server_picker_error = Some(e);
                }
            }
        }
    }
}

// suppress unused-import warning for Arc/Mutex when server picker isn't needed
#[allow(unused_imports)]
use Arc as _Arc;
#[allow(unused_imports)]
use Mutex as _Mutex;

impl Default for MiscPanel {
    fn default() -> Self {
        Self::new()
    }
}
