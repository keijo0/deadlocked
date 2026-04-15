//! Player ESP settings panel.
use crate::{
    config::{BoxMode, Config, DrawMode},
    cs2::key_codes::KeyCode,
};
use super::color_edit;
use imgui::Key;

pub struct EspPanel {
    listening_id: Option<String>,
}

impl EspPanel {
    pub fn new() -> Self {
        Self { listening_id: None }
    }

    /// Returns `true` when any setting changed.
    pub fn render(&mut self, ui: &imgui::Ui, config: &mut Config) -> bool {
        let mut changed = false;
        let p = &mut config.player;

        changed |= ui.checkbox("Enable ESP", &mut p.enabled);

        // ESP hotkey — inline keybind button
        changed |= keybind_button(ui, "ESP Hotkey", "esp_hk", &mut p.esp_hotkey, &mut self.listening_id);

        changed |= ui.checkbox("Show Friendlies", &mut p.show_friendlies);
        changed |= ui.checkbox("Visible Only", &mut p.visible_only);

        ui.separator();

        // Box draw mode
        let modes = ["None", "Health", "Color"];
        let mut box_idx = draw_mode_idx(&p.draw_box);
        if ui.combo_simple_string("Box Mode", &mut box_idx, &modes) {
            p.draw_box = idx_to_draw_mode(box_idx);
            changed = true;
        }
        ui.same_line();
        changed |= color_edit(ui, "##box_vis_col", &mut p.box_visible_color);
        ui.same_line();
        changed |= color_edit(ui, "##box_invis_col", &mut p.box_invisible_color);

        // Box style (Gap / Full)
        let box_modes = ["Gap", "Full"];
        let mut bm_idx: usize = match p.box_mode {
            BoxMode::Gap => 0,
            BoxMode::Full => 1,
        };
        if ui.combo_simple_string("Box Style", &mut bm_idx, &box_modes) {
            p.box_mode = match bm_idx {
                1 => BoxMode::Full,
                _ => BoxMode::Gap,
            };
            changed = true;
        }

        // Skeleton draw mode
        let mut skel_idx = draw_mode_idx(&p.draw_skeleton);
        if ui.combo_simple_string("Skeleton Mode", &mut skel_idx, &modes) {
            p.draw_skeleton = idx_to_draw_mode(skel_idx);
            changed = true;
        }
        ui.same_line();
        changed |= color_edit(ui, "##skel_col", &mut p.skeleton_color);

        ui.separator();

        changed |= ui.checkbox("Health Bar", &mut p.health_bar);
        changed |= ui.checkbox("Health Text", &mut p.health_text);
        changed |= ui.checkbox("Armor Text", &mut p.armor_text);
        changed |= ui.checkbox("Player Name", &mut p.player_name);
        changed |= ui.checkbox("Weapon Name", &mut p.weapon_icon);
        changed |= ui.checkbox("Show Tags", &mut p.tags);

        ui.separator();

        if p.backtrack_visual {
            changed |= color_edit(ui, "##bt_col", &mut p.backtrack_color);
            ui.same_line();
        }
        changed |= ui.checkbox("Backtrack Visual", &mut p.backtrack_visual);
        if p.backtrack_visual {
            let mut ms = p.backtrack_ms as i32;
            if ui.slider_config("  Backtrack (ms)", 15_i32, 200_i32).build(&mut ms) {
                p.backtrack_ms = ms.max(15) as u32;
                changed = true;
            }
            changed |= ui.checkbox("  Follows ESP", &mut p.backtrack_in_esp);
        }

        ui.separator();

        // Unsafe Features Section
        ui.text("Unsafe (Memory Writing)");
        ui.separator();
        
        let misc = &mut config.misc;
        
        // No Flash
        changed |= ui.checkbox("No Flash", &mut misc.no_flash);
        if misc.no_flash {
            let mut max_alpha = misc.max_flash_alpha;
            if ui.slider_config("  Max Flash Alpha", 0.0, 255.0).build(&mut max_alpha) {
                misc.max_flash_alpha = max_alpha;
                changed = true;
            }
        }

        // No Smoke
        changed |= ui.checkbox("No Smoke", &mut misc.no_smoke);
        

        changed
    }
}

fn keybind_button(
    ui: &imgui::Ui,
    label: &str,
    id: &str,
    keycode: &mut KeyCode,
    listening_id: &mut Option<String>,
) -> bool {
    let is_listening = listening_id.as_deref() == Some(id);
    
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

    if ui.is_key_pressed_no_repeat(Key::Escape) {
        *listening_id = None;
        return false;
    }

    let mouse_map = [
        (imgui::MouseButton::Left,   KeyCode::MouseLeft),
        (imgui::MouseButton::Right,  KeyCode::MouseRight),
        (imgui::MouseButton::Middle, KeyCode::MouseMiddle),
        (imgui::MouseButton::Extra1, KeyCode::Mouse4),
        (imgui::MouseButton::Extra2, KeyCode::Mouse5),
    ];
    
    // First try imgui's built-in detection
    for (btn, kc) in mouse_map {
        if ui.is_mouse_clicked(btn) {
            *keycode = kc;
            *listening_id = None;
            return true;
        }
    }
    
    // Additional check for Mouse4/Mouse5 using mouse_down state
    // Check if these buttons are currently pressed (even if imgui didn't detect the click)
    let mouse4_down = ui.is_mouse_down(imgui::MouseButton::Extra1);
    let mouse5_down = ui.is_mouse_down(imgui::MouseButton::Extra2);
    
    if mouse4_down {
        *keycode = KeyCode::Mouse4;
        *listening_id = None;
        return true;
    }
    
    if mouse5_down {
        *keycode = KeyCode::Mouse5;
        *listening_id = None;
        return true;
    }

    let wheel = ui.io().mouse_wheel;
    if wheel > 0.0 { *keycode = KeyCode::MouseWheelUp;   *listening_id = None; return true; }
    if wheel < 0.0 { *keycode = KeyCode::MouseWheelDown; *listening_id = None; return true; }

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
        (Key::Space, KeyCode::Space),
        (Key::LeftShift, KeyCode::LeftShift),
        (Key::LeftCtrl,  KeyCode::LeftControl),
        (Key::LeftAlt,   KeyCode::LeftAlt),
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

fn draw_mode_idx(mode: &DrawMode) -> usize {
    match mode {
        DrawMode::None   => 0,
        DrawMode::Health => 1,
        DrawMode::Color  => 2,
    }
}

fn idx_to_draw_mode(idx: usize) -> DrawMode {
    match idx {
        1 => DrawMode::Health,
        2 => DrawMode::Color,
        _ => DrawMode::None,
    }
}


impl Default for EspPanel {
    fn default() -> Self {
        Self::new()
    }
}
