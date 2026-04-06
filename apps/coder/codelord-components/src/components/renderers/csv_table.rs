//! CSV table renderer with production-ready layout.
//!
//! Renders parsed CSV data as a formatted table using egui's TableBuilder.
//! Follows clean, minimal design principles.

use codelord_core::previews::csv::CsvData;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

/// Renders a CSV table with headers, alternating row colors, and scroll
/// support.
pub fn render(ui: &mut egui::Ui, csv_data: &CsvData) {
  if let Some(error) = &csv_data.parse_error {
    render_error(ui, error);
    return;
  }

  if csv_data.headers.is_empty() || csv_data.rows.is_empty() {
    render_empty(ui);
    return;
  }

  let column_count = csv_data.headers.len();

  // Pre-calculate column widths based on all data (headers + all rows)
  // This prevents columns from jumping around during scrolling
  let mut column_widths = vec![100.0_f32; column_count]; // Minimum width
  let char_width = 7.0_f32; // Approximate character width in pixels
  let padding = 30.0_f32; // Extra padding per column

  // Calculate max width for each column based on headers
  for (col_idx, header) in csv_data.headers.iter().enumerate() {
    let header_width = (header.len() as f32 * char_width) + padding;
    column_widths[col_idx] = column_widths[col_idx].max(header_width);
  }

  // Calculate max width for each column based on all row data
  for row in csv_data.rows.iter() {
    for (col_idx, cell) in row.iter().enumerate() {
      if col_idx < column_count {
        let cell_width = (cell.len() as f32 * char_width) + padding;
        column_widths[col_idx] = column_widths[col_idx].max(cell_width);
      }
    }
  }

  // Cap maximum column width to prevent extremely wide columns
  for width in column_widths.iter_mut() {
    *width = width.min(400.0_f32);
  }

  // Main scroll area
  egui::ScrollArea::both()
    .auto_shrink([false; 2])
    .show(ui, |ui| {
      let mut table = TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .min_scrolled_height(0.0)
        .sense(egui::Sense::hover());

      // Add columns with pre-calculated widths
      // Last column gets remainder to fill available space
      for (i, &width) in column_widths.iter().enumerate() {
        if i == column_count - 1 {
          table = table.column(Column::remainder().at_least(width));
        } else {
          table = table.column(Column::initial(width).at_least(width));
        }
      }

      let table = table;

      table
        .header(28.0, |mut header| {
          // Render header row
          for col_name in csv_data.headers.iter() {
            header.col(|ui| {
              ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.label(
                  egui::RichText::new(col_name)
                    .strong()
                    .size(14.0)
                    .color(egui::Color32::from_rgb(204, 253, 62)),
                );
              });
            });
          }
        })
        .body(|body| {
          body.rows(24.0, csv_data.rows.len(), |mut row| {
            let row_index = row.index();
            let row_data = &csv_data.rows[row_index];

            for cell in row_data.iter() {
              row.col(|ui| {
                ui.horizontal_centered(|ui| {
                  ui.add_space(12.0);

                  // Determine text color based on content type
                  let text_color = if is_number(cell) {
                    egui::Color32::from_rgb(181, 206, 168) // Green for numbers
                  } else if cell.is_empty() {
                    egui::Color32::from_gray(100) // Dim for empty
                  } else {
                    egui::Color32::from_gray(220) // Normal for text
                  };

                  ui.label(
                    egui::RichText::new(cell).color(text_color).size(13.0),
                  );
                });
              });
            }
          });
        });

      // Footer with row count info
      // ui.add_space(16.0);
      // ui.separator();
      // ui.add_space(8.0);

      // let footer_text = if csv_data.is_truncated() {
      //   format!(
      //     "Showing {} of {} rows (limited for performance)",
      //     csv_data.rows.len(),
      //     csv_data.total_rows
      //   )
      // } else {
      //   format!(
      //     "{} rows × {} columns",
      //     csv_data.rows.len(),
      //     csv_data.headers.len()
      //   )
      // };

      // ui.horizontal(|ui| {
      //   ui.add_space(12.0);
      //   ui.label(
      //     egui::RichText::new(footer_text)
      //       .size(12.0)
      //       .color(egui::Color32::from_gray(150)),
      //   );
      // });
    });
}

/// Renders an error message for malformed CSV.
fn render_error(ui: &mut egui::Ui, error: &str) {
  ui.centered_and_justified(|ui| {
    ui.vertical_centered(|ui| {
      ui.heading(
        egui::RichText::new("CSV Parse Error")
          .color(egui::Color32::from_rgb(255, 100, 100))
          .size(24.0),
      );
      ui.add_space(16.0);
      ui.label(
        egui::RichText::new(error)
          .size(14.0)
          .color(egui::Color32::from_gray(180)),
      );
    });
  });
}

/// Renders a message for empty CSV files.
fn render_empty(ui: &mut egui::Ui) {
  ui.centered_and_justified(|ui| {
    ui.vertical_centered(|ui| {
      ui.heading(
        egui::RichText::new("Empty CSV")
          .color(egui::Color32::from_rgb(204, 253, 62))
          .size(24.0),
      );
      ui.add_space(16.0);
      ui.label(
        egui::RichText::new("This CSV file has no data")
          .size(14.0)
          .color(egui::Color32::from_gray(180)),
      );
    });
  });
}

/// Helper function to detect if a cell contains a number.
fn is_number(s: &str) -> bool {
  s.parse::<f64>().is_ok()
}
