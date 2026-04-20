//! Shimmer text rendering effect.
//!
//! Creates a sweeping highlight effect that moves across text.
//! Uses ECS ContinuousAnimations to track animation state.

use crate::assets::font;
use crate::assets::theme::get_theme;

use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::animation::shimmer::ShimmerAnimation;
use codelord_core::ecs::world::World;

use eframe::egui;

/// Renders text with a horizontal shimmer effect.
/// Uses ECS ContinuousAnimations to track animation state.
pub fn show(
  ui: &mut egui::Ui,
  world: &mut World,
  text: &str,
  font_size: f32,
  time: f32,
) {
  let theme = get_theme(world);
  let base_color = egui::Color32::from_rgba_unmultiplied(
    theme.primary[0],
    theme.primary[1],
    theme.primary[2],
    theme.primary[3],
  );

  let font_id = font::aeonik(font_size);

  // Pre-calculate text width
  let text_width = ui.fonts_mut(|f| {
    f.layout_no_wrap(text.to_string(), font_id.clone(), base_color)
      .rect
      .width()
  });

  // Allocate space
  let (rect, _response) = ui.allocate_exact_size(
    egui::vec2(text_width, font_size),
    egui::Sense::hover(),
  );

  // Animation config (same as codelord: 2s shimmer, 4s pause)
  let animation = ShimmerAnimation::with_timing(2.0, 100.0).with_intensity(0.7);

  // Calculate shimmer position with pause between cycles
  let (shimmer_center, _progress) =
    animation.calculate_position_with_pause(time, text_width, 4.0);

  // Render character by character with shimmer
  let painter = ui.painter();
  let pos = rect.left_top();
  let mut x_offset = 0.0;

  for ch in text.chars() {
    let char_str = ch.to_string();
    let galley =
      painter.layout_no_wrap(char_str.clone(), font_id.clone(), base_color);

    let char_width = galley.rect.width();
    let char_center_x = x_offset + char_width * 0.5;

    // Calculate shimmer intensity for this character
    let shimmer_intensity =
      animation.calculate_intensity(char_center_x, shimmer_center);

    // Blend base color with white
    let final_color = egui::Color32::from_rgba_unmultiplied(
      (base_color.r() as f32 * (1.0 - shimmer_intensity)
        + 255.0 * shimmer_intensity) as u8,
      (base_color.g() as f32 * (1.0 - shimmer_intensity)
        + 255.0 * shimmer_intensity) as u8,
      (base_color.b() as f32 * (1.0 - shimmer_intensity)
        + 255.0 * shimmer_intensity) as u8,
      base_color.a(),
    );

    painter.galley_with_override_text_color(
      pos + egui::vec2(x_offset, 0.0),
      galley.clone(),
      final_color,
    );

    x_offset += char_width;
  }

  // Track shimmer animation via ECS
  if let Some(mut cont) = world.get_resource_mut::<ContinuousAnimations>() {
    cont.set_shimmer_active();
  }
}
