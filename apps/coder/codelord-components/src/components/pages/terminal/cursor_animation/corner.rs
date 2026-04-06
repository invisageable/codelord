//! Corner of the animated cursor quad.

use super::spring::CriticallyDampedSpring;

/// One corner of the animated cursor quad.
/// Each corner moves independently using spring physics to create the smear
/// effect.
#[derive(Clone, Debug)]
pub struct Corner {
  /// Current screen position (pixels)
  pub current_position: (f32, f32),
  /// Position relative to cursor center (grid units)
  relative_position: (f32, f32),
  /// Last target position to detect jumps
  previous_destination: (f32, f32),
  /// X-axis spring animation
  animation_x: CriticallyDampedSpring,
  /// Y-axis spring animation
  animation_y: CriticallyDampedSpring,
  /// How long this corner should take to reach target
  pub animation_length: f32,
}

impl Corner {
  pub fn new(relative_position: (f32, f32)) -> Self {
    Self {
      current_position: (0.0, 0.0),
      relative_position,
      previous_destination: (-1000.0, -1000.0), /* Far away to trigger
                                                 * initial jump */
      animation_x: CriticallyDampedSpring::new(),
      animation_y: CriticallyDampedSpring::new(),
      animation_length: 0.0,
    }
  }

  /// Update corner position for one frame.
  ///
  /// # Arguments
  /// * `destination` - Target center position in pixels
  /// * `cursor_dimensions` - Size of cursor (width, height) in pixels
  /// * `dt` - Delta time since last frame
  /// * `immediate_movement` - Skip animation (for fast typing)
  ///
  /// # Returns
  /// `true` if still animating, `false` if settled
  pub fn update(
    &mut self,
    destination: (f32, f32),
    cursor_dimensions: (f32, f32),
    dt: f32,
    immediate_movement: bool,
  ) -> bool {
    // Calculate where this corner should be relative to cursor center
    let corner_destination =
      self.get_destination(destination, cursor_dimensions);

    // Detect if cursor jumped to new position
    if corner_destination != self.previous_destination {
      let delta = (
        corner_destination.0 - self.current_position.0,
        corner_destination.1 - self.current_position.1,
      );

      // Initialize spring with distance to target
      self.animation_x.position = delta.0;
      self.animation_y.position = delta.1;
      self.previous_destination = corner_destination;
    }

    // Immediate movement (no animation)
    if immediate_movement {
      self.current_position = corner_destination;
      return false;
    }

    // Update both axes using spring physics
    let mut animating = self.animation_x.update(dt, self.animation_length);
    animating |= self.animation_y.update(dt, self.animation_length);

    // Current position = target - remaining distance
    self.current_position.0 = corner_destination.0 - self.animation_x.position;
    self.current_position.1 = corner_destination.1 - self.animation_y.position;

    animating
  }

  /// Get the screen position this corner should move toward
  fn get_destination(
    &self,
    center: (f32, f32),
    cursor_dimensions: (f32, f32),
  ) -> (f32, f32) {
    // Scale relative position by cursor size
    let scaled = (
      self.relative_position.0 * cursor_dimensions.0,
      self.relative_position.1 * cursor_dimensions.1,
    );

    // Add to center position
    (center.0 + scaled.0, center.1 + scaled.1)
  }

  /// Calculate how aligned this corner is with the direction of motion.
  /// Returns value from -1.0 (opposite) to 1.0 (aligned).
  /// Used to rank corners for the smear effect.
  pub fn calculate_direction_alignment(
    &self,
    destination: (f32, f32),
    cursor_dimensions: (f32, f32),
  ) -> f32 {
    let corner_destination =
      self.get_destination(destination, cursor_dimensions);

    // Direction from current position to target
    let travel_dir = (
      corner_destination.0 - self.current_position.0,
      corner_destination.1 - self.current_position.1,
    );
    let travel_len =
      (travel_dir.0 * travel_dir.0 + travel_dir.1 * travel_dir.1).sqrt();

    if travel_len < 0.001 {
      return 0.0; // Not moving
    }

    let travel_normalized =
      (travel_dir.0 / travel_len, travel_dir.1 / travel_len);

    // Corner's direction from center
    let corner_len = (self.relative_position.0 * self.relative_position.0
      + self.relative_position.1 * self.relative_position.1)
      .sqrt();

    if corner_len < 0.001 {
      return 0.0;
    }

    let corner_normalized = (
      self.relative_position.0 / corner_len,
      self.relative_position.1 / corner_len,
    );

    // Dot product: how aligned are the directions?
    travel_normalized.0 * corner_normalized.0
      + travel_normalized.1 * corner_normalized.1
  }
}
