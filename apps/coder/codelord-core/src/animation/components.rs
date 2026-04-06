//! Animation components for ECS entities.
//!
//! Generic animation components that work with any Interpolate type.
//! Uses `eazy` for easing functions.

use super::interpolate::Interpolate;

use bevy_ecs::component::Component;
use bevy_ecs::resource::Resource;
use eazy::interpolation::Interpolation;
use eazy::{Curve, Easing};

/// Generic animation component for any interpolatable value.
///
/// Animates from current value to target value over duration using easing.
#[derive(Component)]
pub struct Animatable<T: Interpolate + Send + Sync + 'static> {
  /// Current value.
  pub current: T,

  /// Target value (what we're animating towards).
  pub target: T,

  /// Elapsed time (in seconds).
  pub elapsed: f32,

  /// Total duration (in seconds).
  pub duration: f32,

  /// Easing function to use.
  pub easing: Easing,

  /// Whether animation is complete.
  pub is_complete: bool,
}

impl<T: Interpolate + Send + Sync + 'static> Animatable<T> {
  /// Create a new animation.
  pub fn new(current: T, target: T, duration: f32, easing: Easing) -> Self {
    Self {
      current,
      target,
      elapsed: 0.0,
      duration,
      easing,
      is_complete: false,
    }
  }

  /// Create with default easing (smoothstep).
  pub fn with_smoothstep(current: T, target: T, duration: f32) -> Self {
    Self::new(
      current,
      target,
      duration,
      Easing::Interpolation(Interpolation::InOutSmooth),
    )
  }

  /// Update animation by delta time.
  ///
  /// Returns true if animation just completed this frame.
  pub fn update(&mut self, delta: f32) -> bool {
    if self.is_complete {
      return false;
    }

    self.elapsed += delta;

    if self.elapsed >= self.duration {
      self.current = self.target.clone();
      self.is_complete = true;
      return true;
    }

    // Normalize time to 0.0-1.0
    let t = (self.elapsed / self.duration).clamp(0.0, 1.0);

    // Apply easing
    let eased_t = self.easing.y(t);

    // Interpolate
    self.current = self.current.lerp(&self.target, eased_t);

    false
  }

  /// Set new target (keeps current value, restarts animation).
  pub fn set_target(&mut self, new_target: T) {
    self.target = new_target;
    self.elapsed = 0.0;
    self.is_complete = false;
  }

  /// Get current interpolated value.
  pub fn value(&self) -> &T {
    &self.current
  }
}

/// Resource for tracking delta time and accumulated elapsed time.
#[derive(Resource)]
pub struct DeltaTime {
  /// Frame delta time (seconds).
  pub delta: f32,
  /// Accumulated elapsed time since app start (seconds).
  pub elapsed: f32,
}

impl Default for DeltaTime {
  fn default() -> Self {
    Self {
      delta: 0.016, // ~60 FPS
      elapsed: 0.0,
    }
  }
}

impl DeltaTime {
  /// Update with new frame delta.
  pub fn update(&mut self, delta: f32) {
    self.delta = delta;
    self.elapsed += delta;
  }

  /// Get frame delta time.
  pub fn delta(&self) -> f32 {
    self.delta
  }

  /// Get accumulated elapsed time.
  pub fn elapsed(&self) -> f32 {
    self.elapsed
  }
}
