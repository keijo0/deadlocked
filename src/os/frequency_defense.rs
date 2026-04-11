/// Syscall-frequency monitoring and adaptive throttling.
///
/// [`FrequencyDefense`] tracks the rate at which `process_vm_readv` (or any
/// other syscall) is issued and can detect anomalously high observation rates
/// that may indicate the process is being profiled.
///
/// When a suspicious rate is detected the caller can:
/// 1. Increase inter-read jitter (see [`StealthReader`]).
/// 2. Temporarily pause data collection.
/// 3. Switch to a slower but less conspicuous read method.
///
/// [`StealthReader`]: super::stealth_reader::StealthReader

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use utils::log;

/// Default threshold: more than this many syscalls per second is considered
/// suspicious.
///
/// Each read call issues one `process_vm_readv` syscall and increments this
/// counter.  At 64 Hz with 64 players and ~100 reads per player per frame the
/// sustained rate can reach ~400 000–600 000/s during normal ESP operation, so
/// the threshold must sit well above that.  2 000 000 (2M) is comfortably
/// above any realistic ESP workload; only a pathological read loop caused by a
/// bug could approach that level.
const DEFAULT_SUSPICIOUS_RATE: u64 = 2_000_000;

/// How long to stay in the throttled state before rechecking.
const THROTTLE_DURATION: Duration = Duration::from_secs(2);

/// Current operational state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefenseState {
    /// Normal operation; no suspicious activity detected.
    Normal,
    /// Suspicious syscall rate detected; reads are throttled.
    Throttled,
}

/// Monitors syscall rate and provides adaptive throttling recommendations.
pub struct FrequencyDefense {
    suspicious_rate: u64,
    /// Atomic counter incremented by the caller for each syscall issued.
    /// Wrapped in `Arc` so the same counter can be shared with [`StealthReader`].
    counter: Arc<AtomicU64>,
    window_start: Instant,
    state: DefenseState,
    throttle_until: Option<Instant>,
}

impl FrequencyDefense {
    /// Create a new monitor with the default suspicious-rate threshold.
    pub fn new() -> Self {
        Self::with_threshold(DEFAULT_SUSPICIOUS_RATE)
    }

    /// Create a new monitor with a custom threshold (syscalls/second).
    pub fn with_threshold(suspicious_rate: u64) -> Self {
        Self {
            suspicious_rate,
            counter: Arc::new(AtomicU64::new(0)),
            window_start: Instant::now(),
            state: DefenseState::Normal,
            throttle_until: None,
        }
    }

    /// Return a shared handle to the syscall counter so that [`StealthReader`]
    /// can increment it directly after each read without needing a mutable
    /// reference to the full `FrequencyDefense` struct.
    pub fn shared_counter(&self) -> Arc<AtomicU64> {
        self.counter.clone()
    }

    /// Evaluate the current rate and update state.
    ///
    /// Returns the current [`DefenseState`]. Should be called once per game
    /// loop iteration (not once per syscall).
    pub fn evaluate(&mut self) -> DefenseState {
        let now = Instant::now();

        // If currently throttled, check whether the window has passed.
        if let Some(until) = self.throttle_until {
            if now < until {
                return DefenseState::Throttled;
            }
            self.throttle_until = None;
            self.state = DefenseState::Normal;
            log::debug!("rate limiter: throttle expired, resuming");
        }

        let elapsed = now.duration_since(self.window_start);
        if elapsed < Duration::from_secs(1) {
            return self.state;
        }

        // Reset measurement window.
        let count = self.counter.swap(0, Ordering::Relaxed);
        let secs = elapsed.as_secs_f64().max(0.001);
        let rate = (count as f64 / secs) as u64;
        self.window_start = now;

        log::debug!("rate limiter: {rate}/sec (threshold {})", self.suspicious_rate);

        if rate > self.suspicious_rate {
            log::warn!(
                "rate limiter: suspicious rate ({rate}/s), throttling for {}s",
                THROTTLE_DURATION.as_secs()
            );
            self.state = DefenseState::Throttled;
            self.throttle_until = Some(now + THROTTLE_DURATION);
        } else {
            self.state = DefenseState::Normal;
        }

        self.state
    }

    /// Return the extra sleep duration a caller should insert when throttled.
    ///
    /// Returns `Duration::ZERO` in the `Normal` state.
    pub fn throttle_delay(&self) -> Duration {
        match self.state {
            DefenseState::Normal => Duration::ZERO,
            DefenseState::Throttled => Duration::from_millis(50),
        }
    }

    /// Reset the monitor (e.g. after re-attaching to a new process).
    pub fn reset(&mut self) {
        self.counter.store(0, Ordering::Relaxed);
        self.window_start = Instant::now();
        self.state = DefenseState::Normal;
        self.throttle_until = None;
    }
}

impl std::fmt::Debug for FrequencyDefense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrequencyDefense")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Default for FrequencyDefense {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_normal() {
        let mut fd = FrequencyDefense::new();
        assert_eq!(fd.evaluate(), DefenseState::Normal);
    }

    #[test]
    fn throttle_delay_zero_when_normal() {
        let fd = FrequencyDefense::new();
        assert_eq!(fd.throttle_delay(), Duration::ZERO);
    }

    #[test]
    fn shared_counter_increments_correctly() {
        let fd = FrequencyDefense::new();
        let counter = fd.shared_counter();
        counter.fetch_add(1, Ordering::Relaxed);
        counter.fetch_add(1, Ordering::Relaxed);
        assert_eq!(fd.counter.load(Ordering::Relaxed), 2);
    }
}
