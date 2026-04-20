//! Magic zoom state.
//!
//! Three eased scalars: zoom factor and (x, y) camera center. All driven
//! by `eazy::Easing::InOutCubic` — matches the codelord animation house
//! style (see `animation/height.rs`, `xmb/resources.rs`).

use bevy_ecs::resource::Resource;
use eazy::{Curve, Easing};

/// Target zoom factor when engaged (Screen Studio uses ~2x).
const ZOOM_ACTIVE: f32 = 2.0;
/// Target zoom factor when idle.
const ZOOM_IDLE: f32 = 1.0;
/// Duration of the zoom-in/out ease (seconds).
const ZOOM_DURATION: f32 = 0.30;
/// Duration of the cursor-follow ease (seconds).
///
/// Shorter than `ZOOM_DURATION` so the center catches the cursor faster
/// than the zoom settles — feels anchored, not laggy.
const FOLLOW_DURATION: f32 = 0.18;
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
    if (self.target - target).abs() < EPSILON {
      return;
    }

    self.start = self.current;
    self.target = target;
    self.elapsed = 0.0;
  }

  fn tick(&mut self, dt: f32) {
    if self.elapsed >= self.duration {
      self.current = self.target;

      return;
    }

    self.elapsed = (self.elapsed + dt).min(self.duration);

    let t = self.elapsed / self.duration;
    let eased = Easing::InOutCubic.y(t);

    self.current = self.start + (self.target - self.start) * eased;
  }

  #[inline(always)]
  const fn is_settled(&self) -> bool {
    self.elapsed >= self.duration
  }
}

/// Camera state for the Screen-Studio-style magic zoom effect.
#[derive(Resource, Debug, Clone)]
pub struct MagicZoomState {
  /// True while the hotkey is held.
  pub engaged: bool,
  zoom: Eased,
  center_x: Eased,
  center_y: Eased,
}

impl Default for MagicZoomState {
  fn default() -> Self {
    Self {
      engaged: false,
      zoom: Eased::new(ZOOM_IDLE, ZOOM_DURATION),
      center_x: Eased::new(0.0, FOLLOW_DURATION),
      center_y: Eased::new(0.0, FOLLOW_DURATION),
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

  /// Advance all eased scalars by `dt` seconds.
  pub fn tick(&mut self, dt: f32) {
    self.zoom.tick(dt);
    self.center_x.tick(dt);
    self.center_y.tick(dt);
  }
}
