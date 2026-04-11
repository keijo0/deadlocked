//! Panels for the imgui settings GUI.
pub mod aimbot;
pub mod config;
pub mod esp;
pub mod grenade;
pub mod hud;
pub mod menu;
pub mod misc;

pub use aimbot::AimbotPanel;
pub use config::ConfigPanel;
pub use esp::EspPanel;
pub use grenade::GrenadePanel;
pub use hud::HudPanel;
pub use menu::MainMenu;
pub use misc::MiscPanel;

// ── Color helpers ─────────────────────────────────────────────────────────────

/// Convert `egui::Color32` to an RGBA `[f32; 4]` array that imgui color-edit
/// widgets accept.
pub fn c32_to_f4(c: egui::Color32) -> [f32; 4] {
    [
        c.r() as f32 / 255.0,
        c.g() as f32 / 255.0,
        c.b() as f32 / 255.0,
        c.a() as f32 / 255.0,
    ]
}

/// Convert an RGBA `[f32; 4]` array back to `egui::Color32`.
pub fn f4_to_c32(c: [f32; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (c[0] * 255.0).round() as u8,
        (c[1] * 255.0).round() as u8,
        (c[2] * 255.0).round() as u8,
        (c[3] * 255.0).round() as u8,
    )
}

/// Render an imgui color-edit widget that reads/writes an `egui::Color32`.
/// Shows only the small colored square (no text inputs). Returns `true` when changed.
pub fn color_edit(ui: &imgui::Ui, label: &str, color: &mut egui::Color32) -> bool {
    use imgui::ColorEditFlags;
    let mut arr = c32_to_f4(*color);
    let id = format!("##col_{}", label);
    let changed = ui
        .color_edit4_config(&id, &mut arr)
        .flags(ColorEditFlags::NO_INPUTS | ColorEditFlags::NO_LABEL)
        .build();
    if changed {
        *color = f4_to_c32(arr);
    }
    changed
}
