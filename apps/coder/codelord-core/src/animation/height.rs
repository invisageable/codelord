//! Height animation for smooth panel transitions.

use eazy::{Curve, Easing};

/// Animates a height value smoothly between current and target.
#[derive(Debug, Clone)]
pub struct HeightAnimation {
  /// The current animated value.
  current: f32,
  /// The target value to animate towards.
  target: f32,
  /// The animation start time (seconds).
  start_time: f32,
  /// The animation duration (seconds).
  duration: f32,
  /// The initial value when animation started.
  start_value: f32,
}

impl HeightAnimation {
  /// Creates a new height animation starting at the given value.
  #[inline(always)]
  pub const fn new(initial_value: f32) -> Self {
    Self {
      current: initial_value,
      target: initial_value,
      start_time: 0.0,
      duration: 0.15,
      start_value: initial_value,
    }
  }

  /// Sets a new target value and starts animation.
  pub fn set_target(&mut self, target: f32, current_time: f32) {
    if (self.target - target).abs() > 0.001 {
      self.target = target;
      self.start_value = self.current;
      self.start_time = current_time;
    }
  }

  /// Updates the animation and returns the current value.
  pub fn update(&mut self, current_time: f32) -> f32 {
    let elapsed = current_time - self.start_time;

    if elapsed >= self.duration {
      self.current = self.target;
    } else {
      let t = (elapsed / self.duration).clamp(0.0, 1.0);
      let eased = Easing::InOutCubic.y(t);
      self.current =
        self.start_value + (self.target - self.start_value) * eased;
    }

    self.current
  }

  /// Gets the current value without updating.
  pub fn current_value(&self) -> f32 {
    self.current
  }

  /// Returns true if animation is still in progress.
  pub fn is_animating(&self) -> bool {
    (self.current - self.target).abs() > 0.001
  }
}

impl Default for HeightAnimation {
  fn default() -> Self {
    Self::new(40.0)
  }
}
