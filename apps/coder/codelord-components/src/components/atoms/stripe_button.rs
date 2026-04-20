//! Stripe button - animated diagonal stripes on hover.

use crate::components::effects::stripe;

use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::ecs::world::World;

use eframe::egui;

/// Button style variant.
#[derive(Debug, Clone, Copy, Default)]
pub enum ButtonStyle {
  /// Default style from theme.
  #[default]
  Default,
  /// Primary action (green background).
  Primary,
  /// Secondary action (black/dark background).
  Secondary,
}

/// Renders a button with animated diagonal stripes on hover.
pub fn show(
  ui: &mut egui::Ui,
  world: &mut World,
  text: &str,
  size: egui::Vec2,
) -> egui::Response {
  show_styled(ui, world, text, size, ButtonStyle::Default)
}

/// Renders a styled button with animated diagonal stripes on hover.
pub fn show_styled(
  ui: &mut egui::Ui,
  world: &mut World,
  text: &str,
  size: egui::Vec2,
  style: ButtonStyle,
) -> egui::Response {
  let (button_id, button_rect) = ui.allocate_space(size);
  let button_response =
    ui.interact(button_rect, button_id, egui::Sense::click());

  let is_hovered = button_response.hovered();

  let (
    primary_color,
    secondary_color,
    tertiary_color,
    border_color,
    stripe_color,
  ) = {
    let visuals = ui.visuals();
    (
      visuals.widgets.hovered.fg_stroke.color,
      visuals.widgets.active.bg_fill,
      visuals.widgets.active.fg_stroke.color,
      visuals.widgets.active.bg_stroke.color,
      visuals.widgets.noninteractive.bg_stroke.color,
    )
  };

  // Determine colors based on style
  let (bg_color, text_color, hover_bg, hover_text) = match style {
    ButtonStyle::Default => (
      primary_color,
      secondary_color,
      secondary_color,
      tertiary_color,
    ),
    ButtonStyle::Primary => (
      primary_color,
      secondary_color,
      secondary_color,
      tertiary_color,
    ),
    ButtonStyle::Secondary => (
      egui::Color32::BLACK,
      egui::Color32::from_gray(180),
      egui::Color32::from_gray(30),
      egui::Color32::WHITE,
    ),
  };

  let final_bg = if is_hovered { hover_bg } else { bg_color };
  let final_text = if is_hovered { hover_text } else { text_color };

  ui.painter()
    .rect_filled(button_rect, egui::CornerRadius::ZERO, final_bg);

  if is_hovered {
    let prev_clip_rect = ui.clip_rect();
    ui.set_clip_rect(button_rect);
    stripe::show(ui, button_rect, stripe_color);
    ui.set_clip_rect(prev_clip_rect);

    ui.painter().rect_stroke(
      button_rect,
      egui::CornerRadius::ZERO,
      egui::Stroke::new(1.0_f32, border_color),
      egui::StrokeKind::Outside,
    );

    if let Some(mut cont) = world.get_resource_mut::<ContinuousAnimations>() {
      cont.set_stripe_active();
    }
  } else if matches!(style, ButtonStyle::Secondary) {
    // Secondary style has border when not hovered too
    ui.painter().rect_stroke(
      button_rect,
      egui::CornerRadius::ZERO,
      egui::Stroke::new(1.0_f32, egui::Color32::from_gray(50)),
      egui::StrokeKind::Outside,
    );
  }

  let text_pos = button_rect.center();
  ui.painter().text(
    text_pos,
    egui::Align2::CENTER_CENTER,
    text,
    egui::FontId::new(14.0, egui::FontFamily::Proportional),
    final_text,
  );

  button_response
}
