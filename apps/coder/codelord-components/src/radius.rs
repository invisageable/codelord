//! Egui `CornerRadius` helpers.

use eframe::egui;

/// Build a [`egui::CornerRadius`] with the same radius on both top
/// corners (`north`) and both bottom corners (`south`).
pub fn symmetric(north: u8, south: u8) -> egui::CornerRadius {
  egui::CornerRadius {
    nw: north,
    ne: north,
    sw: south,
    se: south,
  }
}
