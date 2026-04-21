use std::{cell::Cell, num::NonZeroU32, sync::Arc, time::Instant};

use egui::{Color32, FontData, FontDefinitions, Stroke, Style};
use egui_glow::glow::{self, HasContext as _};
use glutin::prelude::PossiblyCurrentGlContext;
use winit::platform::x11::{WindowAttributesExtX11, WindowType};
use winit::raw_window_handle::RawWindowHandle;
use x11rb::{
    connection::Connection as X11Connection,
    protocol::xproto::{ConfigureWindowAux, ConnectionExt as _, StackMode},
    rust_connection::RustConnection,
};

use crate::ui::color::{ColorScheme, Colors};

/// How often [`WindowContext::raise_to_top`] issues a `ConfigureWindow` request
/// to the X11 server. 5 Hz is enough to keep the overlay above CS2 in all DWM
/// modes while generating negligible X11 traffic.
const RAISE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);

pub struct WindowContext {
    window: winit::window::Window,
    gl_context: glutin::context::PossiblyCurrentContext,
    _gl_display: glutin::display::Display,
    gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    glow: Arc<glow::Context>,
    egui_glow: egui_glow::EguiGlow,
    clear_color: Color32,
    /// X11rb connection used to raise the overlay to the top of the X11
    /// window stack. `None` when the connection could not be established.
    x11_conn: Option<RustConnection>,
    /// X11 resource ID of the overlay window. `None` when running on a
    /// non-X11 backend (e.g. Wayland) or when not an overlay window.
    x11_window_id: Option<u32>,
    /// Timestamp of the last `raise_to_top` X11 request, used to throttle
    /// requests to [`RAISE_INTERVAL`] to avoid flooding the X11 server.
    last_raise: Cell<Instant>,
}

