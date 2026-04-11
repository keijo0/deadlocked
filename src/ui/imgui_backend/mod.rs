//! ImGui context wrapper — initialization, event routing, and per-frame
//! rendering for the settings GUI window.

pub mod panels;
mod renderer;

use std::{sync::Arc, time::Instant};

use egui_glow::glow;
use winit::{event::WindowEvent, window::Window};

use renderer::ImGuiRenderer;
pub use panels::{AimbotPanel, ConfigPanel, EspPanel, GrenadePanel, HudPanel, MiscPanel};

use crate::ui::color::{AccentStyle, ColorScheme};

// ── Public context struct ─────────────────────────────────────────────────────

pub struct ImGuiContext {
    pub context: imgui::Context,
    pub platform: imgui_winit_support::WinitPlatform,
    pub(crate) renderer: ImGuiRenderer,
    pub last_frame: Instant,

    // ── Persistent panel state ───────────────────────────────────────────
    pub aimbot: AimbotPanel,
    pub esp: EspPanel,
    pub hud: HudPanel,
    pub misc: MiscPanel,
    pub cfg_panel: ConfigPanel,
    pub grenades: GrenadePanel,

    /// Tracks the last applied accent style so we only rebuild colors on change.
    pub last_accent_style: Option<AccentStyle>,
}

impl ImGuiContext {
    /// Create a new `ImGuiContext` and upload the font atlas to the GPU.
    pub fn new(window: &Window, glow: &Arc<glow::Context>) -> Self {
        let mut context = imgui::Context::create();

        context.set_ini_filename(None);
        context.set_log_filename(None);

        // Disable keyboard navigation so alt-tab doesn't cycle through tabs.
        context.io_mut().config_flags.remove(imgui::ConfigFlags::NAV_ENABLE_KEYBOARD);

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut context);
        platform.attach_window(
            context.io_mut(),
            window,
            imgui_winit_support::HiDpiMode::Default,
        );

        // Apply default dark style on first creation.
        apply_imgui_colors(context.style_mut(), &AccentStyle::Default);

        let renderer =
            ImGuiRenderer::new(glow, &mut context).expect("failed to initialize imgui renderer");

        Self {
            context,
            platform,
            renderer,
            last_frame: Instant::now(),
            aimbot: AimbotPanel::new(),
            esp: EspPanel::new(),
            hud: HudPanel::new(),
            misc: MiscPanel::new(),
            cfg_panel: ConfigPanel::new(),
            grenades: GrenadePanel::new(),
            last_accent_style: Some(AccentStyle::Default),
        }
    }

    /// Re-apply imgui colors for the given accent style (only if changed).
    pub fn apply_accent_if_changed(&mut self, accent: &AccentStyle) {
        if self.last_accent_style.as_ref() == Some(accent) {
            return;
        }
        apply_imgui_colors(self.context.style_mut(), accent);
        self.last_accent_style = Some(accent.clone());
    }

    /// Forward a winit `WindowEvent` for the GUI window to imgui.
    pub fn handle_window_event(
        &mut self,
        window: &Window,
        window_id: winit::window::WindowId,
        event: &WindowEvent,
    ) {
        let wrapped: winit::event::Event<()> = winit::event::Event::WindowEvent {
            window_id,
            event: event.clone(),
        };
        self.platform
            .handle_event(self.context.io_mut(), window, &wrapped);
    }
}

// ── Style helpers ─────────────────────────────────────────────────────────────

fn c32f4(c: egui::Color32) -> [f32; 4] {
    [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, c.a() as f32 / 255.0]
}

/// Apply imgui colors derived from an `AccentStyle`.
pub fn apply_imgui_colors(style: &mut imgui::Style, accent_style: &AccentStyle) {
    use imgui::StyleColor;

    style.window_rounding = 0.0;
    style.frame_rounding = 2.0;
    style.scrollbar_rounding = 2.0;
    style.tab_rounding = 2.0;
    style.grab_rounding = 2.0;
    style.child_rounding = 0.0;
    style.popup_rounding = 2.0;

    style.window_padding = [8.0, 8.0];
    style.frame_padding = [4.0, 2.0];
    style.item_spacing = [6.0, 4.0];
    style.item_inner_spacing = [4.0, 4.0];

    let scheme = ColorScheme::for_style(accent_style);

    let is_light = matches!(accent_style, AccentStyle::Winter);

    let bg          = c32f4(scheme.backdrop);
    let panel       = c32f4(scheme.base);
    let highlight   = c32f4(scheme.highlight);
    let accent      = c32f4(scheme.accent);
    let accent_b    = c32f4(scheme.accent_bright);
    let subtext     = c32f4(scheme.subtext);

    let text = if is_light { [0.05, 0.05, 0.05, 1.0] } else { [0.90, 0.90, 0.90, 1.0] };
    let text_dim = [0.50, 0.50, 0.50, 1.0];

    // Frame background: slightly lighter than panel on dark themes.
    let frame_bg = if is_light {
        [0.82, 0.84, 0.87, 1.0]
    } else {
        [
            (panel[0] * 0.75).min(1.0),
            (panel[1] * 0.75).min(1.0),
            (panel[2] * 0.75).min(1.0),
            1.0,
        ]
    };
    let frame_hov = highlight;
    let frame_act = accent;

    style[StyleColor::Text]                  = text;
    style[StyleColor::TextDisabled]          = text_dim;
    style[StyleColor::WindowBg]              = bg;
    style[StyleColor::ChildBg]               = panel;
    style[StyleColor::PopupBg]               = panel;
    style[StyleColor::FrameBg]               = frame_bg;
    style[StyleColor::FrameBgHovered]        = frame_hov;
    style[StyleColor::FrameBgActive]         = frame_act;
    style[StyleColor::TitleBg]               = bg;
    style[StyleColor::TitleBgActive]         = bg;
    style[StyleColor::MenuBarBg]             = panel;
    style[StyleColor::ScrollbarBg]           = bg;
    style[StyleColor::ScrollbarGrab]         = subtext;
    style[StyleColor::ScrollbarGrabHovered]  = accent;
    style[StyleColor::ScrollbarGrabActive]   = accent_b;
    style[StyleColor::CheckMark]             = accent;
    style[StyleColor::SliderGrab]            = accent;
    style[StyleColor::SliderGrabActive]      = accent_b;
    style[StyleColor::Button]                = highlight;
    style[StyleColor::ButtonHovered]         = accent;
    style[StyleColor::ButtonActive]          = accent_b;
    style[StyleColor::Header]                = highlight;
    style[StyleColor::HeaderHovered]         = accent;
    style[StyleColor::HeaderActive]          = accent_b;
    style[StyleColor::Tab]                   = panel;
    style[StyleColor::TabHovered]            = accent;
    style[StyleColor::TabActive]             = accent;
    style[StyleColor::TabUnfocused]          = panel;
    style[StyleColor::TabUnfocusedActive]    = highlight;
    style[StyleColor::Separator]             = subtext;
    style[StyleColor::SeparatorHovered]      = accent;
    style[StyleColor::SeparatorActive]       = accent_b;
}
