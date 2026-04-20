//! Magic zoom state.
//!
//! Two smoothing models, one per use case:
//! - Zoom factor: duration-based `eazy::Easing::InOutCubic` (discrete
//!   engage/disengage event, like `animation/height.rs`).
//! - Camera center: exponential smoothing (continuous cursor tracking —
//!   duration-based easing would reset on every retarget and lag forever behind
//!   a moving cursor).

use bevy_ecs::resource::Resource;
use eazy::{Curve, Easing};

/// Target zoom factor when engaged (Screen Studio uses ~2x).
const ZOOM_ACTIVE: f32 = 2.0;
/// Target zoom factor when idle.
const ZOOM_IDLE: f32 = 1.0;
/// Duration of the zoom-in/out ease (seconds).
const ZOOM_DURATION: f32 = 0.30;
/// Time constant of the cursor-follow exponential smoother (seconds).
///
/// Each frame closes `1 - exp(-dt/tau)` of the gap to target. After `3*tau`
/// we've closed ~95%. Chosen by feel: 0.08s is snappy enough to stay with
/// the cursor, smooth enough to not feel jittery. Screen-Studio-like.
const FOLLOW_TAU: f32 = 0.08;
/// Completion epsilon for both zoom and center.
const EPSILON: f32 = 0.001;

/// A single scalar smoothly eased toward a target.
#[derive(Debug, Clone, Copy)]
struct Eased {
  current: f32,
  start: f32,
  target: f32,
  elapsed: f32,
  duration: f32,
}

impl Eased {
  const fn new(value: f32, duration: f32) -> Self {
    Self {
      current: value,
      start: value,
      target: value,
      elapsed: duration,
      duration,
    }
  }

  fn retarget(&mut self, target: f32) {
    // Skip if already heading there: resetting `elapsed` to 0 would cause
    // a visible pause/restart on the next tick.
    if (self.target - target).abs() < EPSILON {
      return;
    }

    self.start = self.current;
    self.target = target;
    self.elapsed = 0.0;
  }

  fn tick(&mut self, delta: f32) {
    if self.elapsed >= self.duration {
      self.current = self.target;

      return;
    }

    self.elapsed = (self.elapsed + delta).min(self.duration);

    let progress = self.elapsed / self.duration;
    let eased = Easing::InOutCubic.y(progress);

    self.current = self.start + (self.target - self.start) * eased;
  }

  #[inline(always)]
  const fn is_settled(&self) -> bool {
    self.elapsed >= self.duration
  }
}

/// Exponentially smoothed scalar. Approaches target asymptotically with
/// time constant `tau`. Ideal for continuous cursor tracking: retargeting
/// every frame is cheap and the follower never "restarts" — it always
/// chases the current target with frame-rate-independent motion.
#[derive(Debug, Clone, Copy)]
struct Smoothed {
  current: f32,
  target: f32,
  tau: f32,
}

impl Smoothed {
  const fn new(value: f32, tau: f32) -> Self {
    Self {
      current: value,
      target: value,
      tau,
    }
  }

  fn retarget(&mut self, target: f32) {
    self.target = target;
  }

  fn tick(&mut self, delta: f32) {
    let gap = self.target - self.current;

    // Snap to target once within epsilon: exponential approach is
    // asymptotic, so without this we'd accumulate sub-pixel drift forever.
    if gap.abs() < EPSILON {
      self.current = self.target;

      return;
    }

    // Frame-rate-independent exponential smoothing.
    let alpha = 1.0 - (-delta / self.tau).exp();

    self.current += gap * alpha;
  }

  #[inline(always)]
  fn is_settled(&self) -> bool {
    (self.current - self.target).abs() < EPSILON
  }
}

