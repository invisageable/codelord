//! Hacker text animation.
//!
//! Shows all characters immediately with random chars cycling
//! through before they lock into place progressively from left to right.
//! Preserves string length throughout the animation.

/// Range of ASCII characters for random text effect.
const SPECIAL_CHARS_RANGE: std::ops::RangeInclusive<u8> = 33_u8..=126;

/// Generate random char in range using thread-local RNG.
#[inline]
fn random_special_char() -> char {
  fastrand::u8(SPECIAL_CHARS_RANGE) as char
}

/// Hacker animation state for constant-width text effects.
#[derive(Debug, Clone)]
pub struct HackerAnimation {
  /// The full text to animate.
  text: String,
  /// Timer for animation progress.
  timer: f32,
  /// Animation speed multiplier.
  speed: f32,
  /// Whether animation is finished.
  finished: bool,
  /// Intermediate characters showing random chars before locking.
  intermediate_chars: Vec<char>,
}

impl HackerAnimation {
  /// Creates a new hacker animation.
  pub fn new(text: impl Into<String>) -> Self {
    let text = text.into();

    let intermediate_chars = text
      .chars()
      .map(|c| {
        if c.is_whitespace() {
          c
        } else {
          random_special_char()
        }
      })
      .collect::<Vec<_>>();

    Self {
      text,
      timer: 0.0,
      speed: 2.5,
      finished: false,
      intermediate_chars,
    }
  }

  /// Returns whether animation is finished.
  pub fn is_finished(&self) -> bool {
    self.finished
  }

  /// Returns the current animated text.
  pub fn visible_text(&self) -> String {
    self.intermediate_chars.iter().collect()
  }

  /// Resets the animation.
  pub fn reset(&mut self) {
    self.timer = 0.0;
    self.finished = false;

    self.intermediate_chars = self
      .text
      .chars()
      .map(|c| {
        if c.is_whitespace() {
          c
        } else {
          random_special_char()
        }
      })
      .collect();
  }

  /// Updates the animation. Returns true if still animating.
  pub fn update(&mut self, dt: f32) -> bool {
    if self.finished {
      return false;
    }

    let increment = dt * self.speed;
    let chars = self.text.chars().collect::<Vec<_>>();
    let num_chars = chars.len();
    let locked_chars = (self.timer * num_chars as f32).floor() as usize;

    // Update ALL characters with random chars
    for i in 0..num_chars {
      if self.intermediate_chars.get(i) != chars.get(i) {
        if chars[i].is_whitespace() {
          if let Some(intermediate_char) = self.intermediate_chars.get_mut(i) {
            *intermediate_char = chars[i];
          }
        } else {
          let random_char = random_special_char();
          if let Some(intermediate_char) = self.intermediate_chars.get_mut(i) {
            *intermediate_char = random_char;
          }
        }
      }
    }

    // Lock characters progressively from left to right
    let lock_threshold = 0.09;
    for i in 0..locked_chars.min(num_chars) {
      if self.intermediate_chars.get(i) != chars.get(i)
        && !chars[i].is_whitespace()
        && fastrand::f32() < lock_threshold
        && let Some(intermediate_char) = self.intermediate_chars.get_mut(i)
      {
        *intermediate_char = chars[i];
      }
    }

    self.timer = (self.timer + increment).min(num_chars as f32);
    if self.timer >= num_chars as f32 {
      self.finished = true;
      return false;
    }

    true
  }
}
