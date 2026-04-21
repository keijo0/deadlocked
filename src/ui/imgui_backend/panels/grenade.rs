//! Grenade lineup editor panel.
use std::sync::Arc;

use utils::sync::Mutex;

use crate::{
    constants::cs2::GRENADES,
    data::Data,
    ui::grenades::{Grenade, GrenadeList, write_grenades},
};

pub struct GrenadePanel {
    grenades: GrenadeList,
    new_grenade: Grenade,
    /// Active grenade being edited: (map_name, index_in_list).
    current_grenade: Option<(String, usize)>,
}

impl GrenadePanel {
    pub fn new() -> Self {
        use crate::ui::grenades::read_grenades;
        Self {
            grenades: read_grenades(),
            new_grenade: Grenade::new(),
            current_grenade: None,
        }
    }

    /// Render the panel. `data` is the live game data used for recording new
    /// grenades.  This panel does not change `Config`, so it always returns
    /// `false`.
    pub fn render(&mut self, ui: &imgui::Ui, data: &Arc<Mutex<Data>>) {
        let avail = ui.content_region_avail();
        ui.child_window("##grenades_scroll")
            .size(avail)
            .build(|| {
                if self.current_grenade.is_some() {
                    self.render_edit(ui);
                } else {
                    self.render_record(ui, data);
                }

                ui.separator();

                // Grenade list grouped by map.
                if let Some(_node) = ui.tree_node("Grenade List") {
                    self.render_list(ui);
                }
            });
    }

    // ── Record new grenade ────────────────────────────────────────────────

    fn render_record(&mut self, ui: &imgui::Ui, data: &Arc<Mutex<Data>>) {
        let data = data.lock();

        if !data.in_game {
            ui.text_colored([0.9, 0.8, 0.2, 1.0], "Not in game.");
            return;
        }

        if !GRENADES.contains(&data.local_player.weapon) {
            ui.text_colored([0.9, 0.8, 0.2, 1.0], "Invalid Weapon (hold a grenade).");
            return;
        }

        // Name field
        ui.text("Name:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        ui.input_text("##grn_name", &mut self.new_grenade.name).build();

        // Description field
        ui.text("Instructions:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        ui.input_text_multiline(
            "##grn_desc",
            &mut self.new_grenade.description,
            [200.0, 60.0],
        )
        .build();

        ui.checkbox("Jump", &mut self.new_grenade.modifiers.jump);
        ui.same_line();
        ui.checkbox("Duck", &mut self.new_grenade.modifiers.duck);
        ui.same_line();
        ui.checkbox("Run", &mut self.new_grenade.modifiers.run);

        if ui.button("Save Grenade") {
            let map = data.map_name.clone();
            let grenade_list = self.grenades.entry(map).or_default();

            let mut new_g = Grenade::new();
            std::mem::swap(&mut new_g, &mut self.new_grenade);
            new_g.weapon = data.local_player.weapon.clone();
            new_g.position = data.local_player.position;
            new_g.view_angles = data.view_angles;

            grenade_list.push(new_g);
            write_grenades(&self.grenades);
            self.new_grenade = Grenade::new();
        }
    }

    // ── Edit existing grenade ─────────────────────────────────────────────

    fn render_edit(&mut self, ui: &imgui::Ui) {
        let (map, index) = match &self.current_grenade {
            Some(g) => (g.0.clone(), g.1),
            None => return,
        };

        let Some(grenades) = self.grenades.get_mut(&map) else {
            return;
        };
        let Some(grenade) = grenades.get_mut(index) else {
            return;
        };

        ui.text("Name:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        ui.input_text("##edit_name", &mut grenade.name).build();

        ui.text("Description:");
        ui.same_line();
        ui.set_next_item_width(200.0);
        ui.input_text_multiline("##edit_desc", &mut grenade.description, [200.0, 60.0])
            .build();

        ui.checkbox("Jump##e", &mut grenade.modifiers.jump);
        ui.same_line();
        ui.checkbox("Duck##e", &mut grenade.modifiers.duck);
        ui.same_line();
        ui.checkbox("Run##e", &mut grenade.modifiers.run);

        if ui.button("Done##edit") {
            write_grenades(&self.grenades);
            self.current_grenade = None;
        }
        ui.same_line();
        if ui.button("Cancel##edit") {
            self.current_grenade = None;
        }
    }

    // ── Grenade list ──────────────────────────────────────────────────────

    fn render_list(&mut self, ui: &imgui::Ui) {
        let mut should_write = false;
        let maps: Vec<String> = self.grenades.keys().cloned().collect();

        for map in &maps {
            if let Some(_node) = ui.tree_node(map) {
                let mut delete_idx: Option<usize> = None;

                if let Some(grenades) = self.grenades.get(map) {
                    let grenades_cloned: Vec<(usize, String)> = grenades
                        .iter()
                        .enumerate()
                        .map(|(i, g)| (i, g.name.clone()))
                        .collect();

                    for (index, name) in grenades_cloned {
                        let active = matches!(
                            &self.current_grenade,
                            Some((m, i)) if m == map && *i == index
                        );
                        if ui.selectable_config(&name).selected(active).build() {
                            self.current_grenade = if active {
                                None
                            } else {
                                Some((map.clone(), index))
                            };
                        }
                        ui.same_line();
                        let del_lbl = format!("X##{map}{index}");
                        if ui.small_button(&del_lbl) {
                            delete_idx = Some(index);
                        }
                    }
                }

                if let Some(idx) = delete_idx {
                    if let Some(list) = self.grenades.get_mut(map) {
                        list.remove(idx);
                        should_write = true;
                        // Clear selection if it points into the now-modified list.
                        if let Some((m, i)) = &self.current_grenade {
                            if m == map && *i >= idx {
                                self.current_grenade = None;
                            }
                        }
                    }
                }
            }
        }

        if should_write {
            write_grenades(&self.grenades);
        }
    }
}

impl Default for GrenadePanel {
    fn default() -> Self {
        Self::new()
    }
}
