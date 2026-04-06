//! Decoration atom - reusable circular button
//!
//! Pure UI component that can optionally read from ECS entity state

use codelord_core::ecs::{entity::Entity, world::World};
use codelord_core::ui::component::Hovered;

use eframe::egui;

/// Decoration button style
#[derive(Debug, Clone, Copy)]
pub struct DecorationStyle {
  /// Base color
  pub color: egui::Color32,
  /// Color when hovered
  pub hover_color: egui::Color32,
  /// Size of the decoration
  pub size: egui::Vec2,
  /// Corner radius (for roundness)
  pub corner_radius: f32,
}

impl DecorationStyle {
  /// macOS-style close button (red)
  pub fn close() -> Self {
    Self {
      color: egui::Color32::from_rgb(255, 97, 87),
      hover_color: egui::Color32::from_rgb(255, 70, 60),
      size: egui::Vec2::new(24.0, 12.0),
      corner_radius: 6.0,
    }
  }

  /// macOS-style minimize button (yellow)
  pub fn minimize() -> Self {
    Self {
      color: egui::Color32::from_rgb(255, 189, 76),
      hover_color: egui::Color32::from_rgb(255, 180, 50),
      size: egui::Vec2::new(24.0, 12.0),
      corner_radius: 6.0,
    }
  }

  /// macOS-style maximize button (green)
  pub fn maximize() -> Self {
    Self {
      color: egui::Color32::from_rgb(40, 201, 64),
      hover_color: egui::Color32::from_rgb(30, 190, 50),
      size: egui::Vec2::new(24.0, 12.0),
      corner_radius: 6.0,
    }
  }

  /// Custom decoration
  pub fn custom(color: egui::Color32, size: f32) -> Self {
    Self {
      color,
      hover_color: color,
      size: egui::Vec2::splat(size),
      corner_radius: size * 0.5,
    }
  }
}

/// Render a decoration button reading state from ECS entity
///
/// Returns true if clicked. Updates entity's Hovered component.
pub fn show(
  ui: &mut egui::Ui,
  world: &World,
  entity: Entity,
  style: DecorationStyle,
) -> bool {
  let response = ui.allocate_response(style.size, egui::Sense::click());

  // Read hover state from entity (marker presence = hovered)
  let is_hovered =
    world.entity(entity).contains::<Hovered>() || response.hovered();

  // Choose color based on hover
  let color = if is_hovered {
    style.hover_color
  } else {
    style.color
  };

  // Draw circle
  ui.painter()
    .rect_filled(response.rect, style.corner_radius, color);

  response.clicked()
}
