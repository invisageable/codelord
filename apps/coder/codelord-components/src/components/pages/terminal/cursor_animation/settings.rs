//! Configuration for cursor animation behavior.

/// Configuration for cursor animation behavior.
#[derive(Clone, Debug)]
pub struct CursorAnimationSettings {
  /// Enable/disable cursor animation
  pub enabled: bool,
  /// Normal animation duration (seconds)
  pub animation_length: f32,
  /// Fast animation for typing/small movements (seconds)
  pub short_animation_length: f32,
  /// Trail size: 0.0 = no trail, 1.0 = maximum trail/smear effect
  pub trail_size: f32,
}

impl Default for CursorAnimationSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      animation_length: 0.150,       // 150ms
      short_animation_length: 0.040, // 40ms
      trail_size: 0.7,               // Moderate trail
    }
  }
}

impl CursorAnimationSettings {
  /// Create settings with custom values
  pub fn new(enabled: bool, animation_length: f32, trail_size: f32) -> Self {
    Self {
      enabled,
      animation_length,
      short_animation_length: (animation_length * 0.3).max(0.020),
      trail_size: trail_size.clamp(0.0, 1.0),
    }
  }

  /// Disable all animation (instant cursor jumps)
  pub fn instant() -> Self {
    Self {
      enabled: false,
      ..Default::default()
    }
  }

  /// Subtle animation (minimal trail)
  pub fn subtle() -> Self {
    Self {
      trail_size: 0.3,
      ..Default::default()
    }
  }

  /// Maximum trail effect
  pub fn max_trail() -> Self {
    Self {
      trail_size: 1.0,
      ..Default::default()
    }
  }
}
