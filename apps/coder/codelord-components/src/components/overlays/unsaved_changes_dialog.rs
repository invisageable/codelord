use crate::assets::theme::get_theme;

use codelord_core::animation::resources::ActiveAnimations;
use codelord_core::ecs::world::World;
use codelord_core::tabbar::{UnsavedChangesDialog, UnsavedChangesResponse};

use eazy::{Curve, Easing};

use eframe::egui;

/// Animation duration in seconds.
const ANIMATION_DURATION: f64 = 0.4;

/// Renders the unsaved changes confirmation dialog.
/// Returns the user's response to the dialog.
pub fn show(ctx: &egui::Context, world: &mut World) -> UnsavedChangesResponse {
  let mut response = UnsavedChangesResponse::None;

  // Get dialog state
  let (is_visible, filename, animation_start) = {
    let dialog = world.resource::<UnsavedChangesDialog>();
    (
      dialog.is_visible(),
      dialog.filename.clone(),
      dialog.animation_start,
    )
  };

  if !is_visible {
    return response;
  }

  let current_time = ctx.input(|i| i.time);

  // Initialize animation start time if needed
  let animation_start = animation_start.unwrap_or_else(|| {
    world.resource_mut::<UnsavedChangesDialog>().animation_start =
      Some(current_time);

    current_time
  });

  // Calculate animation progress
  let elapsed = current_time - animation_start;
  let animation_t = (elapsed / ANIMATION_DURATION).min(1.0) as f32;

  // Track open/close animation via ActiveAnimations.
  if animation_t < 1.0
    && let Some(mut anim) = world.get_resource_mut::<ActiveAnimations>()
  {
    anim.increment();
  }

  let theme = get_theme(world);
  let screen_rect = ctx.content_rect();

  // Full-screen overlay
  egui::Area::new(egui::Id::new("unsaved_changes_overlay"))
    .fixed_pos(egui::pos2(0.0, 0.0))
    .order(egui::Order::Foreground)
    .show(ctx, |ui| {
      ui.painter().rect_filled(
        screen_rect,
        0.0,
        egui::Color32::from_black_alpha(180),
      );
    });

  // Dialog box with elastic bounce animation
  let base_size = egui::vec2(500.0, 200.0);
  let eased = Easing::OutElastic.y(animation_t);
  let scale = eased.clamp(0.01, 1.0);
  let dialog_size = base_size * scale;

  let dialog_center = screen_rect.center();

  egui::Area::new(egui::Id::new("unsaved_changes_dialog"))
    .fixed_pos(egui::pos2(
      dialog_center.x - dialog_size.x / 2.0,
      dialog_center.y - dialog_size.y / 2.0,
    ))
    .order(egui::Order::Tooltip)
    .show(ctx, |ui| {
      // Use base (background) and surface0 (border) from theme
      let bg_color = egui::Color32::from_rgba_unmultiplied(
        theme.base[0],
        theme.base[1],
        theme.base[2],
        255,
      );

      let border_color = egui::Color32::from_rgba_unmultiplied(
        theme.surface0[0],
        theme.surface0[1],
        theme.surface0[2],
        theme.surface0[3],
      );

      egui::Frame::new()
        .fill(bg_color)
        .stroke(egui::Stroke::new(1.0, border_color))
        .corner_radius(0.0)
        .inner_margin(20.0)
        .show(ui, |ui| {
          ui.set_min_size(dialog_size);
          ui.set_max_size(dialog_size);

          ui.vertical_centered(|ui| {
            // Title
            ui.heading("Unsaved Changes");
            ui.add_space(30.0);

            // Warning message
            ui.label(
              egui::RichText::new(format!(
                "Do you want to save the changes you made to \"{filename}\"?",
              ))
              .size(14.0),
            );

            ui.add_space(10.0);

            ui.label(
              egui::RichText::new(
                "Your changes will be lost if you don't save them.",
              )
              .size(12.0)
              .color(egui::Color32::GRAY),
            );

            ui.add_space(30.0);

            // Button row
            ui.horizontal(|ui| {
              ui.add_space((dialog_size.x - 360.0) / 2.0);

              // Don't Save button
              if ui
                .add_sized(
                  [110.0, 35.0],
                  egui::Button::new("Don't Save").corner_radius(0.0),
                )
                .clicked()
              {
                response = UnsavedChangesResponse::DontSave;
              }

              ui.add_space(10.0);

              // Cancel button
              if ui
                .add_sized(
                  [110.0, 35.0],
                  egui::Button::new("Cancel").corner_radius(0.0),
                )
                .clicked()
              {
                response = UnsavedChangesResponse::Cancel;
              }

              ui.add_space(10.0);

              // Save button (primary action - green)
              let green = egui::Color32::from_rgb(204, 253, 62);

              if ui
                .add_sized(
                  [110.0, 35.0],
                  egui::Button::new(
                    egui::RichText::new("Save").color(egui::Color32::BLACK),
                  )
                  .fill(green)
                  .corner_radius(0.0),
                )
                .clicked()
              {
                response = UnsavedChangesResponse::Save;
              }
            });
          });
        });
    });

  response
}
