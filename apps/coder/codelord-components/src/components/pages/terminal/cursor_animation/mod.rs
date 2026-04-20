//! Animated cursor with smear/trail effect.
//!
//! Uses spring physics for smooth cursor movement. The cursor is rendered
//! as a quad with 4 independently animated corners to create the smear effect.

mod corner;
mod settings;
mod spring;

pub use settings::CursorAnimationSettings;

use corner::Corner;
use eframe::egui::{self, epaint::PathShape};

// Standard cursor corners: (x, y) relative to center
// (-0.5, -0.5) = top-left, (0.5, -0.5) = top-right, etc.
const STANDARD_CORNERS: [(f32, f32); 4] =
  [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];

/// Animated cursor with smear/trail effect.
/// Cursor is rendered as a quad with 4 independently animated corners.
#[derive(Clone)]
pub struct AnimatedCursor {
  corners: [Corner; 4],
  destination: (f32, f32),
  jumped: bool,
  pub settings: CursorAnimationSettings,
}

impl AnimatedCursor {
  pub fn new() -> Self {
    Self::with_settings(CursorAnimationSettings::default())
  }

  pub fn with_settings(settings: CursorAnimationSettings) -> Self {
    let corners = [
      Corner::new(STANDARD_CORNERS[0]),
      Corner::new(STANDARD_CORNERS[1]),
      Corner::new(STANDARD_CORNERS[2]),
      Corner::new(STANDARD_CORNERS[3]),
    ];

    Self {
      corners,
      destination: (0.0, 0.0),
      jumped: false,
      settings,
    }
  }

  /// Jump cursor to new position.
  /// Triggers animation with smear effect based on distance and direction.
  pub fn jump_to(
    &mut self,
    new_destination: (f32, f32),
    cell_size: (f32, f32),
  ) {
    if !self.settings.enabled {
      // Instant jump
      self.destination = new_destination;
      for corner in &mut self.corners {
        corner.current_position = new_destination;
      }
      return;
    }

    self.destination = new_destination;
    self.jumped = true;

    // Calculate how far the cursor is jumping (in grid cells)
    let prev_dest = self.destination;
    let jump_vec = (
      (new_destination.0 - prev_dest.0) / cell_size.0,
      (new_destination.1 - prev_dest.1) / cell_size.1,
    );

    // Determine if this is a short jump (typing) or long jump (mouse/search)
    let is_short_jump = jump_vec.0.abs() <= 2.0 && jump_vec.1.abs() < 0.001;

    if is_short_jump {
      // Fast animation for typing
      for corner in &mut self.corners {
        corner.animation_length = self
          .settings
          .animation_length
          .min(self.settings.short_animation_length);
      }
    } else {
      // Long jump: rank corners by direction alignment
      let mut alignments: Vec<(usize, f32)> = self
        .corners
        .iter()
        .enumerate()
        .map(|(i, corner)| {
          (
            i,
            corner.calculate_direction_alignment(new_destination, cell_size),
          )
        })
        .collect();

      // Sort by alignment (lowest = trailing, highest = leading)
      alignments.sort_by(|a, b| {
        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
      });

      // Create rank lookup (0 = trailing, 3 = leading)
      let mut ranks = [0usize; 4];
      for (rank, (index, _)) in alignments.iter().enumerate() {
        ranks[*index] = rank;
      }

      // Assign animation speeds based on rank
      let leading_speed = self.settings.animation_length
        * (1.0 - self.settings.trail_size).clamp(0.0, 1.0);
      let trailing_speed = self.settings.animation_length;

      for (i, corner) in self.corners.iter_mut().enumerate() {
        corner.animation_length = match ranks[i] {
          3 | 2 => leading_speed, // Front corners move fast
          1 => (leading_speed + trailing_speed) / 2.0, // Middle speed
          0 => trailing_speed,    // Trailing corner moves slow
          _ => trailing_speed,
        };
      }
    }
  }

  /// Animate cursor for one frame.
  ///
  /// # Arguments
  /// * `dt` - Delta time since last frame (seconds)
  /// * `cursor_dimensions` - Size of cursor (width, height) in pixels
  /// * `immediate_movement` - Skip animation (for fast typing mode)
  ///
  /// # Returns
  /// `true` if still animating, `false` if settled.
  /// The caller should use ContinuousAnimations to request repaint if true.
  pub fn animate(
    &mut self,
    dt: f32,
    cursor_dimensions: (f32, f32),
    immediate_movement: bool,
  ) -> bool {
    if !self.settings.enabled {
      return false;
    }

    let mut still_animating = false;

    for corner in &mut self.corners {
      let animating = corner.update(
        self.destination,
        cursor_dimensions,
        dt,
        immediate_movement,
      );
      still_animating |= animating;
    }

    self.jumped = false;

    still_animating
  }

  /// Render the animated cursor.
  pub fn render(&self, painter: &egui::Painter, color: egui::Color32) {
    // Build path from corner positions
    let points = self
      .corners
      .iter()
      .map(|c| egui::Pos2::new(c.current_position.0, c.current_position.1))
      .collect::<Vec<_>>();

    // Draw filled quad
    let shape = PathShape::convex_polygon(points, color, egui::Stroke::NONE);

    painter.add(shape);
  }

  /// Get current destination position
  pub fn destination(&self) -> (f32, f32) {
    self.destination
  }

  /// Check if cursor is currently animating
  pub fn is_animating(&self) -> bool {
    self.jumped
  }
}

impl Default for AnimatedCursor {
  fn default() -> Self {
    Self::new()
  }
}
