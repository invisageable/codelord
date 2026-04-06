//! Critically damped spring animation for smooth cursor movement.
//! Based on neovide's implementation using harmonic oscillator physics.
//!
//! References:
//! - https://gdcvault.com/play/1027059/Math-In-Game-Development-Summit
//! - https://www.ryanjuckett.com/damped-springs/

/// Critically damped spring for smooth animation without overshoot.
#[derive(Clone, Debug)]
pub struct CriticallyDampedSpring {
  /// Current position relative to target (0.0 = at target)
  pub position: f32,
  /// Current velocity
  velocity: f32,
}

impl CriticallyDampedSpring {
  pub fn new() -> Self {
    Self {
      position: 0.0,
      velocity: 0.0,
    }
  }

  /// Update the spring animation for one frame.
  ///
  /// # Arguments
  /// * `dt` - Delta time since last frame (seconds)
  /// * `animation_length` - Time to reach target with 2% tolerance
  ///
  /// # Returns
  /// `true` if still animating, `false` if settled at target
  pub fn update(&mut self, dt: f32, animation_length: f32) -> bool {
    // If animation would complete this frame, just snap to target
    if animation_length <= dt {
      self.reset();
      return false;
    }

    // Already at target
    if self.position == 0.0 {
      return false;
    }

    // Critically damped spring physics
    // zeta = 1.0 (critically damped - no overshoot, fastest settling)
    let zeta = 1.0;

    // Calculate omega so we reach destination with 2% tolerance in
    // animation_length
    let omega = 4.0 / (zeta * animation_length);

    // Analytical solution for critically damped harmonic oscillation
    // Initial conditions: a = position at t=0, b = velocity contribution
    let a = self.position;
    let b = self.position * omega + self.velocity;

    // Exponential decay factor
    let c = (-omega * dt).exp();

    // Update position and velocity using analytical formula
    self.position = (a + b * dt) * c;
    self.velocity = c * (-a * omega - b * dt * omega + b);

    // Consider settled if very close to target (< 0.01 pixels)
    if self.position.abs() < 0.01 {
      self.reset();
      false
    } else {
      true
    }
  }

  /// Reset spring to settled state at target
  pub fn reset(&mut self) {
    self.position = 0.0;
    self.velocity = 0.0;
  }
}

impl Default for CriticallyDampedSpring {
  fn default() -> Self {
    Self::new()
  }
}
