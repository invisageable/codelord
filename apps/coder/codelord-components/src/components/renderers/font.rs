//! Font atlas renderer.
//!
//! Displays font preview with uppercase, lowercase, and digits.

use eframe::egui::{self, Color32, FontFamily, FontId, RichText, Ui};

const PREVIEW_UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const PREVIEW_LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const PREVIEW_DIGITS: &str = "1234567890";

/// View data for font preview rendering.
pub struct FontViewData<'a> {
  pub font_name: &'a str,
  pub has_error: bool,
  pub error_msg: Option<&'a str>,
  pub family_name: Option<&'a str>,
}

/// Renders font atlas preview.
pub fn render(ui: &mut Ui, data: &FontViewData<'_>) {
  let available = ui.available_size();

  ui.vertical_centered(|ui| {
    ui.add_space(40.0);

    // Header - font name
    ui.label(
      RichText::new(data.font_name)
        .size(16.0)
        .color(Color32::from_rgb(180, 180, 180)),
    );

    ui.add_space(40.0);

    if data.has_error {
      ui.label(
        RichText::new(data.error_msg.unwrap_or("Failed to load font"))
          .size(14.0)
          .color(Color32::from_rgb(255, 100, 100)),
      );
      return;
    }

    // Calculate font size to fit width (80% of available)
    let font_size = calculate_font_size(available.x);

    // Get font family to use
    let font_family = match data.family_name {
      Some(name) => FontFamily::Name(name.into()),
      None => FontFamily::Monospace,
    };
    let font_id = FontId::new(font_size, font_family);

    // Uppercase
    ui.label(RichText::new(PREVIEW_UPPER).font(font_id.clone()));
    ui.add_space(24.0);

    // Lowercase
    ui.label(RichText::new(PREVIEW_LOWER).font(font_id.clone()));
    ui.add_space(24.0);

    // Digits
    ui.label(RichText::new(PREVIEW_DIGITS).font(font_id));
  });
}

/// Calculates optimal font size based on available width.
fn calculate_font_size(available_width: f32) -> f32 {
  // Target 80% of available width for 26 chars (uppercase)
  // Rough estimate: proportional fonts ~0.5 width ratio
  let target_width = available_width * 0.8;
  let char_count = PREVIEW_UPPER.len() as f32;
  let estimated_char_width = 0.55;
  let size = target_width / (char_count * estimated_char_width);
  size.clamp(24.0, 120.0)
}

/// Registers a font with egui for preview.
/// Returns the family name to use for rendering.
/// Only call when font_data changes (check generation).
pub fn register_preview_font(
  ctx: &egui::Context,
  font_name: &str,
  font_data: &[u8],
) -> String {
  use crate::assets::font::base_font_definitions;

  let family_name = format!("preview_{font_name}");

  // Start with app's base fonts to preserve all custom fonts
  let mut fonts = base_font_definitions();

  // Add our preview font
  fonts.font_data.insert(
    family_name.clone(),
    egui::FontData::from_owned(font_data.to_vec()).into(),
  );

  fonts.families.insert(
    FontFamily::Name(family_name.clone().into()),
    vec![family_name.clone()],
  );

  ctx.set_fonts(fonts);

  // Request repaint to ensure fonts are loaded before next render
  ctx.request_repaint();

  family_name
}
