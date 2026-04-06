//! Shimmer animation for text with sweeping highlight effect.

/// Animation state for shimmer text effect.
#[derive(Debug, Clone)]
pub struct ShimmerAnimation {
  /// The duration for one complete shimmer cycle (in seconds).
  pub cycle_duration: f32,
  /// The width of the shimmer highlight (in pixels).
  pub shimmer_width: f32,
  /// The maximum brightness increase (0.0 to 1.0).
  pub intensity: f32,
  /// Whether the shimmer should loop continuously.
  pub looping: bool,
}

impl ShimmerAnimation {
  /// Creates a new [`ShimmerAnimation`] with default settings.
  pub fn new() -> Self {
    Self {
      cycle_duration: 3.0,
      shimmer_width: 60.0,
      intensity: 0.8,
      looping: true,
    }
  }

  /// Creates a shimmer animation with custom timing.
  pub fn with_timing(cycle_duration: f32, shimmer_width: f32) -> Self {
    Self {
      cycle_duration,
      shimmer_width,
      intensity: 0.8,
      looping: true,
    }
  }

  /// Sets custom intensity.
  pub fn with_intensity(mut self, intensity: f32) -> Self {
    self.intensity = intensity.clamp(0.0, 1.0);
    self
  }

  /// Calculates the shimmer position for a given time and text width.
  pub fn calculate_position(
    &self,
    current_time: f32,
    text_width: f32,
  ) -> (f32, f32) {
    let total_travel_distance = text_width + self.shimmer_width * 2.0;

    let progress = if self.looping {
      (current_time % self.cycle_duration) / self.cycle_duration
    } else {
      (current_time / self.cycle_duration).min(1.0)
    };

    let shimmer_center = progress * total_travel_distance - self.shimmer_width;

    (shimmer_center, progress)
  }

  /// Calculates shimmer position with a pause between cycles.
  pub fn calculate_position_with_pause(
    &self,
    current_time: f32,
    text_width: f32,
    pause_duration: f32,
  ) -> (f32, f32) {
    let total_travel_distance = text_width + self.shimmer_width * 2.0;
    let full_cycle_duration = self.cycle_duration + pause_duration;
    let time_in_cycle = current_time % full_cycle_duration;

    // Only animate during active time (not during pause)
    let progress = if time_in_cycle < self.cycle_duration {
      time_in_cycle / self.cycle_duration
    } else {
      1.0 // Keep at end position during pause
    };

    let shimmer_center = progress * total_travel_distance - self.shimmer_width;

    (shimmer_center, progress)
  }

  /// Calculates the shimmer intensity for a given character position.
  pub fn calculate_intensity(&self, char_x: f32, shimmer_center: f32) -> f32 {
    let distance_from_shimmer = (char_x - shimmer_center).abs();

    if distance_from_shimmer < self.shimmer_width {
      let normalized = distance_from_shimmer / self.shimmer_width;
      // Smooth curve: peaks at center, fades to edges
      (1.0 - normalized).powf(2.0) * self.intensity
    } else {
      0.0
    }
  }
}

impl Default for ShimmerAnimation {
  fn default() -> Self {
    Self::new()
  }
}
