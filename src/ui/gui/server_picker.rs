use egui::{Color32, Grid, RichText, ScrollArea, Ui};

use crate::{
    server_picker::{Continent, block_region, fetch_servers_async, unblock_region, ServerRegion},
    ui::{app::App, color::Colors, gui::helpers::collapsing_open},
};

impl App {
    /// Poll the async fetch result and move it into `self.server_regions` if ready.
    fn poll_server_picker(&mut self) {
        let result = self.server_picker_result.lock().take();
        if let Some(result) = result {
            self.server_picker_loading = false;
            match result {
                Ok(regions) => {
                    // Unblock any previously-blocked regions so that iptables rules
                    // from the old list are not left behind as orphans.
                    let old_blocked: Vec<&ServerRegion> = self
                        .server_regions
                        .iter()
                        .filter(|r| r.blocked)
                        .collect();
                    for region in old_blocked {
                        unblock_region(&region.relay_ips);
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

    pub fn server_picker_settings(&mut self, ui: &mut Ui) {
        self.poll_server_picker();

        collapsing_open(ui, "Server Picker", |ui| {
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

                if !self.server_regions.is_empty()
                    && ui.button("Unblock All").clicked()
                {
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

                if !self.server_regions.is_empty()
                    && ui.button("Block All").clicked()
                {
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
                        .color(Colors::TEXT),
                );
                return;
            }

            ui.add_space(4.0);

            // Collect per-row and per-continent actions before rendering to avoid borrow conflicts.
            let mut to_block: Option<usize> = None;
            let mut to_unblock: Option<usize> = None;
            let mut to_block_continent: Option<Continent> = None;
            let mut to_unblock_continent: Option<Continent> = None;

            ScrollArea::vertical()
                .id_salt("server_picker_scroll")
                .max_height(260.0)
                .auto_shrink([false, true])
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
                                // Emit a continent header row when the continent changes.
                                if prev_continent != Some(region.continent) {
                                    prev_continent = Some(region.continent);
                                    ui.label(
                                        RichText::new(region.continent.as_str())
                                            .strong()
                                            .color(Colors::TEXT),
                                    );
                                    ui.label(""); // key column — intentionally empty
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
                                                    .color(Colors::GREEN),
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
                                        .button(RichText::new("Unblock").color(Colors::GREEN))
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
        });
    }
}
