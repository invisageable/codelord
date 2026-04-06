//! Animation resources

use crate::ecs::prelude::*;

/// Shake animation state for window vibration effect.
#[derive(Debug, Clone)]
pub struct ShakeAnimation {
  pub start_time: f64,
  pub duration: f64,
  pub intensity: f32,
  pub original_x: f32,
  pub original_y: f32,
}

impl ShakeAnimation {
  pub fn new(start_time: f64, original_x: f32, original_y: f32) -> Self {
    Self {
      start_time,
      duration: 0.5,
      intensity: 10.0,
      original_x,
      original_y,
    }
  }
}

/// Resource that tracks active animations
///
/// Systems increment/decrement this counter when creating/removing animations.
/// The application layer checks this to know if it should request repaint.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ActiveAnimations {
  pub count: u32,
}

impl ActiveAnimations {
  pub fn increment(&mut self) {
    self.count = self.count.saturating_add(1);
  }

  pub fn decrement(&mut self) {
    self.count = self.count.saturating_sub(1);
  }

  pub fn has_active(&self) -> bool {
    self.count > 0
  }
}

/// Resource that tracks continuous animations.
///
/// Continuous animations run as long as a condition is true:
/// - Wave animation: while welcome page is visible
/// - Stripe animation: while explorer item is hovered
/// - Cursor blink: while text editor is focused
///
/// Uses a two-frame approach:
/// - `current_*`: Set to true by render code when animation is active this
///   frame
/// - `prev_*`: What was active last frame
///
/// A system compares current vs prev to update ActiveAnimations counter.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ContinuousAnimations {
  // Current frame state (set by render code)
  pub wave_current: bool,
  pub stripe_current: bool,
  pub cursor_blink_current: bool,
  pub terminal_current: bool,
  pub hacker_current: bool,
  pub shimmer_current: bool,
  pub toast_current: bool,
  pub voice_current: bool,
  pub shake_current: bool,
  pub sonar_current: bool,
  pub search_hint_current: bool,
  pub music_player_current: bool,
  pub blame_current: bool,
  pub spinner_current: bool,
  pub presenter_current: bool,
  pub loading_bar_current: bool,

  // Previous frame state (for comparison)
  wave_prev: bool,
  stripe_prev: bool,
  cursor_blink_prev: bool,
  terminal_prev: bool,
  hacker_prev: bool,
  shimmer_prev: bool,
  toast_prev: bool,
  voice_prev: bool,
  shake_prev: bool,
  sonar_prev: bool,
  search_hint_prev: bool,
  music_player_prev: bool,
  blame_prev: bool,
  spinner_prev: bool,
  presenter_prev: bool,
  loading_bar_prev: bool,
}

impl ContinuousAnimations {
  /// Mark wave animation as active this frame.
  pub fn set_wave_active(&mut self) {
    self.wave_current = true;
  }

  /// Mark stripe animation as active this frame.
  pub fn set_stripe_active(&mut self) {
    self.stripe_current = true;
  }

  /// Mark cursor blink as active this frame.
  pub fn set_cursor_blink_active(&mut self) {
    self.cursor_blink_current = true;
  }

  /// Mark terminal as active this frame.
  pub fn set_terminal_active(&mut self) {
    self.terminal_current = true;
  }

  /// Mark hacker animation as active this frame.
  pub fn set_hacker_active(&mut self) {
    self.hacker_current = true;
  }

  /// Mark shimmer animation as active this frame.
  pub fn set_shimmer_active(&mut self) {
    self.shimmer_current = true;
  }

  /// Mark toast animation as active this frame.
  pub fn set_toast_active(&mut self) {
    self.toast_current = true;
  }

  /// Mark voice animation as active this frame.
  pub fn set_voice_active(&mut self) {
    self.voice_current = true;
  }

  /// Mark shake animation as active this frame.
  pub fn set_shake_active(&mut self) {
    self.shake_current = true;
  }

  /// Mark sonar animation as active this frame.
  pub fn set_sonar_active(&mut self) {
    self.sonar_current = true;
  }

  /// Mark search hint animation as active this frame.
  pub fn set_search_hint_active(&mut self) {
    self.search_hint_current = true;
  }

  /// Mark music player animation as active this frame.
  pub fn set_music_player_active(&mut self) {
    self.music_player_current = true;
  }

  /// Mark blame animation as active this frame.
  pub fn set_blame_active(&mut self) {
    self.blame_current = true;
  }

  /// Mark spinner animation as active this frame.
  pub fn set_spinner_active(&mut self) {
    self.spinner_current = true;
  }

  /// Mark presenter animation as active this frame.
  pub fn set_presenter_active(&mut self) {
    self.presenter_current = true;
  }

