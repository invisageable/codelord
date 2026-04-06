//! Lightweight icon button component.
//!
//! A simple icon button that doesn't require ECS, suitable for toolbars
//! and quick actions where entity management is overhead.

use crate::assets::icon::icon_to_image;

use codelord_core::icon::components::Icon;

use eframe::egui;

/// Renders an icon-only button.
///
/// Returns `true` if the button was clicked.
pub fn show(ui: &mut egui::Ui, icon: &Icon, tint: egui::Color32) -> bool {
  let response = ui.add(
    egui::Button::image(
      icon_to_image(icon)
        .fit_to_exact_size(egui::vec2(12.0, 12.0))
        .tint(tint),
    )
    .frame(false),
  );

  if response.hovered() {
    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
  }

  response.clicked()
}

/// Renders an icon-only button with custom size.
///
/// Returns `true` if the button was clicked.
pub fn show_sized(
  ui: &mut egui::Ui,
  icon: &Icon,
  tint: egui::Color32,
  size: egui::Vec2,
) -> bool {
  let response = ui.add(
    egui::Button::image(icon_to_image(icon).fit_to_exact_size(size).tint(tint))
      .frame(false),
  );

  if response.hovered() {
    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
  }

  response.clicked()
}
