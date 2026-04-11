use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use utils::{channel::Channel, log, sync::Mutex};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, StartCause, WindowEvent},
    keyboard::NamedKey,
};

use crate::{
    config::{
        CONFIG_PATH, Config, DEFAULT_CONFIG_NAME, parse_config,
        read_app_state, write_config,
    },
    data::Data,
    message::{GameStatus, Message},
    server_picker::ServerRegion,
    ui::{
        color::{AccentStyle, ColorScheme},
        grenades::{GrenadeList, read_grenades},
        imgui_backend::ImGuiContext,
        trail::Trail,
        window_context::WindowContext,
    },
};

pub struct App {
    pub gui: Option<WindowContext>,
    pub overlay: Option<WindowContext>,
    /// imgui context for the settings GUI window (replaces the egui GUI panels).
    pub imgui_ctx: Option<ImGuiContext>,
    next_frame_time: Instant,

    pub channel: Channel<Message>,
    pub data: Arc<Mutex<Data>>,

    pub game_status: GameStatus,
    pub display_scale: f32,
    pub overlay_window_pos: Option<(i32, i32)>,
    pub overlay_window_size: Option<(u32, u32)>,
    pub trails: HashMap<u64, Trail>,

    pub grenades: GrenadeList,

    pub config: Config,
    pub current_config: PathBuf,

    /// Tracks which accent style was last applied to both egui contexts so we
    /// can detect changes and re-apply the full color scheme on the fly.
    pub last_applied_style: AccentStyle,

    /// Frame-time tracker used for adaptive overlay frame skipping.
    pub frame_timer: crate::performance::FrameTimer,

    // Server Picker
    pub server_regions: Vec<ServerRegion>,

    // Media info (playerctl)
    pub media_info_text: String,
    pub media_info_last_poll: Instant,

    // Watermark (date/time + weather via curl)
    pub watermark_text: String,
    pub watermark_last_poll: Instant,
}

impl App {
    pub fn new(channel: Channel<Message>, data: Arc<Mutex<Data>>) -> Self {
        // read config
        let config = parse_config(&CONFIG_PATH.join(DEFAULT_CONFIG_NAME));
        // override config if invalid
        write_config(&config, &CONFIG_PATH.join(DEFAULT_CONFIG_NAME));
        let grenades = read_grenades();

        let ret = Self {
            gui: None,
            overlay: None,
            imgui_ctx: None,

            next_frame_time: Instant::now() + frame_duration(&config),

            channel,
            data,
            current_config: CONFIG_PATH.join(DEFAULT_CONFIG_NAME),

            game_status: GameStatus::NotStarted,
            display_scale: 1.0,
            overlay_window_pos: None,
            overlay_window_size: None,
            trails: HashMap::new(),

            grenades,

            last_applied_style: config.accent_style.clone(),

            frame_timer: crate::performance::FrameTimer::new(frame_duration(&config)),

            server_regions: Vec::new(),
            media_info_text: String::new(),
            media_info_last_poll: Instant::now(),
            watermark_text: String::new(),
            watermark_last_poll: Instant::now() - Duration::from_secs(300),
            config,
        };
        ret.send_config();
        ret
    }

    fn create_window(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let current_style = self.config.accent_style.clone();
        let scheme = ColorScheme::for_style(&current_style);

        // Size the GUI window to exactly contain the imgui menu.
        let app_state = read_app_state();
        let win_w = app_state.menu_width.ceil() as u32;
        let win_h = app_state.menu_height.ceil() as u32;

        let gui = WindowContext::new(event_loop, false, &scheme, Some((win_w, win_h)));
        let overlay = WindowContext::new(event_loop, true, &scheme, None);
        self.last_applied_style = current_style;

        self.display_scale = gui.window().scale_factor() as f32;
        log::info!("detected display scale: {}", self.display_scale);

        // `overlay`'s WindowContext::new() calls make_current() last, so the
        // overlay's GL context is now current.  Switch back to the GUI context
        // so that all imgui GL objects (VAO, VBO, shaders, font texture) are
        // created in the GUI's context rather than the overlay's context.
        gui.make_current().expect("failed to make GUI GL context current for imgui init");

        let imgui_ctx = ImGuiContext::new(gui.window(), gui.glow());

        self.gui = Some(gui);
        self.overlay = Some(overlay);
        self.imgui_ctx = Some(imgui_ctx);
    }