/// Camera state for the Screen-Studio-style magic zoom effect.
///
/// Zoom uses duration-based easing (discrete engage/disengage event).
/// Center uses exponential smoothing (continuous cursor tracking — a
/// duration ease would reset every retarget and fall permanently behind).
#[derive(Resource, Debug, Clone)]
pub struct MagicZoomState {
  /// True while the hotkey is held.
  pub engaged: bool,
  zoom: Eased,
  center_x: Smoothed,
  center_y: Smoothed,
}

impl Default for MagicZoomState {
  fn default() -> Self {
    Self {
      engaged: false,
      zoom: Eased::new(ZOOM_IDLE, ZOOM_DURATION),
      center_x: Smoothed::new(0.0, FOLLOW_TAU),
      center_y: Smoothed::new(0.0, FOLLOW_TAU),
    }
  }
}

impl MagicZoomState {
  /// Current eased zoom factor.
  #[inline(always)]
  pub fn zoom(&self) -> f32 {
    self.zoom.current
  }

  /// Current eased (x, y) camera center in screen-space pixels.
  #[inline(always)]
  pub fn center(&self) -> (f32, f32) {
    (self.center_x.current, self.center_y.current)
  }

  /// True if any eased scalar is still in motion. Callers can use this to
  /// skip the render-time transform path when fully idle.
  pub fn is_animating(&self) -> bool {
    (self.zoom.current - ZOOM_IDLE).abs() > EPSILON
      || !self.zoom.is_settled()
      || !self.center_x.is_settled()
      || !self.center_y.is_settled()
  }

  /// Retarget the zoom factor based on engaged state.
  pub fn retarget_zoom(&mut self, engaged: bool) {
    self.engaged = engaged;
    self
      .zoom
      .retarget(if engaged { ZOOM_ACTIVE } else { ZOOM_IDLE });
  }

  /// Retarget the camera center (typically to the current cursor pos).
  pub fn retarget_center(&mut self, x: f32, y: f32) {
    self.center_x.retarget(x);
    self.center_y.retarget(y);
  }

