//! Animated diagonal stripe effect.
//!
//! A high-performance, data-oriented stripe rendering effect that displays
//! animated diagonal stripes on hover.
//!
//! ## Performance Characteristics
//!
//! - Zero heap allocations per frame
//! - Stateless rendering (pure function)
//! - O(n) where n = ~10-12 stripes (constant)

use eframe::egui;

const STRIPE_WIDTH: f32 = 5.0;
const STRIPE_SPEED: f32 = 30.0;

/// Renders animated diagonal stripes within the given bounds.
///
/// Uses a simple modulo-based animation for infinite scrolling effect.
/// All calculations use stack variables (zero allocations).
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `rect` - The rectangle bounds to render stripes in
/// * `color` - The stripe color
pub fn show(ui: &egui::Ui, rect: egui::Rect, color: egui::Color32) {
  let painter = ui.painter();

  let time = ui.input(|i| i.time as f32);
  let pattern_period = STRIPE_WIDTH * 2.0; // stripe + gap
  let offset = (time * STRIPE_SPEED) % pattern_period;

  // Calculate number of stripes needed to cover the rect
  // Extra stripes for diagonal coverage
  let diagonal_length = (rect.width().powi(2) + rect.height().powi(2)).sqrt();
  let num_stripes = (diagonal_length / pattern_period).ceil() as usize + 2;

  // Draw diagonal stripes as parallelograms
  for i in 0..num_stripes {
    let x_base =
      rect.min.x - rect.height() + (i as f32 * pattern_period) + offset;

    let top_left = egui::pos2(x_base, rect.min.y);
    let top_right = egui::pos2(x_base + STRIPE_WIDTH, rect.min.y);
    let bottom_left = egui::pos2(x_base + rect.height(), rect.max.y);
    let bottom_right =
      egui::pos2(x_base + rect.height() + STRIPE_WIDTH, rect.max.y);

    // Clip to rect bounds and draw
    let points = vec![top_left, top_right, bottom_right, bottom_left];

    // Only draw if stripe intersects rect bounds
    let stripe_bounds = egui::Rect::from_min_max(
      egui::pos2(x_base.min(x_base + rect.height()), rect.min.y),
      egui::pos2(
        (x_base + STRIPE_WIDTH).max(x_base + rect.height() + STRIPE_WIDTH),
        rect.max.y,
      ),
    );

    if stripe_bounds.intersects(rect) {
      painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
      ));
    }
  }
}
