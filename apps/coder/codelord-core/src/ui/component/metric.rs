//! Metric indicator component.
//!
//! Displays a label with info icon and a counter value with unit.
//! Clicking the info icon shows a popup with additional information.

use crate::animation::components::Animatable;

use eazy::Easing;
use eazy::interpolation::Interpolation;

use bevy_ecs::component::Component;

/// Unit display mode for metric.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MetricUnit {
  /// Static unit string.
  #[default]
  Static,
  /// Dynamic time unit (μs, ms, s) based on value in ms.
  Time,
}

/// Metric indicator component.
///
/// Displays a label, info popup, and animated counter value with unit.
/// Examples: "Output" with "33015 bytes"
#[derive(Component)]
pub struct Metric {
  /// Label text displayed at the top.
  pub label: &'static str,
  /// Popup content shown when clicking the info icon.
  pub info: &'static str,
  /// Animated counter value.
  pub animation: Animatable<f64>,
  /// Unit displayed after the value (for Static mode).
  pub unit: &'static str,
  /// Unit display mode.
  pub unit_mode: MetricUnit,
  /// Color for label, value, and unit (RGBA).
  pub color: [u8; 4],
  /// Display as integer (no decimal places).
  pub is_integer: bool,
}

impl Metric {
  /// Creates a new metric indicator (integer display, static unit).
  pub fn new(
    label: &'static str,
    info: &'static str,
    value: f64,
    unit: &'static str,
    color: [u8; 4],
  ) -> Self {
    Self {
      label,
      info,
      animation: Animatable::new(
        value,
        value,
        0.3,
        Easing::Interpolation(Interpolation::InOutSmooth),
      ),
      unit,
      unit_mode: MetricUnit::Static,
      color,
      is_integer: true,
    }
  }

  /// Creates a new metric indicator with dynamic time unit (μs, ms, s).
  pub fn new_time(
    label: &'static str,
    info: &'static str,
    value: f64,
    color: [u8; 4],
  ) -> Self {
    Self {
      label,
      info,
      animation: Animatable::new(
        value,
        value,
        0.3,
        Easing::Interpolation(Interpolation::InOutSmooth),
      ),
      unit: "ms",
      unit_mode: MetricUnit::Time,
      color,
      is_integer: false,
    }
  }

  /// Set new target value (animates to it).
  ///
  /// Returns `true` if animation was started (wasn't already running).
  /// Caller should increment `ActiveAnimations` when this returns `true`.
  pub fn set_value(&mut self, value: f64) -> bool {
    let was_complete = self.animation.is_complete;
    self.animation.set_target(value);
    was_complete
  }

  /// Get current animated value.
  pub fn value(&self) -> f64 {
    *self.animation.value()
  }

  /// Update animation.
  ///
  /// Returns `true` if animation just completed this frame.
  /// Caller should decrement `ActiveAnimations` when this returns `true`.
  pub fn update(&mut self, delta: f32) -> bool {
    self.animation.update(delta)
  }
}
