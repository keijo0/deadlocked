use std::{
    sync::Arc,
    thread::{self, sleep},
    time::{Duration, Instant},
};

use utils::{channel::Channel, log, sync::Mutex};

use crate::{
    config::{
        AntiAfk, CONFIG_PATH, Config, DEFAULT_CONFIG_NAME, LOOP_DURATION, SLEEP_DURATION,
        parse_config,
    },
    cs2::CS2,
    data::Data,
    message::{GameStatus, Message},
    os::mouse::Mouse,
};

pub trait Game: std::fmt::Debug {
    fn is_valid(&self) -> bool;
    fn setup(&mut self);
    fn run(&mut self, config: &Config, mouse: &mut Mouse);
    fn data(&self, config: &Config, data: &mut Data);
}

pub struct GameManager {
    channel: Channel<Message>,
    data: Arc<Mutex<Data>>,
    config: Config,
    mouse: Mouse,
    game: CS2,
    antiafk_config: Arc<Mutex<AntiAfk>>,
}

impl GameManager {
    pub fn new(channel: Channel<Message>, data: Arc<Mutex<Data>>) -> Self {
        let mouse = match Mouse::open() {
            Ok(mouse) => mouse,
            Err(err) => {
                log::error!("error creating uinput device: {err}");
                log::error!("uinput kernel module is not loaded, or user is not in input group.");
                std::process::exit(1);
            }
        };

        let mut game = Self {
            channel,
            data,
            config: Config::default(),
            mouse,
            game: CS2::new(),
            antiafk_config: Arc::new(Mutex::new(AntiAfk::default())),
        };

        let config_path = CONFIG_PATH.join(DEFAULT_CONFIG_NAME);
        if config_path.exists() {
            game.config = parse_config(&config_path);
            *game.antiafk_config.lock() = game.config.misc.antiafk.clone();
        }

        let antiafk_config = game.antiafk_config.clone();
        thread::spawn(move || {
            run_antiafk_loop(antiafk_config);
        });

        game
    }

    fn send_game_message(&self, message: Message) {
        if self.channel.send(message).is_err() {
            std::process::exit(1);
        }
    }

    pub fn run(&mut self) {
        self.send_game_message(Message::GameStatus(GameStatus::NotStarted));
        let mut previous_status = GameStatus::NotStarted;
        loop {
            let start = Instant::now();
            while let Ok(message) = self.channel.try_receive() {
                self.parse_message(message);
            }

            let mut is_valid = self.game.is_valid();
            if !is_valid {
                if previous_status == GameStatus::Working {
                    self.send_game_message(Message::GameStatus(GameStatus::NotStarted));
                    previous_status = GameStatus::NotStarted;
                }
                self.game.setup();
                is_valid = self.game.is_valid();
            }

            if is_valid {
                if previous_status == GameStatus::NotStarted {
                    self.send_game_message(Message::GameStatus(GameStatus::Working));
                    previous_status = GameStatus::Working;
                }
                self.game.run(&self.config, &mut self.mouse);
                let mut data = self.data.lock();
                self.game.data(&self.config, &mut data);
            } else {
                *self.data.lock() = Data::default();
            }

            if is_valid {
                let elapsed = start.elapsed();
                if elapsed < LOOP_DURATION {
                    sleep(LOOP_DURATION - elapsed);
                } else {
                    log::debug!(
                        "game loop took {} ms (max {} ms)",
                        elapsed.as_millis(),
                        LOOP_DURATION.as_millis()
                    );
                    sleep(LOOP_DURATION);
                }
            } else {
                sleep(SLEEP_DURATION);
            }
        }
    }

    fn parse_message(&mut self, message: Message) {
        if let Message::Config(config) = message {
            *self.antiafk_config.lock() = config.misc.antiafk.clone();
            self.config = *config;
        }
    }
}

fn run_antiafk_loop(config: Arc<Mutex<AntiAfk>>) {
    let mut last_action = Instant::now();
    // Seed with current time for non-deterministic output
    let mut rng_state: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ (d.as_secs().wrapping_mul(6364136223846793005)))
        .unwrap_or(12345);

    loop {
        let cfg = config.lock().clone();

        if !cfg.enabled {
            drop(cfg);
            sleep(Duration::from_secs(1));
            last_action = Instant::now();
            continue;
        }

        let interval_min = cfg.interval_min.max(1.0) as u64;
        let interval_max = cfg.interval_max.max(cfg.interval_min).max(1.0) as u64;
        let walk_bot = cfg.walk_bot;
        drop(cfg);

        if last_action.elapsed() >= Duration::from_secs(interval_min) {
            let dx = lcg_rand(&mut rng_state, 11) as i32 - 5;
            let dy = lcg_rand(&mut rng_state, 11) as i32 - 5;

            match std::process::Command::new("xdotool")
                .args(["mousemove_relative", "--", &dx.to_string(), &dy.to_string()])
                .spawn()
            {
                Ok(mut child) => {
                    let _ = child.wait();
                }
                Err(err) => log::warn!("anti-afk: failed to run xdotool: {err}"),
            }

            if walk_bot {
                const WASD: [&str; 4] = ["w", "a", "s", "d"];
                let key = WASD[lcg_rand(&mut rng_state, 4) as usize];
                // hold duration: 300-800 ms
                let hold_ms = 300 + lcg_rand(&mut rng_state, 501);

                match std::process::Command::new("xdotool")
                    .args(["keydown", "--clearmodifiers", key])
                    .spawn()
                {
                    Ok(mut c) => {
                        let _ = c.wait();
                    }
                    Err(err) => log::warn!("anti-afk walk-bot: keydown failed: {err}"),
                }
                sleep(Duration::from_millis(hold_ms));
                match std::process::Command::new("xdotool")
                    .args(["keyup", "--clearmodifiers", key])
                    .spawn()
                {
                    Ok(mut c) => {
                        let _ = c.wait();
                    }
                    Err(err) => log::warn!("anti-afk walk-bot: keyup failed: {err}"),
                }
            }

            let sleep_secs = interval_min
                + lcg_rand(&mut rng_state, (interval_max - interval_min + 1).max(1));
            last_action = Instant::now();
            sleep(Duration::from_secs(sleep_secs));
        } else {
            sleep(Duration::from_millis(100));
        }
    }
}

/// Simple LCG pseudo-random number generator returning a value in [0, max).
fn lcg_rand(state: &mut u64, max: u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*state >> 33) % max
}
