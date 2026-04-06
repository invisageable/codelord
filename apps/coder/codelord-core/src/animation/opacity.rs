//! Opacity fade animation.
//!
//! Simple fade-in effect for smooth element appearance.

/// Opacity animation state for fade effects.
#[derive(Debug, Clone)]
pub struct OpacityAnimation {
  /// Current opacity (0.0 to 1.0).
  opacity: f32,
  /// Target opacity.
  target: f32,
  /// Animation speed multiplier.
  speed: f32,
  /// Whether animation is finished.
  finished: bool,
}

impl Default for OpacityAnimation {
  fn default() -> Self {
    Self::fade_in()
  }
}

impl OpacityAnimation {
  /// Creates a fade-in animation (0 -> 1).
  pub fn fade_in() -> Self {
    Self {
      opacity: 0.0,
      target: 1.0,
      speed: 3.0,
      finished: false,
    }
  }

  /// Creates a fade-out animation (1 -> 0).
  pub fn fade_out() -> Self {
    Self {
      opacity: 1.0,
      target: 0.0,
      speed: 3.0,
      finished: false,
    }
  }

  /// Creates with custom speed.
  pub fn with_speed(mut self, speed: f32) -> Self {
    self.speed = speed;
    self
  }

  /// Returns current opacity value (0.0 to 1.0).
  pub fn opacity(&self) -> f32 {
    self.opacity
  }

  /// Returns whether animation is finished.
  pub fn is_finished(&self) -> bool {
    self.finished
  }

  /// Resets animation to fade-in from zero.
  pub fn reset(&mut self) {
    self.opacity = 0.0;
    self.target = 1.0;
    self.finished = false;
  }

  /// Updates the animation. Returns true if still animating.
  pub fn update(&mut self, dt: f32) -> bool {
    if self.finished {
      return false;
    }

    let diff = self.target - self.opacity;
    let delta = diff * self.speed * dt;

    self.opacity += delta;

    // Snap to target when close enough
    if (self.target - self.opacity).abs() < 0.01 {
      self.opacity = self.target;
      self.finished = true;

      return false;
    }

    true
  }
}
