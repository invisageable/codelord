use crate::animation::components::Animatable;

use bevy_ecs::component::Component;
use eazy::Easing;
use eazy::interpolation::Interpolation;

/// Counter indicator component.
///
/// Displays an animated numeric value with a unit label.
/// Examples: "1 items", "12 tokens", "5.750 μs"
#[derive(Component)]
pub struct Counter {
  pub animation: Animatable<f64>,
  pub unit: &'static str,
}

impl Counter {
  pub fn new(value: f64, unit: &'static str) -> Self {
    Self {
      animation: Animatable::new(
        value,
        value,
        0.3,
        Easing::Interpolation(Interpolation::InOutSmooth),
      ),
      unit,
    }
  }

  /// Set new target value (animates to it).
  pub fn set_value(&mut self, value: f64) {
    self.animation.set_target(value);
  }

  /// Get current animated value.
  pub fn value(&self) -> f64 {
    *self.animation.value()
  }

  /// Update animation.
  pub fn update(&mut self, delta: f32) {
    self.animation.update(delta);
  }
}
