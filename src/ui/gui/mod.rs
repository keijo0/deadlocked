use utils::log;

use crate::{
    config::write_config,
    message::Message,
    ui::{app::App, color::ColorScheme},
};

impl App {
    pub fn send_config(&self) {
        self.send_message(Message::Config(Box::new(self.config.clone())));
        self.save();
    }

    pub fn send_message(&self, message: Message) {
        if self.channel.send(message).is_err() {
            std::process::exit(1);
        }
    }

    fn save(&self) {
        write_config(&self.config, &self.current_config);
    }

    pub fn render(&mut self) {
        self.frame_timer.begin_frame();

        // Re-apply the color scheme to the overlay egui context whenever the
        // accent style changes.
        let current_style = self.config.accent_style.clone();
        if current_style != self.last_applied_style {
            let scheme = ColorScheme::for_style(&current_style);
            if let Some(overlay_win) = &self.overlay {
                overlay_win.apply_style(&scheme);
            }
            self.last_applied_style = current_style;
        }

        let self_ptr = self as *mut Self;

        let gui = self.gui.as_mut().unwrap();

        if let Err(err) = gui.make_current() {
            log::error!("could not make gui window current: {err}");
            self.frame_timer.end_frame();
            return;
        }

        // ── imgui GUI rendering ───────────────────────────────────────────
        //
        // Phase 1: build the imgui frame (must happen before clear so that
        // prepare_frame can sync display_size from the window).
        let config_changed = {
            let app = unsafe { &mut *self_ptr };
            if let Some(imgui_ctx) = &mut app.imgui_ctx {
                let now = std::time::Instant::now();
                imgui_ctx
                    .context
                    .io_mut()
                    .update_delta_time(now.duration_since(imgui_ctx.last_frame));
                imgui_ctx.last_frame = now;

                imgui_ctx
                    .platform
                    .prepare_frame(imgui_ctx.context.io_mut(), gui.window())
                    .expect("imgui prepare_frame failed");

                // Apply UI scale + gui_font_size so font tracks preferences.
                let ui_scale = imgui_ctx.cfg_panel.ui_scale();
                let gui_font_size = unsafe { (*self_ptr).config.hud.gui_font_size };
                imgui_ctx.context.io_mut().font_global_scale = ui_scale * (gui_font_size / 13.0);

                // Re-apply imgui color scheme whenever AccentStyle changes.
                let accent = unsafe { (*self_ptr).config.accent_style.clone() };
                imgui_ctx.apply_accent_if_changed(&accent);

                // Build the UI — access app fields through self_ptr to work
                // around the borrow checker while imgui_ctx is mutably borrowed.
                let mut config_snapshot = unsafe { (*self_ptr).config.clone() };
                let game_status = unsafe { (*self_ptr).game_status.clone() };
                let data_arc = unsafe { (*self_ptr).data.clone() };

                let ui = imgui_ctx.context.new_frame();

                let changed = crate::ui::imgui_backend::panels::MainMenu::render(
                    ui,
                    &mut config_snapshot,
                    game_status,
                    &data_arc,
                    &mut imgui_ctx.esp,
                    &mut imgui_ctx.aimbot,
                    &mut imgui_ctx.hud,
                    &mut imgui_ctx.misc,
                    &mut imgui_ctx.cfg_panel,
                    &mut imgui_ctx.grenades,
                );

                imgui_ctx.platform.prepare_render(ui, gui.window());

                if changed {
                    unsafe { (*self_ptr).config = config_snapshot };
                }
                changed
            } else {
                false
            }
        };

        // Phase 2: clear framebuffer then render imgui draw lists to GL.
        gui.clear();

        {
            let app = unsafe { &mut *self_ptr };
            if let Some(imgui_ctx) = &mut app.imgui_ctx {
                let draw_data = imgui_ctx.context.render();
                if let Err(e) = imgui_ctx.renderer.render(gui.glow(), draw_data) {
                    log::error!("imgui render error: {e}");
                }
            }
        }

        if config_changed {
            // Sync the active config path from the config panel before saving
            // so that autosave writes to the file the user actually selected,
            // not always the default config.
            {
                let app = unsafe { &mut *self_ptr };
                if let Some(ctx) = &app.imgui_ctx {
                    app.current_config = ctx.cfg_panel.current_config.clone();
                }
            }
            unsafe { &mut *self_ptr }.send_config();
        }

        // Resize the OS window when the user explicitly changes W/H in the
        // Config tab.  We only fire once per committed change (flag is set by
        // the config panel and cleared here) so the WM is never fought on
        // every frame.
        {
            let app = unsafe { &mut *self_ptr };
            if let Some(ctx) = &mut app.imgui_ctx {
                if ctx.cfg_panel.resize_requested {
                    ctx.cfg_panel.resize_requested = false;
                    let cp = &ctx.cfg_panel;
                    let needed_w = cp.menu_width;
                    let needed_h = cp.menu_height;
                    let _ = gui.window().request_inner_size(
                        winit::dpi::LogicalSize::new(needed_w, needed_h),
                    );
                }
            }
        }

        if let Err(err) = gui.swap_buffers() {
            log::error!("could not swap gui window buffers: {err}");
            self.frame_timer.end_frame();
            return;
        }

        // ── Overlay (egui) rendering — unchanged ─────────────────────────
        if !self.frame_timer.should_skip_frame() {
            let overlay = self.overlay.as_mut().unwrap();

            overlay.window().set_cursor_hittest(false).unwrap();
            if let Err(err) = overlay.make_current() {
                log::error!("could not make overlay window current: {err}");
                self.frame_timer.end_frame();
                return;
            }

            overlay.run(move |ui| {
                (unsafe { &mut *self_ptr }).overlay(ui);
            });
            overlay.raise_to_top();
            overlay.clear();
            overlay.paint();

            if let Err(err) = overlay.swap_buffers() {
                log::error!("could not swap overlay window buffers: {err}");
            }
        }

        self.frame_timer.end_frame();
    }
}

