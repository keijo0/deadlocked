use egui::{Color32, Grid, RichText, ScrollArea, Ui};

use crate::server_picker::{
    Continent, FetchResult, ServerRegion, block_region, fetch_servers_async, new_fetch_result,
    unblock_region,
};

const CS2_PROCESS: &str = "cs2";

/// Returns `true` when a process named `cs2` is found under `/proc`.
fn is_cs2_running() -> bool {
    let Ok(proc) = std::fs::read_dir("/proc") else {
        return false;
    };
    for entry in proc.flatten() {
        let name = entry.file_name();
        let pid_str = name.to_string_lossy();
        if !pid_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if std::fs::read_link(format!("/proc/{pid_str}/exe"))
            .map(|exe| exe.file_name().map(|n| n == CS2_PROCESS).unwrap_or(false))
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

pub struct App {
    server_regions: Vec<ServerRegion>,
    server_picker_result: FetchResult,
    server_picker_loading: bool,
    server_picker_error: Option<String>,
    /// Cached result of the last CS2 process check.
    cs2_running: bool,
    /// Countdown until the next CS2 process check (in update ticks).
    cs2_check_timer: u32,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            server_regions: Vec::new(),
            server_picker_result: new_fetch_result(),
            server_picker_loading: false,
            server_picker_error: None,
            cs2_running: is_cs2_running(),
            cs2_check_timer: 0,
        }
    }

    /// Poll the async fetch result and move it into `self.server_regions` if ready.
    fn poll_server_picker(&mut self) {
        let result = self.server_picker_result.lock().unwrap().take();
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
                        if let Some(old) = old_blocked.iter().find(|r| r.name == new_region.name) {
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

    /// Check CS2 process status every ~120 frames (~2 s at 60 fps).
    fn poll_cs2_status(&mut self) {
        if self.cs2_check_timer == 0 {
            self.cs2_running = is_cs2_running();
            self.cs2_check_timer = 120;
        } else {
            self.cs2_check_timer -= 1;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.poll_server_picker();
        self.poll_cs2_status();

        // CS2 status banner
            let (status_text, status_color) = if self.cs2_running {
                ("● CS2 is running", Color32::from_rgb(80, 200, 80))
            } else {
                ("● CS2 is not running", Color32::from_rgb(160, 160, 160))
            };
            ui.label(RichText::new(status_text).color(status_color).small());

            ui.separator();

            // Load / Reload / block-all / unblock-all row
            ui.horizontal(|ui| {
                let btn_label = if self.server_picker_loading {
                    "Loading…"
                } else if self.server_regions.is_empty() {
                    "Load Servers"
                } else {
                    "Reload Servers"
                };

                if ui
                    .add_enabled(!self.server_picker_loading, egui::Button::new(btn_label))
                    .clicked()
                {
                    self.server_picker_loading = true;
                    self.server_picker_error = None;
                    fetch_servers_async(self.server_picker_result.clone());
                }

                if !self.server_regions.is_empty() && ui.button("Unblock All").clicked() {
                    let ips: Vec<Vec<String>> = self
                        .server_regions
                        .iter()
                        .filter(|r| r.blocked)
                        .map(|r| r.relay_ips.clone())
                        .collect();
                    for relay_ips in &ips {
                        unblock_region(relay_ips);
                    }
                    for region in &mut self.server_regions {
                        region.blocked = false;
                    }
                }

                if !self.server_regions.is_empty() && ui.button("Block All").clicked() {
                    for region in &mut self.server_regions {
                        if !region.blocked {
                            block_region(&region.relay_ips);
                            region.blocked = true;
                        }
                    }
                }
            });

            if let Some(err) = &self.server_picker_error {
                ui.label(RichText::new(format!("⚠ {err}")).color(Color32::RED));
            }

            if self.server_picker_loading {
                ui.spinner();
                return;
            }

            if self.server_regions.is_empty() {
                ui.label(
                    RichText::new("Press \"Load Servers\" to fetch relay regions.")
                        .color(Color32::GRAY),
                );
                return;
            }

            ui.add_space(4.0);

            let mut to_block: Option<usize> = None;
            let mut to_unblock: Option<usize> = None;
            let mut to_block_continent: Option<Continent> = None;
            let mut to_unblock_continent: Option<Continent> = None;

            ScrollArea::vertical()
                .id_salt("server_picker_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    Grid::new("server_picker_grid")
                        .num_columns(3)
                        .striped(true)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Region").strong());
                            ui.label(RichText::new("Key").strong());
                            ui.label(RichText::new("").strong());
                            ui.end_row();

                            let mut prev_continent: Option<Continent> = None;

                            for (i, region) in self.server_regions.iter().enumerate() {
                                if prev_continent != Some(region.continent) {
                                    prev_continent = Some(region.continent);
                                    ui.label(
                                        RichText::new(region.continent.as_str())
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                    ui.label("");
                                    ui.horizontal(|ui| {
                                        if ui
                                            .small_button(
                                                RichText::new("Block All").color(Color32::RED),
                                            )
                                            .clicked()
                                        {
                                            to_block_continent = Some(region.continent);
                                        }
                                        if ui
                                            .small_button(
                                                RichText::new("Unblock All")
                                                    .color(Color32::from_rgb(80, 200, 80)),
                                            )
                                            .clicked()
                                        {
                                            to_unblock_continent = Some(region.continent);
                                        }
                                    });
                                    ui.end_row();
                                }

                                ui.label(&region.description);
                                ui.label(
                                    RichText::new(&region.name)
                                        .color(ui.style().visuals.weak_text_color()),
                                );
                                if region.blocked {
                                    if ui
                                        .button(
                                            RichText::new("Unblock")
                                                .color(Color32::from_rgb(80, 200, 80)),
                                        )
                                        .clicked()
                                    {
                                        to_unblock = Some(i);
                                    }
                                } else if ui
                                    .button(RichText::new("Block").color(Color32::RED))
                                    .clicked()
                                {
                                    to_block = Some(i);
                                }
                                ui.end_row();
                            }
                        });
                });

            if let Some(i) = to_block {
                let relay_ips = self.server_regions[i].relay_ips.clone();
                block_region(&relay_ips);
                self.server_regions[i].blocked = true;
            }
            if let Some(i) = to_unblock {
                let relay_ips = self.server_regions[i].relay_ips.clone();
                unblock_region(&relay_ips);
                self.server_regions[i].blocked = false;
            }
            if let Some(continent) = to_block_continent {
                let indices: Vec<usize> = self
                    .server_regions
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.continent == continent && !r.blocked)
                    .map(|(i, _)| i)
                    .collect();
                for i in indices {
                    block_region(&self.server_regions[i].relay_ips);
                    self.server_regions[i].blocked = true;
                }
            }
            if let Some(continent) = to_unblock_continent {
                let indices: Vec<usize> = self
                    .server_regions
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.continent == continent && r.blocked)
                    .map(|(i, _)| i)
                    .collect();
                for i in indices {
                    unblock_region(&self.server_regions[i].relay_ips);
                    self.server_regions[i].blocked = false;
                }
            }

        // Keep UI responsive while loading.
        if self.server_picker_loading {
            ui.ctx().request_repaint();
        }
    }
}
