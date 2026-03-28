use std::sync::Arc;

use utils::{
    channel::Channel,
    log::{self, Logger, LoggerOptions},
    sync::Mutex,
};

use crate::{data::Data, os::mouse::check_uinput, parser::parse_maps, ui::app::App};

mod config;
mod constants;
mod cs2;
mod data;
mod game;
mod math;
mod message;
mod os;
mod parser;
mod ui;

#[cfg(not(target_os = "linux"))]
compile_error!("only linux is supported.");

fn main() {
    Logger::install(
        LoggerOptions::default()
            .file("deadlocked.log")
            .debug(true)
            .truncate(true)
            .module(module_path!()),
    );

    let args: Vec<String> = std::env::args().collect();

    if !check_uinput() {
        return;
    }

    // this runs as x11 for now, because wayland decorations for winit are not good
    // and don't support disabling the maximize button
    unsafe { std::env::remove_var("WAYLAND_DISPLAY") };

    let force_reparse = args.iter().any(|arg| arg == "--force-reparse");
    let use_system_binary = args.iter().any(|arg| arg == "--local-s2v");
    std::thread::spawn(move || {
        parse_maps(force_reparse, use_system_binary);
    });

    let (channel_gui, channel_game) = Channel::new();
    let data = Arc::new(Mutex::new(Data::default()));
    let data_game = data.clone();

    std::thread::spawn(move || {
        game::GameManager::new(channel_game, data_game).run();
    });

    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            log::error!("failed to create event loop: {err}");
            return;
        }
    };
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::new(channel_gui, data);
    event_loop.run_app(&mut app).unwrap();
}
