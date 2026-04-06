use crate::assets::icon;

use codelord_core::button::components::{Button, ButtonContent, ButtonVariant};
use codelord_core::ecs::entity::Entity;
use codelord_core::ecs::world::World;
use codelord_core::ui::component::{Clickable, Hovered};

use eframe::egui;

/// Render a button with ECS entity state.
///
/// Reads Hovered and Clickable from entity.
/// Returns true if clicked.
pub fn show(ui: &mut egui::Ui, world: &mut World, entity: Entity) -> bool {
  let button = match world.get::<Button>(entity) {
    Some(b) => b.clone(),
    None => return false,
  };

  let is_enabled = world
    .get::<Clickable>(entity)
    .map(|c| c.is_enabled())
    .unwrap_or(true);

  let visuals = ui.style().visuals.clone();

  let (bg, bg_hovered, text_color) = match button.variant {
    ButtonVariant::Primary => (
      visuals.widgets.hovered.fg_stroke.color,
      visuals.widgets.hovered.fg_stroke.color.gamma_multiply(1.2),
      visuals.extreme_bg_color,
    ),
    ButtonVariant::Secondary => (
      visuals.widgets.noninteractive.bg_fill,
      visuals.widgets.inactive.bg_fill,
      visuals.text_color(),
    ),
    ButtonVariant::Ghost => (
      egui::Color32::TRANSPARENT,
      visuals.widgets.noninteractive.bg_fill,
      visuals.text_color(),
    ),
  };

  let button_widget = match &button.content {
    ButtonContent::Label(label) => {
      egui::Button::new(egui::RichText::new(*label).color(text_color))
    }
    ButtonContent::Icon(icn) => egui::Button::image(
      icon::icon_to_image(icn)
        .fit_to_exact_size(egui::Vec2::splat(12.0))
        .tint(text_color),
    ),
    ButtonContent::IconLabel(icn, label) => egui::Button::image_and_text(
      icon::icon_to_image(icn)
        .fit_to_exact_size(egui::Vec2::splat(12.0))
        .tint(text_color),
      egui::RichText::new(*label).color(text_color),
    ),
  };

  let response =
    ui.add_enabled(is_enabled, button_widget.frame(false).fill(bg));

  // Update hovered state in ECS (marker presence = hovered)
  if response.hovered() {
    if !world.entity(entity).contains::<Hovered>() {
      world.entity_mut(entity).insert(Hovered);
    }
  } else if world.entity(entity).contains::<Hovered>() {
    world.entity_mut(entity).remove::<Hovered>();
  }

  // Use hovered color if hovered
  if response.hovered() {
    ui.painter().rect_filled(
      response.rect,
      visuals.widgets.noninteractive.corner_radius,
      bg_hovered,
    );
  }

  response.clicked()
}
