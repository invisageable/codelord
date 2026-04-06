pub mod font;
pub mod icon;
pub mod image;
pub mod theme;

use eframe::egui;

/// Installs all assets (images, fonts, sounds).
pub fn install_assets(ctx: &egui::Context) {
  image::install_images(ctx);
  font::install_fonts(ctx);
}
