//! Interpolation trait for animatable types
//!
//! Any type that implements Interpolate can be animated.

/// Trait for types that can be interpolated
pub trait Interpolate: Clone {
  /// Linearly interpolate between self and target
  ///
  /// # Arguments
  /// * `target` - The target value
  /// * `t` - Interpolation factor (0.0 = self, 1.0 = target)
  fn lerp(&self, target: &Self, t: f32) -> Self;

  /// Check if value is close enough to target (for early completion)
  fn is_close(&self, other: &Self, epsilon: f32) -> bool;
}

// ============================================================================
// Implementations for common types
// ============================================================================

impl Interpolate for f32 {
  fn lerp(&self, target: &Self, t: f32) -> Self {
    self + (target - self) * t
  }

  fn is_close(&self, other: &Self, epsilon: f32) -> bool {
    (self - other).abs() < epsilon
  }
}

impl Interpolate for f64 {
  fn lerp(&self, target: &Self, t: f32) -> Self {
    self + (target - self) * t as f64
  }

  fn is_close(&self, other: &Self, epsilon: f32) -> bool {
    (self - other).abs() < epsilon as f64
  }
}

/// 2D vector for positions, sizes, etc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
  pub x: f32,
  pub y: f32,
}

impl Interpolate for Vec2 {
  fn lerp(&self, target: &Self, t: f32) -> Self {
    Self {
      x: self.x.lerp(&target.x, t),
      y: self.y.lerp(&target.y, t),
    }
  }

  fn is_close(&self, other: &Self, epsilon: f32) -> bool {
    self.x.is_close(&other.x, epsilon) && self.y.is_close(&other.y, epsilon)
  }
}

/// RGB Color (0.0 to 1.0 range)
///
/// Pure data structure - no dependencies on rendering libraries
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
  pub r: f32,
  pub g: f32,
  pub b: f32,
  pub a: f32,
}

impl Interpolate for Color {
  fn lerp(&self, target: &Self, t: f32) -> Self {
    Self {
      r: self.r.lerp(&target.r, t),
      g: self.g.lerp(&target.g, t),
      b: self.b.lerp(&target.b, t),
      a: self.a.lerp(&target.a, t),
    }
  }

  fn is_close(&self, other: &Self, epsilon: f32) -> bool {
    self.r.is_close(&other.r, epsilon)
      && self.g.is_close(&other.g, epsilon)
      && self.b.is_close(&other.b, epsilon)
      && self.a.is_close(&other.a, epsilon)
  }
}

impl Color {
  /// Create color from RGB (0-255 range)
  pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
    Self {
      r: r as f32 / 255.0,
      g: g as f32 / 255.0,
      b: b as f32 / 255.0,
      a: 1.0,
    }
  }

  /// Create color from RGBA (0-255 range)
  pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
    Self {
      r: r as f32 / 255.0,
      g: g as f32 / 255.0,
      b: b as f32 / 255.0,
      a: a as f32 / 255.0,
    }
  }

  /// Convert to RGB bytes (0-255)
  pub fn to_rgba_u8(&self) -> [u8; 4] {
    [
      (self.r * 255.0) as u8,
      (self.g * 255.0) as u8,
      (self.b * 255.0) as u8,
      (self.a * 255.0) as u8,
    ]
  }
}