    /// Poll `playerctl` for the currently-playing track, at most once per second.
    /// Falls back to an empty string if playerctl is unavailable or nothing is playing.
    pub fn poll_media_info(&mut self) {
        if !self.config.hud.media_info {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.media_info_last_poll) < Duration::from_secs(1) {
            return;
        }
        self.media_info_last_poll = now;

        let output = std::process::Command::new("playerctl")
            .args(["metadata", "--format", "{{artist}} - {{title}}"])
            .output();

        self.media_info_text = match output {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            }
            _ => String::new(),
        };
    }

    /// Poll date/time and weather for the watermark, at most once per 30 minutes.
    /// Weather fetch only runs when the `watermark_weather` option is enabled.
    /// Falls back gracefully if curl or wttr.in is unavailable.
    pub fn poll_watermark_info(&mut self) {
        if !self.config.hud.watermark || !self.config.hud.watermark_weather {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.watermark_last_poll) < Duration::from_secs(1800) {
            return;
        }
        self.watermark_last_poll = now;

        let weather = std::process::Command::new("curl")
            .args(["-m", "3", "--silent", "wttr.in?format=3"])
            .output()
            .ok()
            .and_then(|out| {
                if out.status.success() {
                    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if s.is_empty() { None } else { Some(s) }
                } else {
                    None
                }
            })
            .unwrap_or_default();

        self.watermark_text = weather;
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            if let Some(window) = &self.gui {
                window.window().request_redraw();
            }
            if let Some(window) = &self.overlay {
                window.window().request_redraw();
            }
            let budget = frame_duration(&self.config);
            self.next_frame_time += budget;
            self.frame_timer.update_budget(budget);
        }
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.create_window(event_loop);

        self.next_frame_time = Instant::now() + frame_duration(&self.config);
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
            self.next_frame_time,
        ));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        window_event: WindowEvent,
    ) {
        while let Ok(message) = self.channel.try_receive() {
            if let Message::GameStatus(status) = message {
                self.game_status = status
            }
        }

        let Some(gui) = &self.gui else {
            return;
        };
        let Some(overlay) = &self.overlay else {
            return;
        };

        let is_gui_window = gui.window().id() == window_id;

        let window = if is_gui_window {
            gui
        } else if overlay.window().id() == window_id {
            overlay
        } else {
            return;
        };

        match &window_event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                window.resize(*new_size);
                // When the user resizes the GUI window via the native window
                // manager, update the imgui menu dimensions to match so the
                // menu fills the new size without needing a restart.
                if is_gui_window {
                    if let Some(imgui_ctx) = &mut self.imgui_ctx {
                        let scale = self.gui.as_ref().unwrap().window().scale_factor();
                        let logical = new_size.to_logical::<f32>(scale);
                        let cp = &mut imgui_ctx.cfg_panel;
                        cp.menu_width  = logical.width.max(100.0);
                        cp.menu_height = logical.height.max(100.0);
                        cp.persist_app_state();
                        // imgui-winit-support 0.13's prepare_frame() does NOT
                        // update display_size — only handle_event() does.
                        // Since the Resized event is handled here rather than
                        // forwarded to imgui, we must update it directly so the
                        // renderer uses the correct viewport on the very next
                        // frame (otherwise a black band appears at the top).
                        let io = imgui_ctx.context.io_mut();
                        io.display_size = [logical.width, logical.height];
                        io.display_framebuffer_scale = [scale as f32, scale as f32];
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                    self.next_frame_time,
                ));
                // Render once per frame from the GUI event to avoid double work.
                if is_gui_window {
                    self.render();
                }
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                if is_gui_window {
                    // Route keyboard events to imgui for the settings window.
                    if let Some(imgui_ctx) = &mut self.imgui_ctx {
                        let gui_win = self.gui.as_ref().unwrap();
                        imgui_ctx.handle_window_event(
                            gui_win.window(),
                            window_id,
                            &window_event,
                        );
                    }
                } else {
                    // Modifier / keyboard handling for the overlay egui context
                    // (used by keybind recording widgets etc.).
                    if let winit::keyboard::Key::Named(key) = event.logical_key {
                        let modifiers = match key {
                            NamedKey::Control => Some(egui::Modifiers::CTRL),
                            NamedKey::Shift => Some(egui::Modifiers::SHIFT),
                            NamedKey::Alt => Some(egui::Modifiers::ALT),
                            _ => None,
                        };

                        if let Some(modifiers) = modifiers {
                            self.gui.as_mut().unwrap().process_modifier(
                                modifiers,
                                event.state == ElementState::Pressed,
                                event.repeat,
                            );
                        }
                    }
                    let event_response = self.gui.as_mut().unwrap().process_event(&window_event);
                    if event_response.repaint {
                        self.gui.as_ref().unwrap().request_redraw();
                        self.overlay.as_ref().unwrap().request_redraw();
                    }
                }
            }
            // Manually inject Mouse4/Mouse5 into imgui IO — imgui_winit_support
            // may not forward Back/Forward button events.
            WindowEvent::MouseInput { state, button, .. }
                if is_gui_window
                    && matches!(
                        button,
                        winit::event::MouseButton::Back | winit::event::MouseButton::Forward
                    ) =>
            {
                if let Some(imgui_ctx) = &mut self.imgui_ctx {
                    let gui_win = self.gui.as_ref().unwrap();
                    // Let imgui_winit_support try first.
                    imgui_ctx.handle_window_event(gui_win.window(), window_id, &window_event);
                    // Then force the IO state in case it was ignored.
                    let pressed = *state == ElementState::Pressed;
                    let imgui_btn = match button {
                        winit::event::MouseButton::Back => imgui::MouseButton::Extra1,
                        winit::event::MouseButton::Forward => imgui::MouseButton::Extra2,
                        _ => unreachable!(),
                    };
                    imgui_ctx.context.io_mut().add_mouse_button_event(imgui_btn, pressed);
                }
            }
            _ => {
                if is_gui_window {
                    // Route all other GUI-window events to imgui.
                    if let Some(imgui_ctx) = &mut self.imgui_ctx {
                        let gui_win = self.gui.as_ref().unwrap();
                        imgui_ctx.handle_window_event(
                            gui_win.window(),
                            window_id,
                            &window_event,
                        );
                    }
                } else {
                    let event_response = self.gui.as_mut().unwrap().process_event(&window_event);
                    if event_response.repaint {
                        self.gui.as_ref().unwrap().request_redraw();
                        self.overlay.as_ref().unwrap().request_redraw();
                    }
                }
            }
        }
    }
}

fn frame_duration(config: &Config) -> Duration {
    let hz = config.hud.overlay_refresh_rate.clamp(30, 360);
    Duration::from_micros(1_000_000 / hz)
}
