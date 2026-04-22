//! Color picker popup for inline color editing.
//!
//! Shows a color picker when a color preview square is clicked.
//! Allows editing the color and updates the source code.

use codelord_core::color::ColorPickerState;
use codelord_core::ecs::entity::Entity;
use codelord_core::ecs::world::World;
use codelord_core::text_editor::components::TextBuffer;

use eframe::egui;

/// Event to replace text in the buffer.
pub struct ReplaceTextEvent {
  pub entity: Entity,
  pub start: usize,
  pub end: usize,
  pub text: String,
}

/// Renders the color picker popup if it's open.
///
/// Returns Some(ReplaceTextEvent) if the color was changed and should be
/// applied.
pub fn show(ui: &mut egui::Ui, world: &mut World) -> Option<ReplaceTextEvent> {
  // Check if picker is open.
  let picker_open = world
    .get_resource::<ColorPickerState>()
    .map(|s| s.open)
    .unwrap_or(false);

  if !picker_open {
    return None;
  }

  // Clone state to avoid borrow issues.
  let state = world.get_resource::<ColorPickerState>()?.clone();

  let mut result: Option<ReplaceTextEvent> = None;
  let mut should_close = false;
  let mut new_color = state.color;

  // Create popup window at the color's position.
  let popup_id = ui.id().with("color_picker_popup");

  egui::Area::new(popup_id)
    .order(egui::Order::Foreground)
    .fixed_pos(egui::pos2(state.position.0, state.position.1 + 20.0))
    .show(ui.ctx(), |ui| {
      egui::Frame::popup(ui.style())
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
          ui.set_min_width(220.0);

          // Header with close button.
          ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Color Picker").strong().size(14.0));

            ui.with_layout(
              egui::Layout::right_to_left(egui::Align::Center),
              |ui| {
                if ui.small_button("✕").clicked() {
                  should_close = true;
                }
              },
            );
          });

          ui.add_space(8.0);

          // Color picker widget.
          ui.horizontal(|ui| {
            // Color preview (large).
            let preview_size = egui::vec2(48.0, 48.0);
            let (rect, _) =
              ui.allocate_exact_size(preview_size, egui::Sense::hover());

            let preview_color = egui::Color32::from_rgba_unmultiplied(
              (new_color[0] * 255.0) as u8,
              (new_color[1] * 255.0) as u8,
              (new_color[2] * 255.0) as u8,
              (new_color[3] * 255.0) as u8,
            );

            // Draw checkerboard for alpha preview.
            draw_checkerboard(ui.painter(), rect, 8.0);
            ui.painter().rect_filled(rect, 4.0, preview_color);
            ui.painter().rect_stroke(
              rect,
              4.0,
              egui::Stroke::new(1.0_f32, egui::Color32::from_gray(60)),
              egui::StrokeKind::Inside,
            );

            ui.add_space(12.0);

            // RGB sliders.
            ui.vertical(|ui| {
              ui.horizontal(|ui| {
                ui.label(
                  egui::RichText::new("R")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(255, 100, 100)),
                );
                ui.add(
                  egui::Slider::new(&mut new_color[0], 0.0..=1.0)
                    .show_value(false),
                );
              });
              ui.horizontal(|ui| {
                ui.label(
                  egui::RichText::new("G")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(100, 255, 100)),
                );
                ui.add(
                  egui::Slider::new(&mut new_color[1], 0.0..=1.0)
                    .show_value(false),
                );
              });
              ui.horizontal(|ui| {
                ui.label(
                  egui::RichText::new("B")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(100, 100, 255)),
                );
                ui.add(
                  egui::Slider::new(&mut new_color[2], 0.0..=1.0)
                    .show_value(false),
                );
              });
              ui.horizontal(|ui| {
                ui.label(
                  egui::RichText::new("A")
                    .size(11.0)
                    .color(egui::Color32::from_gray(180)),
                );
                ui.add(
                  egui::Slider::new(&mut new_color[3], 0.0..=1.0)
                    .show_value(false),
                );
              });
            });
          });

          ui.add_space(8.0);

          // Hex input.
          let hex = format!(
            "#{:02x}{:02x}{:02x}",
            (new_color[0] * 255.0) as u8,
            (new_color[1] * 255.0) as u8,
            (new_color[2] * 255.0) as u8,
          );

          ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Hex:").size(12.0));
            ui.label(
              egui::RichText::new(&hex)
                .monospace()
                .size(12.0)
                .color(egui::Color32::from_rgb(204, 253, 62)),
            );
          });

          ui.add_space(12.0);

          // Apply button.
          ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
              if let (Some(entity), Some((start, end))) =
                (state.entity, state.byte_range)
              {
                // Format the new color in the original format.
                let mut temp_state = state.clone();
                temp_state.color = new_color;
                let new_text = temp_state.format_color();

                result = Some(ReplaceTextEvent {
                  entity,
                  start,
                  end,
                  text: new_text,
                });
              }
              should_close = true;
            }

            if ui.button("Cancel").clicked() {
              should_close = true;
            }
          });
        });
    });

  // Handle click outside to close.
  if ui.input(|i| i.pointer.any_click()) {
    let popup_rect = egui::Rect::from_min_size(
      egui::pos2(state.position.0, state.position.1 + 20.0),
      egui::vec2(260.0, 200.0),
    );

    if let Some(pos) = ui.input(|i| i.pointer.hover_pos())
      && !popup_rect.contains(pos)
    {
      should_close = true;
    }
  }

  // Update state.
  if should_close {
    if let Some(mut picker_state) = world.get_resource_mut::<ColorPickerState>()
    {
      picker_state.close();
    }
  } else if new_color != state.color
    && let Some(mut picker_state) = world.get_resource_mut::<ColorPickerState>()
  {
    picker_state.color = new_color;
  }

  result
}

/// Draws a checkerboard pattern for alpha preview.
fn draw_checkerboard(
  painter: &egui::Painter,
  rect: egui::Rect,
  cell_size: f32,
) {
  let light = egui::Color32::from_gray(200);
  let dark = egui::Color32::from_gray(120);

  let cols = (rect.width() / cell_size).ceil() as i32;
  let rows = (rect.height() / cell_size).ceil() as i32;

  for row in 0..rows {
    for col in 0..cols {
      let color = if (row + col) % 2 == 0 { light } else { dark };

      let cell_rect = egui::Rect::from_min_size(
        egui::pos2(
          rect.min.x + col as f32 * cell_size,
          rect.min.y + row as f32 * cell_size,
        ),
        egui::vec2(cell_size, cell_size),
      )
      .intersect(rect);

      painter.rect_filled(cell_rect, 0.0, color);
    }
  }
}

/// Applies a replace text event to the buffer.
pub fn apply_replace_event(world: &mut World, event: ReplaceTextEvent) {
  // Get the buffer and replace the text.
  if let Some(mut buffer) = world.get_mut::<TextBuffer>(event.entity) {
    // Convert byte range to char range.
    let start_char = buffer.rope.byte_to_char(event.start);
    let end_char = buffer.rope.byte_to_char(event.end);

    // Remove old text and insert new.
    buffer.rope.remove(start_char..end_char);
    buffer.rope.insert(start_char, &event.text);
  }
}
