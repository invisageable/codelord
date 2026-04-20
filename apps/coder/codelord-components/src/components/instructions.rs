//! Instructions component for displaying command shortcuts.
//!
//! This component shows helpful keyboard shortcuts and commands
//! in a styled format with title + separator using table builder.
//!
//! Reads instruction data from `InstructionsResource` ECS resource.

use crate::assets::{font, icon::icon_to_image};

use codelord_core::ecs::world::World;
use codelord_core::instruction::components::{
  InstructionKey, InstructionSection,
};
use codelord_core::instruction::resources::InstructionsResource;

use eframe::egui;
use egui::emath::GuiRounding as _;
use egui_extras::{Column, TableBuilder};

/// Shows all instruction sections from the ECS resource.
///
/// Reads `InstructionsResource` from the World and renders all sections.
pub fn show(ui: &mut egui::Ui, world: &World) {
  let Some(instructions) = world.get_resource::<InstructionsResource>() else {
    return;
  };

  for section in &instructions.sections {
    render_section(ui, section);
  }
}

/// Renders a section of instructions with a title and separator line.
/// Max width is 400px.
fn render_section(ui: &mut egui::Ui, section: &InstructionSection) {
  const MAX_WIDTH: f32 = 400.0;

  let visuals = ui.style().visuals.clone();

  ui.horizontal(|ui| {
    // Draw title using label to properly allocate space
    let font_id = font::cirka(14.0);

    let title_response = ui.label(
      egui::RichText::new(section.title)
        .color(egui::Color32::from_rgb(204, 253, 62)) // Green accent
        .font(font_id.clone())
        .extra_letter_spacing(1.6),
    );

    // Add spacing between title and line
    ui.add_space(8.0);

    // Draw horizontal separator line
    let line_y = title_response.rect.center().y;
    let line_start_x = ui.cursor().left();
    let line_end_x =
      line_start_x + (MAX_WIDTH - title_response.rect.width() - 8.0);

    ui.painter().line_segment(
      [
        egui::pos2(line_start_x, line_y),
        egui::pos2(line_end_x, line_y),
      ],
      egui::Stroke::new(0.5, egui::Color32::from_gray(30)),
    );
  });

  ui.add_space(20.0);

  // Center the table
  let table_id = format!("instructions_table_{}", section.title);

  TableBuilder::new(ui)
    .id_salt(table_id)
    .max_scroll_height(f32::INFINITY)
    // description.
    .column(Column::exact(200.0))
    // keys.
    .column(Column::exact(200.0))
    .body(|mut body| {
      for instruction in &section.instructions {
        body.row(32.0, |mut row| {
          // description column.
          row.col(|ui| {
            ui.with_layout(
              egui::Layout::left_to_right(egui::Align::Center),
              |ui| {
                ui.label(
                  egui::RichText::new(instruction.description)
                    .size(12.0)
                    .color(egui::Color32::from_gray(180))
                    .family(egui::FontFamily::Name(font::SUISSE_INTL.into())),
                );
              },
            );
          });
          // keys column.
          row.col(|ui| {
            ui.with_layout(
              egui::Layout::right_to_left(egui::Align::Center),
              |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for key in instruction.keys.iter().rev() {
                  render_key_badge(ui, key, &visuals);
                }
              },
            );
          });
        });
      }
    });

  ui.add_space(24.0);
}

/// Renders a keyboard key badge.
fn render_key_badge(
  ui: &mut egui::Ui,
  key: &InstructionKey,
  visuals: &egui::Visuals,
) {
  const PADDING: egui::Vec2 = egui::vec2(6.0, 6.0);

  match key {
    InstructionKey::Icon(icon) => {
      // Render icon key - same height as text keys, but only horizontal padding
      let icon_size = egui::vec2(16.0, 16.0);

      // Calculate text badge height to match
      let font_id = font::firacode(10.0);
      let text_height = ui.fonts_mut(|f| {
        f.layout_no_wrap(
          "A".to_string(),
          font_id,
          egui::Color32::from_gray(200),
        )
        .rect
        .height()
      });

      let badge_height = (text_height + PADDING.y * 2.0).round_ui();
      let total_size =
        egui::vec2((icon_size.x + PADDING.x * 2.0).round_ui(), badge_height);

      let (rect, _response) =
        ui.allocate_exact_size(total_size, egui::Sense::hover());

      let painter = ui.painter();

      // Draw badge background
      painter.rect(
        rect,
        0.0,
        visuals.widgets.inactive.bg_fill,
        egui::Stroke::new(1.0, egui::Color32::TRANSPARENT),
        egui::epaint::StrokeKind::Outside,
      );

      // Draw icon centered in badge
      let icon_rect = egui::Rect::from_center_size(rect.center(), icon_size);
      icon_to_image(icon)
        .tint(egui::Color32::WHITE)
        .fit_to_exact_size(icon_size)
        .paint_at(ui, icon_rect);
    }
    InstructionKey::Text(text) => {
      const TEXT_PADDING: egui::Vec2 = egui::vec2(11.0, 6.0);

      // Render text key
      let font_id = font::firacode(10.0);

      // Calculate badge size
      let text_galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(
          text.to_string(),
          font_id.clone(),
          egui::Color32::from_gray(200),
        )
      });

      let badge_size =
        (text_galley.rect.size() + TEXT_PADDING * 2.0).round_ui();

      let (rect, _response) =
        ui.allocate_exact_size(badge_size, egui::Sense::hover());

      // Draw badge background
      ui.painter().rect(
        rect,
        0.0,
        visuals.widgets.inactive.bg_fill,
        egui::Stroke::new(1.0, egui::Color32::TRANSPARENT),
        egui::epaint::StrokeKind::Outside,
      );

      // Draw key text
      ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font_id,
        egui::Color32::from_gray(200),
      );
    }
  }
}
