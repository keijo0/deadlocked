/// Adaptive frame skipping and frame-time tracking.
///
/// [`FrameTimer`] measures how long each render frame takes and exposes a
/// [`should_skip_frame`](FrameTimer::should_skip_frame) predicate.  When the
/// previous frame exceeded the configured budget by more than
/// [`SKIP_THRESHOLD`] the overlay paint pass is skipped, giving the GPU a full
/// extra budget period to catch up before the next submission.
///
/// # Typical usage
///
/// ```rust
/// let mut timer = FrameTimer::new(frame_budget);
/// // inside render():
/// timer.begin_frame();
/// // … paint gui …
/// if !timer.should_skip_frame() {
///     // … paint overlay …
/// }
/// timer.end_frame();
/// ```

use std::time::{Duration, Instant};

/// Fraction of the frame budget above which the next overlay pass is skipped.
const SKIP_THRESHOLD: f32 = 1.5;

/// Tracks per-frame render times and provides an adaptive skip hint.
pub struct FrameTimer {
    /// Target wall-clock time per frame.
    frame_budget: Duration,
    /// Timestamp recorded at the start of the current frame.
    frame_start: Option<Instant>,
    /// Duration of the most recently completed frame.
    last_frame_time: Duration,
}

impl FrameTimer {
    /// Create a new timer with the given frame budget.
    pub fn new(frame_budget: Duration) -> Self {
        Self {
            frame_budget,
            frame_start: None,
            last_frame_time: Duration::ZERO,
        }
    }

    /// Update the frame budget (e.g. after the user changes the refresh rate).
    pub fn update_budget(&mut self, frame_budget: Duration) {
        self.frame_budget = frame_budget;
    }

    /// Record the start of a new frame.  Call once at the very beginning of
    /// the render function.
    pub fn begin_frame(&mut self) {
        self.frame_start = Some(Instant::now());
    }

    /// Record the end of a frame and update the running frame-time.  Call once
    /// at the very end of the render function.
    pub fn end_frame(&mut self) {
        if let Some(start) = self.frame_start.take() {
            self.last_frame_time = start.elapsed();
        }
    }

    /// Returns `true` when the previous frame took longer than
    /// `frame_budget × SKIP_THRESHOLD`, indicating that the GPU is behind and
    /// the expensive overlay paint should be omitted this frame.
    pub fn should_skip_frame(&self) -> bool {
        self.last_frame_time > self.frame_budget.mul_f32(SKIP_THRESHOLD)
    }
}
