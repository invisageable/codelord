//! XLS/XLSX spreadsheet renderer.
//!
//! Renders Excel files as a formatted table with sheet tabs.
//! Simple design: sheet tabs at top (if multiple), data table below.

use crate::assets::icon::icon_to_image;

use codelord_core::icon::components::{Arrow, Icon};
use codelord_core::previews::xls::XlsPreviewState;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

/// Actions that can be triggered by the XLS renderer.
#[derive(Debug, Clone)]
pub enum XlsAction {
  /// User clicked on a sheet tab to switch sheets.
  SelectSheet(usize),
  /// User clicked pagination to change page.
  ChangePage(usize),
}

/// Renders the XLS preview UI.
/// Returns an action if the user interacted with the UI.
pub fn render(ui: &mut egui::Ui, state: &XlsPreviewState) -> Option<XlsAction> {
  let Some(data) = &state.cached_data else {
    render_empty(ui, "No data loaded");
    return None;
  };

  if let Some(error) = &data.parse_error {
    render_error(ui, error);
    return None;
  }

  if data.headers.is_empty() && data.rows.is_empty() {
    render_empty(ui, "Empty spreadsheet");
    return None;
  }

  let mut action: Option<XlsAction> = None;

  // Get paginated rows
  let current_page = state.current_page;
  let rows_per_page = state.rows_per_page;
  let start = current_page * rows_per_page;
  let end = (start + rows_per_page).min(data.rows.len());

  // Sheet tabs at top (only if multiple sheets)
  if data.sheet_names.len() > 1 {
    egui::Panel::top("xls_sheet_tabs")
      .exact_size(28.0)
      .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 0)))
      .show_inside(ui, |ui| {
        if let Some(sheet_action) =
          render_sheet_tabs(ui, &data.sheet_names, data.selected_sheet)
        {
          action = Some(sheet_action);
        }
      });
  }

  // Footer with pagination and info
  let total_pages = state.total_pages();
  egui::Panel::bottom("xls_footer")
    .exact_size(24.0)
    .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 4)))
    .show_inside(ui, |ui| {
      if let Some(page_action) = render_footer(
        ui,
        current_page,
        total_pages,
        end - start,
        data.headers.len(),
        data.total_rows,
      ) {
        action = Some(page_action);
      }
    });

  // Main data table
  egui::CentralPanel::default()
    .frame(egui::Frame::NONE)
    .show_inside(ui, |ui| {
      render_data_table(ui, &data.headers, &data.rows[start..end], start);
    });

  action
}

/// Renders sheet tabs for switching between sheets.
fn render_sheet_tabs(
  ui: &mut egui::Ui,
  sheet_names: &[String],
  selected_sheet: usize,
) -> Option<XlsAction> {
  let mut action = None;

  ui.horizontal_centered(|ui| {
    ui.spacing_mut().item_spacing.x = 4.0;

    for (idx, name) in sheet_names.iter().enumerate() {
      let is_selected = idx == selected_sheet;

      let text_color = if is_selected {
        egui::Color32::from_rgb(204, 253, 62)
      } else {
        egui::Color32::from_gray(180)
      };

      let button = egui::Button::new(
        egui::RichText::new(name).color(text_color).size(12.0),
      )
      .fill(if is_selected {
        egui::Color32::from_rgba_unmultiplied(204, 253, 62, 40)
      } else {
        egui::Color32::TRANSPARENT
      })
      .stroke(egui::Stroke::NONE)
      .corner_radius(2.0);

      if ui.add(button).clicked() && !is_selected {
        action = Some(XlsAction::SelectSheet(idx));
      }
    }
  });

  action
}

/// Renders the footer with pagination and stats.
fn render_footer(
  ui: &mut egui::Ui,
  current_page: usize,
  total_pages: usize,
  row_count: usize,
  col_count: usize,
  total_rows: usize,
) -> Option<XlsAction> {
  let mut action = None;

  let visuals = ui.style().visuals.clone();
  let enabled_tint = visuals.widgets.inactive.fg_stroke.color;
  let disabled_tint = egui::Color32::from_gray(60);

  let can_go_left = current_page > 0;
  let can_go_right = current_page + 1 < total_pages;

  ui.horizontal_centered(|ui| {
    // Left side: row/column info
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
      if total_rows > row_count {
        ui.label(
          egui::RichText::new(format!("{row_count} of {total_rows} rows"))
            .color(egui::Color32::from_gray(150))
            .size(11.0),
        );
      } else {
        ui.label(
          egui::RichText::new(format!("{row_count} rows"))
            .color(egui::Color32::from_gray(150))
            .size(11.0),
        );
      }

      ui.label(
        egui::RichText::new("•")
          .color(egui::Color32::from_gray(80))
          .size(11.0),
      );

      ui.label(
        egui::RichText::new(format!("{col_count} columns"))
          .color(egui::Color32::from_gray(150))
          .size(11.0),
      );
    });

    // Right side: pagination controls
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      // Right arrow
      let right_response = ui.add_enabled(
        can_go_right,
        egui::Button::image(
          icon_to_image(&Icon::Arrow(Arrow::Right))
            .fit_to_exact_size(egui::vec2(12.0, 12.0))
            .tint(if can_go_right {
              enabled_tint
            } else {
              disabled_tint
            }),
        )
        .frame(false),
      );

      if can_go_right && right_response.clicked() {
        action = Some(XlsAction::ChangePage(current_page + 1));
      }

      // Page indicator
      ui.label(
        egui::RichText::new(format!("{} / {}", current_page + 1, total_pages))
          .color(egui::Color32::from_gray(150))
          .size(11.0),
      );

      // Left arrow
      let left_response = ui.add_enabled(
        can_go_left,
        egui::Button::image(
          icon_to_image(&Icon::Arrow(Arrow::Left))
            .fit_to_exact_size(egui::vec2(12.0, 12.0))
            .tint(if can_go_left {
              enabled_tint
            } else {
              disabled_tint
            }),
        )
        .frame(false),
      );

      if can_go_left && left_response.clicked() {
        action = Some(XlsAction::ChangePage(current_page.saturating_sub(1)));
      }
    });
  });

  action
}