  /// Advance all eased scalars by `delta` seconds.
  pub fn tick(&mut self, delta: f32) {
    self.zoom.tick(delta);
    self.center_x.tick(delta);
    self.center_y.tick(delta);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Time-warp helper: advance by one full zoom duration in small steps so
  /// we exercise the easing curve rather than teleporting in a single tick.
  fn warp(state: &mut MagicZoomState, total: f32, steps: u32) {
    let delta = total / steps as f32;

    for _ in 0..steps {
      state.tick(delta);
    }
  }

  #[test]
  fn eased_starts_at_initial_value() {
    let eased = Eased::new(1.0, 0.3);

    assert_eq!(eased.current, 1.0);
    assert!(eased.is_settled());
  }

  #[test]
  fn eased_reaches_target_after_full_duration() {
    let mut eased = Eased::new(1.0, 0.3);
    eased.retarget(2.0);
    eased.tick(0.3);

    assert!((eased.current - 2.0).abs() < EPSILON);
    assert!(eased.is_settled());
  }

  #[test]
  fn eased_interpolates_mid_duration() {
    let mut eased = Eased::new(0.0, 1.0);
    eased.retarget(1.0);
    eased.tick(0.5);

    // InOutCubic at t=0.5 is exactly 0.5 by symmetry.
    assert!((eased.current - 0.5).abs() < 0.01);
  }

  #[test]
  fn eased_retarget_to_same_value_is_noop() {
    let mut eased = Eased::new(1.0, 0.3);
    eased.retarget(1.0);

    // Still settled; elapsed untouched.
    assert!(eased.is_settled());
    assert_eq!(eased.current, 1.0);
  }

  #[test]
  fn eased_retarget_restarts_from_current_not_start() {
    let mut eased = Eased::new(0.0, 1.0);
    eased.retarget(1.0);
    eased.tick(0.5);
    let mid = eased.current;

    // Reverse direction mid-animation; new start must be `mid`, not 0.
    eased.retarget(0.0);
    assert_eq!(eased.start, mid);
    assert_eq!(eased.target, 0.0);
    assert_eq!(eased.elapsed, 0.0);
  }

  #[test]
  fn zoom_state_defaults_to_idle() {
    let state = MagicZoomState::default();

    assert!(!state.engaged);
    assert_eq!(state.zoom(), 1.0);
    assert!(!state.is_animating());
  }

  #[test]
  fn zoom_state_engages_toward_active() {
    let mut state = MagicZoomState::default();
    state.retarget_zoom(true);

    assert!(state.engaged);
    assert!(state.is_animating());

    // Warp past the zoom duration; must land on ZOOM_ACTIVE.
    warp(&mut state, ZOOM_DURATION + 0.01, 16);
    assert!((state.zoom() - ZOOM_ACTIVE).abs() < EPSILON);
  }

  #[test]
  fn zoom_state_disengage_returns_to_idle() {
    let mut state = MagicZoomState::default();
    state.retarget_zoom(true);
    warp(&mut state, ZOOM_DURATION + 0.01, 16);

    state.retarget_zoom(false);
    warp(&mut state, ZOOM_DURATION + 0.01, 16);

    assert!(!state.engaged);
    assert!((state.zoom() - ZOOM_IDLE).abs() < EPSILON);
  }

  #[test]
  fn zoom_state_center_follows_retarget() {
    let mut state = MagicZoomState::default();
    state.retarget_center(100.0, 200.0);

    // Exponential smoothing is asymptotic. For initial gap of ~200 to drop
    // below EPSILON (0.001), need exp(-t/tau) < 5e-6 → t > tau * ln(2e5) ≈
    // tau * 12.2. Warp for 15*tau to be safe.
    warp(&mut state, FOLLOW_TAU * 15.0, 32);

    let (cx, cy) = state.center();

    assert!((cx - 100.0).abs() < EPSILON);
    assert!((cy - 200.0).abs() < EPSILON);
  }

  #[test]
  fn zoom_state_is_animating_during_motion() {
    let mut state = MagicZoomState::default();
    state.retarget_zoom(true);

    // Midway through the zoom ease.
    warp(&mut state, ZOOM_DURATION * 0.5, 8);
    assert!(state.is_animating());
    assert!(state.zoom() > 1.0 && state.zoom() < ZOOM_ACTIVE);
  }

  #[test]
  fn zoom_state_center_tracks_moving_cursor() {
    // Simulates the held-key use case under sustained cursor motion. With
    // exponential smoothing, the lag reaches a bounded steady state
    // (velocity * tau ≈ gap), not an unbounded drift.
    //
    // Velocity: 2 px/frame * 60 fps = 120 px/s. Steady-state gap ≈
    // 120 * FOLLOW_TAU = 9.6 px. Assert the center stays within a loose
    // bound during motion to prove we're tracking, not diverging.
    let mut state = MagicZoomState::default();
    let delta = 1.0 / 60.0;

    for frame in 0..60 {
      let x = frame as f32 * 2.0;

      state.retarget_center(x, 0.0);
      state.tick(delta);
    }

    let (cx, _cy) = state.center();
    let final_target = 59.0 * 2.0;

    // Tracking: within ~2x the theoretical steady-state gap.
    assert!(
      (cx - final_target).abs() < 20.0,
      "cx = {cx}, target = {final_target}",
    );
  }

  #[test]
  fn zoom_state_center_settles_when_cursor_stops() {
    // Phase 1: 30 frames of moving cursor. Phase 2: cursor stops, warp
    // enough for exponential to converge.
    let mut state = MagicZoomState::default();
    let delta = 1.0 / 60.0;

    for frame in 0..30 {
      let x = frame as f32 * 2.0;
      state.retarget_center(x, 0.0);
      state.tick(delta);
    }
    let final_target = 29.0 * 2.0;

    warp(&mut state, FOLLOW_TAU * 15.0, 32);

    let (cx, _cy) = state.center();
    assert!((cx - final_target).abs() < EPSILON, "cx = {cx}");
  }
}