  /// Mark loading bar animation as active this frame.
  pub fn set_loading_bar_active(&mut self) {
    self.loading_bar_current = true;
  }

  /// Called at end of frame. Compares current vs prev and returns
  /// (increments, decrements) for ActiveAnimations.
  pub fn end_frame(&mut self) -> (u32, u32) {
    let mut increments = 0u32;
    let mut decrements = 0u32;

    // Wave
    if self.wave_current && !self.wave_prev {
      increments += 1;
    } else if !self.wave_current && self.wave_prev {
      decrements += 1;
    }

    // Stripe
    if self.stripe_current && !self.stripe_prev {
      increments += 1;
    } else if !self.stripe_current && self.stripe_prev {
      decrements += 1;
    }

    // Cursor blink
    if self.cursor_blink_current && !self.cursor_blink_prev {
      increments += 1;
    } else if !self.cursor_blink_current && self.cursor_blink_prev {
      decrements += 1;
    }

    // Terminal
    if self.terminal_current && !self.terminal_prev {
      increments += 1;
    } else if !self.terminal_current && self.terminal_prev {
      decrements += 1;
    }

    // Hacker
    if self.hacker_current && !self.hacker_prev {
      increments += 1;
    } else if !self.hacker_current && self.hacker_prev {
      decrements += 1;
    }

    // Shimmer
    if self.shimmer_current && !self.shimmer_prev {
      increments += 1;
    } else if !self.shimmer_current && self.shimmer_prev {
      decrements += 1;
    }

    // Toast
    if self.toast_current && !self.toast_prev {
      increments += 1;
    } else if !self.toast_current && self.toast_prev {
      decrements += 1;
    }

    // Voice
    if self.voice_current && !self.voice_prev {
      increments += 1;
    } else if !self.voice_current && self.voice_prev {
      decrements += 1;
    }

    // Shake
    if self.shake_current && !self.shake_prev {
      increments += 1;
    } else if !self.shake_current && self.shake_prev {
      decrements += 1;
    }

    // Sonar
    if self.sonar_current && !self.sonar_prev {
      increments += 1;
    } else if !self.sonar_current && self.sonar_prev {
      decrements += 1;
    }

    // Search hint
    if self.search_hint_current && !self.search_hint_prev {
      increments += 1;
    } else if !self.search_hint_current && self.search_hint_prev {
      decrements += 1;
    }

    // Music player
    if self.music_player_current && !self.music_player_prev {
      increments += 1;
    } else if !self.music_player_current && self.music_player_prev {
      decrements += 1;
    }

    // Blame
    if self.blame_current && !self.blame_prev {
      increments += 1;
    } else if !self.blame_current && self.blame_prev {
      decrements += 1;
    }

    // Spinner
    if self.spinner_current && !self.spinner_prev {
      increments += 1;
    } else if !self.spinner_current && self.spinner_prev {
      decrements += 1;
    }

    // Presenter
    if self.presenter_current && !self.presenter_prev {
      increments += 1;
    } else if !self.presenter_current && self.presenter_prev {
      decrements += 1;
    }

    // Loading bar
    if self.loading_bar_current && !self.loading_bar_prev {
      increments += 1;
    } else if !self.loading_bar_current && self.loading_bar_prev {
      decrements += 1;
    }

    // Move current to prev, reset current for next frame
    self.wave_prev = self.wave_current;
    self.stripe_prev = self.stripe_current;
    self.cursor_blink_prev = self.cursor_blink_current;
    self.terminal_prev = self.terminal_current;
    self.hacker_prev = self.hacker_current;
    self.shimmer_prev = self.shimmer_current;
    self.toast_prev = self.toast_current;
    self.voice_prev = self.voice_current;
    self.shake_prev = self.shake_current;
    self.sonar_prev = self.sonar_current;
    self.search_hint_prev = self.search_hint_current;
    self.music_player_prev = self.music_player_current;
    self.blame_prev = self.blame_current;
    self.spinner_prev = self.spinner_current;
    self.presenter_prev = self.presenter_current;
    self.loading_bar_prev = self.loading_bar_current;

    self.wave_current = false;
    self.stripe_current = false;
    self.cursor_blink_current = false;
    self.terminal_current = false;
    self.hacker_current = false;
    self.shimmer_current = false;
    self.toast_current = false;
    self.voice_current = false;
    self.shake_current = false;
    self.sonar_current = false;
    self.search_hint_current = false;
    self.music_player_current = false;
    self.blame_current = false;
    self.spinner_current = false;
    self.presenter_current = false;
    self.loading_bar_current = false;

    (increments, decrements)
  }
}
