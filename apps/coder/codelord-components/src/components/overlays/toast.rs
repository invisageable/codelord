//! Single toast notification rendering.

use crate::assets::icon::icon_to_image;
use crate::components::atoms::stripe_button;

use codelord_core::ecs::world::World;
use codelord_core::icon::components::{Feedback, Icon};
use codelord_core::toast::components::{Toast, ToastStatus};

use eframe::egui;

/// Result from toast interaction.
pub struct ToastInteraction {
  /// Toast should be dismissed.
  pub dismiss: bool,
  /// Action button that was clicked.
  pub action_clicked: Option<String>,
}

/// Renders a single toast notification.
/// Returns interaction result (dismiss and/or action clicked).
pub fn show(
  ui: &mut egui::Ui,
  world: &mut World,
  toast: &Toast,
  position: egui::Pos2,
  width: f32,
  height: f32,
) -> ToastInteraction {
  let layer_id = egui::LayerId::new(
    egui::Order::Foreground,
    egui::Id::new("toast").with(toast.id.as_u64()),
  );

  let has_actions = !toast.actions.is_empty();
  let actual_height = if has_actions { height + 36.0 } else { height };
  let rect =
    egui::Rect::from_min_size(position, egui::vec2(width, actual_height));

  // Background and border
  {
    let painter = ui.ctx().layer_painter(layer_id);
    let bg_color = egui::Color32::from_rgba_premultiplied(0, 0, 0, 255);
    painter.rect_filled(rect, 0.0, bg_color);
    painter.rect_stroke(
      rect,
      0.0,
      egui::Stroke::new(1.0, egui::Color32::from_gray(40)),
      egui::StrokeKind::Outside,
    );

    // Icon area (left 50px, full height of message area)
    let icon_rect =
      egui::Rect::from_min_size(position, egui::vec2(50.0, height));
    painter.rect_filled(
      icon_rect,
      0.0,
      egui::Color32::from_rgba_premultiplied(0, 0, 0, 255),
    );
  }

  // Draw icon
  let (r, g, b) = toast.status.color_rgb();
  let icon_tint = egui::Color32::from_rgb(r, g, b);
  let icon_size = egui::vec2(16.0, 16.0);
  let icon_rect = egui::Rect::from_min_size(position, egui::vec2(50.0, height));
  let icon_image_rect =
    egui::Rect::from_center_size(icon_rect.center(), icon_size);

  let feedback_icon = match toast.status {
    ToastStatus::Info => Feedback::Info,
    ToastStatus::Success => Feedback::Success,
    ToastStatus::Warning => Feedback::Warning,
    ToastStatus::Error => Feedback::Alert,
  };

  let icon_image = icon_to_image(&Icon::Feedback(feedback_icon))
    .tint(icon_tint)
    .fit_to_exact_size(icon_size);

  let icon_ui = ui.new_child(
    egui::UiBuilder::new()
      .layer_id(layer_id)
      .max_rect(icon_image_rect),
  );
  icon_image.paint_at(&icon_ui, icon_image_rect);

  // Close button (top-right)
  let close_button_size = egui::vec2(16.0, 16.0);
  let close_button_rect = egui::Rect::from_min_size(
    egui::pos2(position.x + width - 24.0, position.y + 8.0),
    close_button_size,
  );

  let close_icon = icon_to_image(&Icon::Close)
    .tint(egui::Color32::WHITE)
    .fit_to_exact_size(close_button_size);

  let close_ui = ui.new_child(
    egui::UiBuilder::new()
      .layer_id(layer_id)
      .max_rect(close_button_rect)
      .sense(egui::Sense::click()),
  );

  let close_response = close_ui.interact(
    close_button_rect,
    egui::Id::new("close_toast").with(toast.id.as_u64()),
    egui::Sense::click(),
  );

  close_icon.paint_at(&close_ui, close_button_rect);

  // Message text
  let text_alpha = (toast.animation.opacity * 255.0) as u8;
  let text_color =
    egui::Color32::from_rgba_premultiplied(255, 255, 255, text_alpha);
  let text_rect = egui::Rect::from_min_size(
    egui::pos2(position.x + 60.0, position.y),
    egui::vec2(width - 70.0, height),
  );

  ui.ctx()
    .layer_painter(layer_id)
    .with_clip_rect(text_rect)
    .text(
      egui::pos2(position.x + 60.0, position.y + height / 2.0),
      egui::Align2::LEFT_CENTER,
      &toast.message,
      egui::FontId::proportional(12.0),
      text_color,
    );

  // Action buttons
  let mut action_clicked = None;

  if has_actions {
    let button_area_rect = egui::Rect::from_min_size(
      egui::pos2(position.x + 60.0, position.y + height + 4.0),
      egui::vec2(width - 70.0, 28.0),
    );

    let mut button_ui = ui.new_child(
      egui::UiBuilder::new()
        .layer_id(layer_id)
        .max_rect(button_area_rect),
    );

    button_ui.horizontal(|ui| {
      for action in &toast.actions {
        let clicked = if action.stripe {
          stripe_button::show(ui, world, &action.label, egui::vec2(100.0, 24.0))
            .clicked()
        } else {
          let button = if action.primary {
            egui::Button::new(&action.label)
              .fill(egui::Color32::from_rgb(r, g, b))
          } else {
            egui::Button::new(&action.label)
          };
          ui.add(button).clicked()
        };

        if clicked {
          action_clicked = Some(action.id.clone());
        }
      }
    });
  }

  ToastInteraction {
    dismiss: close_response.clicked() || action_clicked.is_some(),
    action_clicked,
  }
}