/// Renders the data table.
fn render_data_table(
  ui: &mut egui::Ui,
  headers: &[String],
  rows: &[Vec<String>],
  row_offset: usize,
) {
  if headers.is_empty() {
    return;
  }

  let column_count = headers.len();

  // Calculate column widths
  let mut column_widths = vec![100.0_f32; column_count];
  let char_width = 7.0_f32;
  let padding = 30.0_f32;

  for (col_idx, header) in headers.iter().enumerate() {
    let header_width = (header.len() as f32 * char_width) + padding;
    column_widths[col_idx] = column_widths[col_idx].max(header_width);
  }

  for row in rows.iter() {
    for (col_idx, cell) in row.iter().enumerate() {
      if col_idx < column_count {
        let cell_width = (cell.len() as f32 * char_width) + padding;
        column_widths[col_idx] = column_widths[col_idx].max(cell_width);
      }
    }
  }

  // Cap max width
  for width in column_widths.iter_mut() {
    *width = width.min(400.0_f32);
  }

  let row_num_width = 50.0_f32;

  egui::ScrollArea::both()
    .id_salt("xls_data_table")
    .auto_shrink([false; 2])
    .show(ui, |ui| {
      let mut table = TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .min_scrolled_height(0.0)
        .sense(egui::Sense::hover());

      // Row number column
      table = table.column(Column::exact(row_num_width));

      // Data columns
      for (i, &width) in column_widths.iter().enumerate() {
        if i == column_count - 1 {
          table = table.column(Column::remainder().at_least(width));
        } else {
          table = table.column(Column::initial(width).at_least(width));
        }
      }

      table
        .header(28.0, |mut header| {
          // Row number header
          header.col(|ui| {
            ui.horizontal_centered(|ui| {
              ui.add_space(8.0);
              ui.label(
                egui::RichText::new("#")
                  .strong()
                  .size(13.0)
                  .color(egui::Color32::from_gray(120)),
              );
            });
          });

          // Column headers
          for col_name in headers.iter() {
            header.col(|ui| {
              ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.label(
                  egui::RichText::new(col_name)
                    .strong()
                    .size(13.0)
                    .color(egui::Color32::from_rgb(204, 253, 62)),
                );
              });
            });
          }
        })
        .body(|body| {
          body.rows(24.0, rows.len(), |mut row| {
            let row_index = row.index();
            let row_data = &rows[row_index];

            // Row number
            row.col(|ui| {
              ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                ui.label(
                  egui::RichText::new(format!(
                    "{}",
                    row_offset + row_index + 1
                  ))
                  .size(12.0)
                  .color(egui::Color32::from_gray(100)),
                );
              });
            });

            // Data cells
            for (col_idx, cell) in row_data.iter().enumerate() {
              if col_idx >= column_count {
                break;
              }

              row.col(|ui| {
                ui.horizontal_centered(|ui| {
                  ui.add_space(12.0);

                  let text_color = if is_number(cell) {
                    egui::Color32::from_rgb(181, 206, 168) // Green for numbers
                  } else if cell.is_empty() {
                    egui::Color32::from_gray(80)
                  } else {
                    egui::Color32::from_gray(220)
                  };

                  ui.label(
                    egui::RichText::new(cell).color(text_color).size(13.0),
                  );
                });
              });
            }
          });
        });
    });
}

/// Renders an error message.
fn render_error(ui: &mut egui::Ui, error: &str) {
  ui.centered_and_justified(|ui| {
    ui.vertical_centered(|ui| {
      ui.heading(
        egui::RichText::new("Error")
          .color(egui::Color32::from_rgb(255, 100, 100))
          .size(18.0),
      );
      ui.add_space(12.0);
      ui.label(
        egui::RichText::new(error)
          .size(13.0)
          .color(egui::Color32::from_gray(150)),
      );
    });
  });
}

/// Renders an empty state message.
fn render_empty(ui: &mut egui::Ui, message: &str) {
  ui.centered_and_justified(|ui| {
    ui.label(
      egui::RichText::new(message)
        .size(13.0)
        .color(egui::Color32::from_gray(120)),
    );
  });
}

/// Helper to detect if a cell contains a number.
fn is_number(s: &str) -> bool {
  s.parse::<f64>().is_ok()
}
