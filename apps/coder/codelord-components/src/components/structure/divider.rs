//! Divider component for visual separation.
//!
//! Supports:
//! - Horizontal/Vertical lines
//! - Horizontal lines with labels (left, center, right aligned)

use eframe::egui;

/// Axis for the divider.
#[derive(Debug, Clone, Copy, Default)]
pub enum Axis {
  #[default]
  Horizontal,
  Vertical,
}

/// Label alignment for horizontal dividers with text.
#[derive(Debug, Clone, Copy, Default)]
pub enum LabelAlign {
  Left,
  #[default]
  Center,
  Right,
}

/// Show a simple divider line.
pub fn show(ui: &mut egui::Ui, axis: Axis) {
  let color = ui.visuals().widgets.noninteractive.bg_stroke.color;
  let stroke = egui::Stroke::new(0.5_f32, color);
  let rect = ui.available_rect_before_wrap();

  match axis {
    Axis::Horizontal => {
      ui.painter().line_segment(
        [
          egui::pos2(rect.left(), ui.cursor().top()),
          egui::pos2(rect.right(), ui.cursor().top()),
        ],
        stroke,
      );
    }
    Axis::Vertical => {
      // Allocate space so cursor advances (needed for RTL layouts)
      let (rect, _) = ui.allocate_exact_size(
        egui::vec2(0.5, rect.height()),
        egui::Sense::hover(),
      );
      ui.painter().line_segment(
        [
          egui::pos2(rect.center().x, rect.top()),
          egui::pos2(rect.center().x, rect.bottom()),
        ],
        stroke,
      );
    }
  }
}

/// Show a horizontal divider with a label.
pub fn show_with_label(ui: &mut egui::Ui, label: &str, align: LabelAlign) {
  let visuals = ui.visuals();
  let line_color = visuals.widgets.noninteractive.bg_stroke.color;
  let text_color = visuals.weak_text_color();
  let stroke = egui::Stroke::new(1.0_f32, line_color);

  let available_width = ui.available_width();
  let font_id = egui::FontId::proportional(12.0);
  let padding = 8.0;

  // Measure text width
  let text_width = ui.fonts_mut(|f| {
    f.layout_no_wrap(label.to_string(), font_id.clone(), text_color)
      .size()
      .x
  });

  let left_x = ui.cursor().left();
  let right_x = left_x + available_width;
  let y = ui.cursor().top();

  match align {
    LabelAlign::Left => {
      // Label --- Line
      let text_end = left_x + text_width;

      ui.painter().text(
        egui::pos2(left_x, y),
        egui::Align2::LEFT_CENTER,
        label,
        font_id,
        text_color,
      );

      ui.painter().line_segment(
        [egui::pos2(text_end + padding, y), egui::pos2(right_x, y)],
        stroke,
      );
    }
    LabelAlign::Center => {
      // Line --- Label --- Line
      let center_x = left_x + available_width / 2.0;
      let text_start = center_x - text_width / 2.0;
      let text_end = center_x + text_width / 2.0;

      // Left line
      ui.painter().line_segment(
        [egui::pos2(left_x, y), egui::pos2(text_start - padding, y)],
        stroke,
      );

      // Center text
      ui.painter().text(
        egui::pos2(center_x, y),
        egui::Align2::CENTER_CENTER,
        label,
        font_id,
        text_color,
      );

      // Right line
      ui.painter().line_segment(
        [egui::pos2(text_end + padding, y), egui::pos2(right_x, y)],
        stroke,
      );
    }
    LabelAlign::Right => {
      // Line --- Label
      let text_start = right_x - text_width;

      ui.painter().line_segment(
        [egui::pos2(left_x, y), egui::pos2(text_start - padding, y)],
        stroke,
      );

      ui.painter().text(
        egui::pos2(right_x, y),
        egui::Align2::RIGHT_CENTER,
        label,
        font_id,
        text_color,
      );
    }
  }
}
