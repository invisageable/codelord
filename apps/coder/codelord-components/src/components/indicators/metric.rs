//! Metric indicator component.
//!
//! Displays a label with info icon and a counter value with unit.
//! Clicking the info icon shows a popup with additional information.

use crate::assets::icon::icon_to_image;

use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::ActiveAnimations;
use codelord_core::ecs::entity::Entity;
use codelord_core::ecs::world::World;
use codelord_core::icon::components::{Feedback, Icon};
use codelord_core::ui::component::{Metric, MetricUnit};

use eframe::egui;

/// Render a metric indicator from the ECS world.
///
/// Updates the counter animation and tracks active animations for repaint.
pub fn show(ui: &mut egui::Ui, world: &mut World, entity: Entity) {
  // Get delta time for animation update
  let delta = world
    .get_resource::<DeltaTime>()
    .map(|dt| dt.delta())
    .unwrap_or(0.016);

  // Update animation and track completion
  let (value, label, info, unit, unit_mode, color, is_integer, completed) =
    if let Some(mut metric) = world.get_mut::<Metric>(entity) {
      // update() returns true when animation just completed this frame
      let completed = metric.update(delta);

      (
        metric.value(),
        metric.label,
        metric.info,
        metric.unit,
        metric.unit_mode,
        metric.color,
        metric.is_integer,
        completed,
      )
    } else {
      return;
    };

  // Decrement active animations when animation completes
  if completed
    && let Some(mut active_anims) = world.get_resource_mut::<ActiveAnimations>()
  {
    active_anims.decrement();
  }

  let popup_id = ui.make_persistent_id(entity);
  let color = egui::Color32::from_rgba_unmultiplied(
    color[0], color[1], color[2], color[3],
  );

  // Info icon color (gray)
  let icon_color = egui::Color32::from_gray(100);

  ui.vertical(|ui| {
    ui.spacing_mut().item_spacing.y = 0.0;

    // Label row with info icon
    ui.horizontal(|ui| {
      ui.add_space(8.0);
      ui.label(egui::RichText::new(label).color(color));

      let icon_response = ui.add(
        icon_to_image(&Icon::Feedback(Feedback::Info))
          .fit_to_exact_size(egui::vec2(16.0, 16.0))
          .tint(icon_color)
          .sense(egui::Sense::click()),
      );

      if icon_response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Help);
      }

      let stroke_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
      egui::Popup::from_toggle_button_response(&icon_response)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(
          egui::Frame::popup(ui.style())
            .corner_radius(egui::CornerRadius::ZERO)
            .stroke(egui::Stroke::new(1.0_f32, stroke_color)),
        )
        .show(|ui| {
          ui.set_width(200.0);
          ui.set_height(100.0);
          ui.label(info);
        });
    });

    // Counter value with unit
    let (value_text, display_unit) = match unit_mode {
      MetricUnit::Static => {
        let text = if is_integer {
          format!("{}", value as i64)
        } else {
          format!("{value:.3}")
        };
        (text, unit)
      }
      MetricUnit::Time => {
        // Value is in milliseconds, convert to appropriate unit
        let micros = value * 1000.0;
        if micros < 1000.0 {
          (format!("{micros:.3}"), "μs")
        } else if value < 1000.0 {
          (format!("{value:.3}"), "ms")
        } else {
          let seconds = value / 1000.0;
          (format!("{seconds:.3}"), "s")
        }
      }
    };

    ui.horizontal(|ui| {
      ui.add_space(8.0);
      ui.label(egui::RichText::new(value_text).size(36.0).color(color));
      ui.label(
        egui::RichText::new(format!(" {display_unit}"))
          .size(24.0)
          .color(color),
      );
    });
  });
}