impl WindowContext {
    pub fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
        overlay: bool,
        color_scheme: &ColorScheme,
        window_size: Option<(u32, u32)>,
    ) -> Self {
        use glutin::context::NotCurrentGlContext as _;
        use glutin::display::GetGlDisplay as _;
        use glutin::display::GlDisplay as _;
        use glutin::prelude::GlSurface as _;
        use winit::raw_window_handle::HasWindowHandle as _;

        let winit_window_builder = if overlay {
            winit::window::WindowAttributes::default()
                .with_decorations(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(1, 1))
                .with_position(winit::dpi::PhysicalPosition::new(0, 0))
                .with_resizable(true)
                .with_transparent(true)
                .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
                .with_override_redirect(true)
                .with_x11_window_type(vec![WindowType::Tooltip])
                .with_title("Bo$$Hook420_overlay")
        } else {
            let (w, h) = window_size.unwrap_or((1000, 800));
            winit::window::WindowAttributes::default()
                .with_inner_size(winit::dpi::LogicalSize::new(w, h))
                .with_resizable(true)
                .with_title("Bo$$Hook420")
        };

        let config_template_builder = if overlay {
            glutin::config::ConfigTemplateBuilder::new()
                .prefer_hardware_accelerated(Some(true))
                .with_transparency(true)
        } else {
            glutin::config::ConfigTemplateBuilder::new()
                .prefer_hardware_accelerated(Some(true))
                .with_transparency(false)
        };

        let (mut window, gl_config) =
            glutin_winit::DisplayBuilder::new() // let glutin-winit helper crate handle the complex parts of opengl context creation
                .with_preference(glutin_winit::ApiPreference::FallbackEgl) // https://github.com/emilk/egui/issues/2520#issuecomment-1367841150
                .with_window_attributes(Some(winit_window_builder.clone()))
                .build(
                    event_loop,
                    config_template_builder,
                    |mut config_iterator| {
                        config_iterator.next().expect(
                            "failed to find a matching configuration for creating glutin config",
                        )
                    },
                )
                .expect("failed to create gl_config");
        let gl_display = gl_config.display();

        let raw_window_handle = window.as_ref().map(|w| {
            w.window_handle()
                .expect("failed to get window handle")
                .as_raw()
        });
        let context_attributes =
            glutin::context::ContextAttributesBuilder::new().build(raw_window_handle);
        let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(raw_window_handle);
        let not_current_gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .unwrap_or_else(|_| {
                    gl_config
                        .display()
                        .create_context(&gl_config, &fallback_context_attributes)
                        .expect("failed to create context even with fallback attributes")
                })
        };

        // this is where the window is created, if it has not been created while searching for suitable gl_config
        let window = window.take().unwrap_or_else(|| {
            glutin_winit::finalize_window(event_loop, winit_window_builder.clone(), &gl_config)
                .expect("failed to finalize glutin window")
        });
        let (width, height): (u32, u32) = window.inner_size().into();
        let width = NonZeroU32::new(width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(height).unwrap_or(NonZeroU32::MIN);
        let surface_attributes =
            glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                .build(
                    window
                        .window_handle()
                        .expect("failed to get window handle")
                        .as_raw(),
                    width,
                    height,
                );
        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &surface_attributes)
                .unwrap()
        };
        let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

        gl_surface
            .set_swap_interval(&gl_context, glutin::surface::SwapInterval::DontWait)
            .unwrap();

        if overlay {
            window.set_cursor_hittest(false).unwrap();
            window.set_outer_position(winit::dpi::PhysicalPosition::new(0, 0));
        }

        let glow = unsafe {
            glow::Context::from_loader_function(|s| {
                let s = std::ffi::CString::new(s)
                    .expect("failed to construct C string from string for gl proc address");

                gl_display.get_proc_address(&s)
            })
        };

        let glow = Arc::new(glow);
        let mut egui_glow = egui_glow::EguiGlow::new(event_loop, glow.clone(), None, None, true);
        prep_ctx(&mut egui_glow.egui_ctx, color_scheme);

        let clear_color = if overlay {
            Color32::TRANSPARENT
        } else {
            Color32::BLACK
        };

        // Set up x11rb connection and window ID for overlay stack management.
        let (x11_conn, x11_window_id) = if overlay {
            let conn_result = RustConnection::connect(None);
            let window_id = window
                .window_handle()
                .ok()
                .and_then(|h| match h.as_raw() {
                    RawWindowHandle::Xcb(h) => Some(h.window.get()),
                    RawWindowHandle::Xlib(h) => Some(h.window as u32),
                    _ => None,
                });
            match conn_result {
                Ok((conn, _)) => (Some(conn), window_id),
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        };

        Self {
            window,
            gl_context,
            _gl_display: gl_display,
            gl_surface,
            glow,
            egui_glow,
            clear_color,
            x11_conn,
            x11_window_id,
            last_raise: Cell::new(Instant::now()),
        }
    }

    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    pub fn glow(&self) -> &Arc<glow::Context> {
        &self.glow
    }

    pub fn resize(&self, physical_size: winit::dpi::PhysicalSize<u32>) {
        use glutin::surface::GlSurface as _;
        let width = NonZeroU32::new(physical_size.width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(physical_size.height).unwrap_or(NonZeroU32::MIN);
        self.gl_surface.resize(&self.gl_context, width, height);
    }

    pub fn swap_buffers(&self) -> glutin::error::Result<()> {
        use glutin::surface::GlSurface as _;
        self.gl_surface.swap_buffers(&self.gl_context)
    }

    pub fn make_current(&self) -> glutin::error::Result<()> {
        self.gl_context.make_current(&self.gl_surface)
    }

    pub fn process_event(&mut self, event: &winit::event::WindowEvent) -> egui_glow::EventResponse {
        self.egui_glow.on_window_event(&self.window, event)
    }

    pub fn process_modifier(&mut self, modifiers: egui::Modifiers, pressed: bool, repeat: bool) {
        self.egui_glow.egui_ctx.input_mut(|i| {
            i.events.push(egui::Event::Key {
                key: egui::Key::F35,
                physical_key: None,
                pressed,
                repeat,
                modifiers,
            });
        });
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Raise the overlay to the top of the X11 window stack.
    ///
    /// Issues a lightweight `ConfigureWindow` request (no round-trip required)
    /// that keeps the overlay permanently at the top of the X11 window stack.
    /// Requests are throttled to [`RAISE_INTERVAL`] (200ms) to avoid flooding
    /// the X11 server on every rendered frame.
    pub fn raise_to_top(&self) {
        let (Some(conn), Some(window_id)) = (&self.x11_conn, self.x11_window_id) else {
            return;
        };
        let now = Instant::now();
        if now.duration_since(self.last_raise.get()) < RAISE_INTERVAL {
            return;
        }
        self.last_raise.set(now);
        let aux = ConfigureWindowAux::new().stack_mode(StackMode::ABOVE);
        let _ = conn.configure_window(window_id, &aux);
        let _ = conn.flush();
    }

    pub fn run(&mut self, func: impl FnMut(&mut egui::Ui)) {
        self.egui_glow.run(&self.window, func);
    }

    pub fn clear(&self) {
        let [r, g, b, a] = self.clear_color.to_normalized_gamma_f32();
        unsafe {
            self.glow.clear_color(r, g, b, a);
            self.glow.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    pub fn paint(&mut self) {
        self.egui_glow.paint(&self.window);
    }

    /// Re-apply a new color scheme to the egui context (e.g. when the user
    /// switches theme while the application is running).
    pub fn apply_style(&self, color_scheme: &ColorScheme) {
        self.egui_glow.egui_ctx.style_mut_of(egui::Theme::Dark, |style| {
            gui_style(style, color_scheme);
        });
    }
}

impl Drop for WindowContext {
    fn drop(&mut self) {
        self.egui_glow.destroy();
    }
}

fn prep_ctx(ctx: &mut egui::Context, color_scheme: &ColorScheme) {
    // add font
    let fira_sans = include_bytes!("../../resources/Quicksand.ttf");
    let cs2_icons = include_bytes!("../../resources/CS2EquipmentIcons.ttf");
    let mut font_definitions = FontDefinitions::default();
    font_definitions.font_data.insert(
        String::from("Quicksand"),
        Arc::new(FontData::from_static(fira_sans)),
    );
    font_definitions.font_data.insert(
        String::from("cs2_icons"),
        Arc::new(FontData::from_static(cs2_icons)),
    );

    // insert into font definitions, so it gets chosen as default
    font_definitions
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, String::from("Quicksand"));
    font_definitions
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, String::from("cs2_icons"));

    ctx.set_fonts(font_definitions);

    ctx.style_mut_of(egui::Theme::Dark, |style| {
        gui_style(style, color_scheme);
    });
}

fn gui_style(style: &mut Style, scheme: &ColorScheme) {
    style.interaction.selectable_labels = false;
    for font in style.text_styles.iter_mut() {
        font.1.size = 16.0;
    }

    style.visuals.window_fill = scheme.base;
    style.visuals.panel_fill = scheme.base;
    style.visuals.extreme_bg_color = scheme.backdrop;

    // Remove all rounded corners
    style.visuals.window_corner_radius = egui::CornerRadius::ZERO;
    style.visuals.menu_corner_radius = egui::CornerRadius::ZERO;

    let bg_stroke = Stroke::new(1.0, scheme.subtext);
    let fg_stroke = Stroke::new(1.0, Colors::TEXT);
    let dark_stroke = Stroke::new(1.0, scheme.base);

    // Hover state uses the highlight color; active/pressed state uses the accent
    let hover_fill = scheme.highlight;
    let active_fill = scheme.accent;

    style.visuals.selection.bg_fill = scheme.accent;
    style.visuals.selection.stroke = dark_stroke;

    style.visuals.widgets.active.bg_fill = active_fill;
    style.visuals.widgets.active.bg_stroke = bg_stroke;
    style.visuals.widgets.active.fg_stroke = fg_stroke;
    style.visuals.widgets.active.weak_bg_fill = active_fill;
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;

    style.visuals.widgets.hovered.bg_fill = hover_fill;
    style.visuals.widgets.hovered.bg_stroke = bg_stroke;
    style.visuals.widgets.hovered.fg_stroke = fg_stroke;
    style.visuals.widgets.hovered.weak_bg_fill = hover_fill;
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;

    // Inactive widgets blend into the background — only active/selected elements pop
    style.visuals.widgets.inactive.bg_fill = scheme.backdrop;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, scheme.highlight);
    style.visuals.widgets.inactive.fg_stroke = fg_stroke;
    style.visuals.widgets.inactive.weak_bg_fill = scheme.backdrop;
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;

    // Noninteractive (labels, separators, frames) — subtle, background-level
    style.visuals.widgets.noninteractive.bg_fill = scheme.base;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, scheme.highlight);
    style.visuals.widgets.noninteractive.fg_stroke = fg_stroke;
    style.visuals.widgets.noninteractive.weak_bg_fill = scheme.base;
    style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;

    style.visuals.widgets.open.bg_fill = hover_fill;
    style.visuals.widgets.open.bg_stroke = bg_stroke;
    style.visuals.widgets.open.fg_stroke = fg_stroke;
    style.visuals.widgets.open.weak_bg_fill = hover_fill;
    style.visuals.widgets.open.corner_radius = egui::CornerRadius::ZERO;

    style.spacing.button_padding = egui::vec2(4.0, 2.0);
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
}
