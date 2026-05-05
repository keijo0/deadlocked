use egui::{DragValue, Ui};
use strum::IntoEnumIterator as _;

use crate::{
    cs2::{bones::Bones, entity::weapon::Weapon},
    ui::{
        app::App,
        drag_range::DragRange,
        gui::helpers::{
            checkbox, checkbox_hover, combo_box, drag, keybind, keybind_list, scroll,
        },
    },
};

#[derive(PartialEq)]
pub enum AimbotTab {
    Global,
    Weapon,
}

impl App {
    pub fn aimbot_settings(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.aimbot_tab, AimbotTab::Global, "Global");
            ui.selectable_value(&mut self.aimbot_tab, AimbotTab::Weapon, "Weapon");
            if self.aimbot_tab == AimbotTab::Weapon {
                egui::ComboBox::new("weapon_select", "")
                    .selected_text(format!("{:?}", self.aimbot_weapon))
                    .show_ui(ui, |ui| {
                        for weapon in Weapon::iter() {
                            let text = format!("{:?}", weapon);
                            ui.selectable_value(&mut self.aimbot_weapon, weapon, text);
                        }
                    });
                if ui.button("Current").clicked() {
                    self.aimbot_weapon = self.data.lock().weapon.clone();
                }
            }
        });
        ui.separator();
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            scroll(left, "aimbot_left", |ui| self.aimbot_left(ui));

            let right = &mut cols[1];
            scroll(right, "aimbot_right", |ui| self.aimbot_right(ui));
        });
    }

    fn aimbot_left(&mut self, ui: &mut Ui) {
        if keybind_list(
            ui,
            "aimbot_hotkeys",
            "Hotkeys",
            &mut self.config.aim.aimbot_hotkeys,
        ) {
            self.send_config();
        }

        if self.aimbot_tab == AimbotTab::Weapon
            && checkbox_hover(
                ui,
                "Enable Override",
                "Enable aimbot settings override for a specific weapon",
                &mut self.weapon_config().aimbot.enable_override,
            )
        {
            self.send_config();
        }

        if checkbox(ui, "Enable Aimbot", &mut self.weapon_config().aimbot.enabled) {
            self.send_config();
        }

        if combo_box(ui, "Mode", &mut self.weapon_config().aimbot.mode) {
            self.send_config();
        }

        if checkbox_hover(
            ui,
            "Target Friendlies",
            "Only active in custom game modes (workshop/custom maps)",
            &mut self.weapon_config().aimbot.target_friendlies,
        ) {
            self.send_config();
        }

        if checkbox_hover(
            ui,
            "Distance-Adjusted FOV",
            "Adjusts FOV based on target distance",
            &mut self.weapon_config().aimbot.distance_adjusted_fov,
        ) {
            self.send_config();
        }

        if self.weapon_config().aimbot.distance_adjusted_fov {
            if drag(
                ui,
                "  Scale Distance",
                DragValue::new(&mut self.weapon_config().aimbot.fov_scale_distance)
                    .range(50.0..=3000.0)
                    .suffix(" u")
                    .speed(5.0)
                    .max_decimals(0),
            ) {
                self.send_config();
            }

            if drag(
                ui,
                "  Max Scale",
                DragValue::new(&mut self.weapon_config().aimbot.fov_max_scale)
                    .range(1.0..=20.0)
                    .suffix("×")
                    .speed(0.05)
                    .max_decimals(2),
            ) {
                self.send_config();
            }
        }

        if drag(
            ui,
            "FOV",
            DragValue::new(&mut self.weapon_config().aimbot.fov)
                .range(0.1..=360.0)
                .suffix("°")
                .speed(0.02)
                .max_decimals(1),
        ) {
            self.send_config();
        }

        if drag(
            ui,
            "Smooth",
            DragValue::new(&mut self.weapon_config().aimbot.smooth)
                .range(0.0..=20.0)
                .speed(0.02)
                .max_decimals(1),
        ) {
            self.send_config();
        }

        if drag(
            ui,
            "Start Bullet",
            DragValue::new(&mut self.weapon_config().aimbot.start_bullet)
                .range(0..=10)
                .speed(0.05),
        ) {
            self.send_config();
        }

        ui.separator();

        // Checks — Visibility and Smoke/Wall on one row; Flash Check is config-only.
        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut self.weapon_config().aimbot.visibility_check,
                    "Visibility",
                )
                .changed()
            {
                self.send_config();
            }
            if ui
                .checkbox(
                    &mut self.weapon_config().aimbot.smoke_wall_check,
                    "Wall/Smoke",
                )
                .on_hover_text("Blocks aiming through map geometry and smoke volumes")
                .changed()
            {
                self.send_config();
            }
        });

        ui.separator();

        // Bones — wrapped horizontal layout for compactness.
        ui.horizontal_wrapped(|ui| {
            // Single bones
            for bone in [Bones::Head, Bones::Neck, Bones::Hip] {
                let text = format!("{:?}", bone);
                let index = self
                    .weapon_config()
                    .aimbot
                    .bones
                    .iter()
                    .position(|b| *b == bone);
                if ui.selectable_label(index.is_some(), text).clicked() {
                    if let Some(index) = index {
                        self.weapon_config().aimbot.bones.remove(index);
                    } else {
                        self.weapon_config().aimbot.bones.push(bone);
                    }
                    self.send_config();
                }
            }

            // Spine (all four segments as one toggle)
            {
                let spine_bones =
                    [Bones::Spine1, Bones::Spine2, Bones::Spine3, Bones::Spine4];
                let selected = spine_bones
                    .iter()
                    .any(|b| self.weapon_config().aimbot.bones.contains(b));
                if ui.selectable_label(selected, "Spine").clicked() {
                    if selected {
                        self.weapon_config()
                            .aimbot
                            .bones
                            .retain(|b| !spine_bones.contains(b));
                    } else {
                        for b in spine_bones {
                            self.weapon_config().aimbot.bones.push(b);
                        }
                    }
                    self.send_config();
                }
            }

            // Merged L/R bone pairs
            for (label, left, right) in [
                ("Shoulder", Bones::LeftShoulder, Bones::RightShoulder),
                ("Elbow", Bones::LeftElbow, Bones::RightElbow),
                ("Hand", Bones::LeftHand, Bones::RightHand),
                ("Knee", Bones::LeftKnee, Bones::RightKnee),
                ("Foot", Bones::LeftFoot, Bones::RightFoot),
                ("Hip", Bones::LeftHip, Bones::RightHip),
            ] {
                let has_left = self.weapon_config().aimbot.bones.contains(&left);
                let has_right = self.weapon_config().aimbot.bones.contains(&right);
                let selected = has_left || has_right;
                if ui.selectable_label(selected, label).clicked() {
                    if selected {
                        self.weapon_config()
                            .aimbot
                            .bones
                            .retain(|b| *b != left && *b != right);
                    } else {
                        self.weapon_config().aimbot.bones.push(left);
                        self.weapon_config().aimbot.bones.push(right);
                    }
                    self.send_config();
                }
            }
        });
    }

    fn aimbot_right(&mut self, ui: &mut Ui) {
        if self.aimbot_tab == AimbotTab::Weapon
            && ui
                .checkbox(
                    &mut self.weapon_config().triggerbot.enable_override,
                    "Enable Override",
                )
                .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(
                &mut self.weapon_config().triggerbot.enabled,
                "Enable Triggerbot",
            )
            .changed()
        {
            self.send_config();
        }

        if keybind(
            ui,
            "triggerbot_hotkey",
            "Hotkey",
            &mut self.config.aim.triggerbot_hotkey,
        ) {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(DragRange::new(
                    &mut self.weapon_config().triggerbot.delay,
                    0..=999,
                ))
                .changed()
            {
                self.send_config();
            }
            ui.label("Delay (ms)");
        });

        if combo_box(
            ui,
            "Mode",
            &mut self.weapon_config().triggerbot.mode,
        ) {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.weapon_config().triggerbot.head_only, "Head Only")
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.weapon_config().triggerbot.shot_duration)
                        .range(0..=2000)
                        .speed(10.0),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Additional Duration (ms)");
        });

        ui.separator();

        // Safety checks — Flash and Scope on one row.
        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut self.weapon_config().triggerbot.flash_check,
                    "Flash",
                )
                .changed()
            {
                self.send_config();
            }
            if ui
                .checkbox(
                    &mut self.weapon_config().triggerbot.scope_check,
                    "Scope",
                )
                .changed()
            {
                self.send_config();
            }
        });

        // Velocity check with inline threshold.
        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut self.weapon_config().triggerbot.velocity_check,
                    "Velocity ≤",
                )
                .on_hover_text("Only shoot if the player moves slower than the specified threshold")
                .changed()
            {
                self.send_config();
            }
            if ui
                .add(
                    DragValue::new(&mut self.weapon_config().triggerbot.velocity_threshold)
                        .range(0..=5000),
                )
                .on_hover_text(
                    "Maximum velocity at which the triggerbot can shoot (in CS2 Units)",
                )
                .changed()
            {
                self.send_config();
            }
        });

        ui.separator();

        // Autowall / penetration section.
        if ui
            .checkbox(
                &mut self.weapon_config().triggerbot.autowall_enabled,
                "Autowall",
            )
            .on_hover_text("Shoots through walls if bullet penetration is sufficient")
            .changed()
        {
            self.send_config();
        }

        if self.weapon_config().triggerbot.autowall_enabled {
            if keybind(
                ui,
                "autowall_hotkey",
                "  Hotkey",
                &mut self.config.aim.autowall_hotkey,
            ) {
                self.send_config();
            }

            if combo_box(
                ui,
                "  Mode",
                &mut self.weapon_config().triggerbot.autowall_mode,
            ) {
                self.send_config();
            }

            if ui
                .checkbox(
                    &mut self.weapon_config().triggerbot.autowall_safe,
                    "  Safe Mode",
                )
                .on_hover_text("Only shoot through walls when any part of the player model is visible")
                .changed()
            {
                self.send_config();
            }

            if keybind(
                ui,
                "autowall_threshold_hotkey",
                "  Threshold Hotkey",
                &mut self.config.aim.autowall_threshold_hotkey,
            ) {
                self.send_config();
            }
            if drag(
                ui,
                "  Threshold",
                DragValue::new(&mut self.weapon_config().triggerbot.autowall_threshold)
                    .range(50..=99)
                    .suffix("%")
                    .speed(1.0),
            ) {
                self.send_config();
            }
        }

        ui.separator();

        // Magnet mode section.
        if checkbox_hover(
            ui,
            "Magnet Mode",
            "Fires when the crosshair is within the specified FOV of a visible enemy",
            &mut self.weapon_config().triggerbot.magnet_enabled,
        ) {
            self.send_config();
        }

        if self.weapon_config().triggerbot.magnet_enabled {
            if drag(
                ui,
                "  FOV",
                DragValue::new(&mut self.weapon_config().triggerbot.magnet_fov)
                    .range(0.1..=30.0)
                    .suffix("°")
                    .speed(0.05)
                    .max_decimals(1),
            ) {
                self.send_config();
            }
            if drag(
                ui,
                "  Hitchance",
                DragValue::new(&mut self.weapon_config().triggerbot.magnet_hitchance)
                    .range(0..=100)
                    .suffix("%")
                    .speed(1.0),
            ) {
                self.send_config();
            }
            if drag(
                ui,
                "  Smooth",
                DragValue::new(&mut self.weapon_config().triggerbot.magnet_smooth)
                    .range(0.0..=19.0)
                    .speed(0.05)
                    .max_decimals(1),
            ) {
                self.send_config();
            }
            if checkbox_hover(
                ui,
                "  Rage",
                "Instant snap + zero delay: tracks target every frame and fires immediately",
                &mut self.weapon_config().triggerbot.magnet_rage,
            ) {
                self.send_config();
            }
        }

        ui.separator();

        if self.aimbot_tab == AimbotTab::Weapon
            && ui
                .checkbox(
                    &mut self.weapon_config().rcs.enable_override,
                    "Enable Override",
                )
                .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.weapon_config().rcs.enabled, "Enable RCS")
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.weapon_config().rcs.smooth)
                        .range(0.0..=10.0)
                        .speed(0.05)
                        .max_decimals(2),
                )
                .on_hover_text("0 = instant full compensation; higher = smoother/slower")
                .changed()
            {
                self.send_config();
            }
            ui.label("RCS Smooth");
        });

        if self.aimbot_tab == AimbotTab::Global {
            ui.separator();
        }
    }
}
