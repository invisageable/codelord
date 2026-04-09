//! SQLite database preview renderer.
//!
//! Renders SQLite database preview with table list and data view.
//! Uses egui's TableBuilder for data display.
//! Follows ECS pattern: UI collects actions, caller spawns request entities.

use crate::assets::icon::icon_to_image;
use crate::components::structure::divider::{self, Axis};

use codelord_core::ecs::world::World;
use codelord_core::icon::components::{Arrow, Download, Icon, Layout};
use codelord_core::popup::resources::{
  PopupAction, PopupCommand, PopupResource,
};
use codelord_core::previews::sqlite::{
  ChangePageRequest, ExecuteSqlRequest, QueryResult, SelectTableRequest,
  SqlitePreviewState, SqliteTab,
};

use eframe::egui;
use egui_extras::{Column, TableBuilder};

/// Renders the SQLite preview UI.
///
/// Follows ECS pattern: collects user actions during rendering,
/// then spawns appropriate request entities after the UI closure.
pub fn render(ui: &mut egui::Ui, world: &mut World) {
  // Get state for rendering (immutable borrow first for checks)
  let (enabled, is_loading, tables_empty) = {
    let Some(state) = world.get_resource::<SqlitePreviewState>() else {
      return;
    };

    (state.enabled, state.is_loading, state.tables.is_empty())
  };

  if !enabled {
    return;
  }

  if is_loading {
    render_loading(ui);
    return;
  }

  if tables_empty {
    render_empty(ui, "No tables found in database");
    return;
  }

  // Split layout: table list on left, content on right
  egui::Panel::left("sqlite_tables")
    .default_size(200.0)
    .frame(egui::Frame::NONE)
    .min_size(150.0)
    .max_size(400.0)
    .resizable(true)
    .show_inside(ui, |ui| show_table_list(ui, world));

  egui::CentralPanel::default()
    .frame(egui::Frame::NONE)
    .show_inside(ui, |ui| {
      egui::Panel::top("sqlite_tabbar")
        .exact_size(24.0)
        .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
        .show_inside(ui, |ui| show_tabbar(ui, world));

      egui::Panel::bottom("sqlite_footer")
        .exact_size(24.0)
        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 4)))
        .show_inside(ui, |ui| show_footer(ui, world));

      egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show_inside(ui, |ui| {
          let active_tab = world
            .get_resource::<SqlitePreviewState>()
            .map(|s| s.active_tab)
            .unwrap_or_default();

          match active_tab {
            SqliteTab::Data => show_data_view(ui, world),
            SqliteTab::Schema => show_schema_view(ui, world),
            SqliteTab::Sql => show_sql_view(ui, world),
          }
        });
    });
}

/// Renders the table list sidebar.
fn show_table_list(ui: &mut egui::Ui, world: &mut World) {
  // Collect table info to avoid borrow issues
  let (table_info, selected) = {
    let Some(state) = world.get_resource::<SqlitePreviewState>() else {
      return;
    };

    let info = state
      .tables
      .iter()
      .map(|t| (t.name.clone(), t.row_count))
      .collect::<Vec<_>>();

    (info, state.selected_table)
  };

  ui.horizontal(|ui| {
    ui.set_height(24.0);

    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
      ui.add_space(8.0);
      ui.add(
        icon_to_image(&Icon::Table).fit_to_exact_size(egui::vec2(14.0, 14.0)),
      );
      ui.label(
        egui::RichText::new("TABLES")
          .size(12.0)
          .color(egui::Color32::from_gray(200)),
      );
    });

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      ui.add_space(8.0);
      ui.label(
        egui::RichText::new(format!("{}", table_info.len()))
          .size(12.0)
          .color(egui::Color32::from_gray(120)),
      );
    });
  });

  divider::show(ui, Axis::Horizontal);

  egui::ScrollArea::vertical()
    .id_salt("sqlite_table_list")
    .auto_shrink([false; 2])
    .show(ui, |ui| {
      for (idx, (name, row_count)) in table_info.iter().enumerate() {
        let is_selected = selected == Some(idx);

        let bg_color = if is_selected {
          egui::Color32::from_rgba_unmultiplied(204, 253, 62, 30)
        } else {
          egui::Color32::TRANSPARENT
        };

        let text_color = if is_selected {
          egui::Color32::from_rgb(204, 253, 62)
        } else {
          egui::Color32::from_gray(200)
        };

        let frame = egui::Frame::NONE
          .fill(bg_color)
          .inner_margin(egui::Margin::symmetric(8, 4))
          .corner_radius(0.0);

        let frame_response = frame.show(ui, |ui| {
          ui.horizontal(|ui| {
            ui.add(
              icon_to_image(&Icon::Layout(Layout::Custom))
                .fit_to_exact_size(egui::vec2(14.0, 14.0))
                .tint(text_color),
            );
            ui.label(egui::RichText::new(name).color(text_color).size(12.0));
            ui.with_layout(
              egui::Layout::right_to_left(egui::Align::Center),
              |ui| {
                ui.label(
                  egui::RichText::new(format!("{row_count}"))
                    .color(egui::Color32::from_gray(120))
                    .size(10.0),
                );
              },
            );
          });
        });

        let response = ui.interact(
          frame_response.response.rect,
          egui::Id::new(("sqlite_table", idx)),
          egui::Sense::click(),
        );

        if response.clicked() && !is_selected {
          world.spawn(SelectTableRequest(idx));
        }

        if response.hovered() && !is_selected {
          ui.painter().rect_filled(
            frame_response.response.rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10),
          );
        }
      }
    });
}

/// Navigation button size (same as tabbar.rs).
const NAV_BUTTON_SIZE: egui::Vec2 = egui::vec2(24.0, 24.0);
/// Navigation icon size (same as tabbar.rs).
const NAV_ICON_SIZE: egui::Vec2 = egui::vec2(12.0, 12.0);

/// Renders the tab bar (Data, Schema, SQL).
fn show_tabbar(ui: &mut egui::Ui, world: &mut World) {
  // Get current state for rendering
  let (active_tab, has_data, current_page, total_pages) = {
    let Some(state) = world.get_resource::<SqlitePreviewState>() else {
      return;
    };

    (
      state.active_tab,
      !state.data.columns.is_empty(),
      state.current_page,
      state.total_pages(),
    )
  };

  let visuals = ui.style().visuals.clone();
  let bg_color = visuals.window_fill;
  let hover_bg = visuals.widgets.hovered.bg_fill;
  let enabled_tint = visuals.widgets.inactive.fg_stroke.color;
  let disabled_tint = egui::Color32::from_gray(60);

  let can_go_left = current_page > 0;
  let can_go_right = current_page + 1 < total_pages;

  ui.horizontal(|ui| {
    ui.spacing_mut().item_spacing.x = 0.0;

    // Left arrow button
    let left_sense = if can_go_left {
      egui::Sense::click()
    } else {
      egui::Sense::hover()
    };

    let left_response = ui.allocate_response(NAV_BUTTON_SIZE, left_sense);
    let left_rect = left_response.rect;

    ui.painter().rect_filled(
      left_rect,
      egui::CornerRadius::ZERO,
      if can_go_left && left_response.hovered() {
        hover_bg
      } else {
        bg_color
      },
    );

    ui.put(
      left_rect,
      icon_to_image(&Icon::Arrow(Arrow::Left))
        .fit_to_exact_size(NAV_ICON_SIZE)
        .tint(if can_go_left {
          enabled_tint
        } else {
          disabled_tint
        }),
    );

    divider::show(ui, Axis::Vertical);

    if !can_go_left && left_response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
    }
    if can_go_left && left_response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if can_go_left && left_response.clicked() {
      world.spawn(ChangePageRequest(current_page.saturating_sub(1)));
    }

    // Right arrow button
    let right_sense = if can_go_right {
      egui::Sense::click()
    } else {
      egui::Sense::hover()
    };

    let right_response = ui.allocate_response(NAV_BUTTON_SIZE, right_sense);
    let right_rect = right_response.rect;

    ui.painter().rect_filled(
      right_rect,
      egui::CornerRadius::ZERO,
      if can_go_right && right_response.hovered() {
        hover_bg
      } else {
        bg_color
      },
    );

    ui.put(
      right_rect,
      icon_to_image(&Icon::Arrow(Arrow::Right))
        .fit_to_exact_size(NAV_ICON_SIZE)
        .tint(if can_go_right {
          enabled_tint
        } else {
          disabled_tint
        }),
    );

    divider::show(ui, Axis::Vertical);

    if !can_go_right && right_response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
    }
    if can_go_right && right_response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if can_go_right && right_response.clicked() {
      world.spawn(ChangePageRequest(current_page + 1));
    }

    ui.add_space(8.0);
    ui.spacing_mut().item_spacing.x = 8.0;

    // Tabs
    let tabs = [
      (SqliteTab::Data, "Data", Icon::Layout(Layout::Custom)),
      (SqliteTab::Schema, "Schema", Icon::Schema),
      (SqliteTab::Sql, "SQL", Icon::Code),
    ];

    for (tab, label, icon) in tabs {
      let is_active = active_tab == tab;
      let text_color = if is_active {
        egui::Color32::from_rgb(204, 253, 62)
      } else {
        egui::Color32::from_gray(180)
      };

      let tab_response = ui.horizontal(|ui| {
        ui.add(
          icon_to_image(&icon)
            .fit_to_exact_size(egui::vec2(14.0, 14.0))
            .tint(text_color),
        );
        ui.add(
          egui::Button::new(
            egui::RichText::new(label).color(text_color).size(13.0),
          )
          .fill(egui::Color32::TRANSPARENT)
          .frame(false),
        )
      });

      let tab_rect = tab_response.response.rect;

      if tab_response.inner.clicked() {
        // Tab change is UI-only state, safe to mutate directly
        if let Some(mut state) = world.get_resource_mut::<SqlitePreviewState>()
        {
          state.active_tab = tab;
        }
      }

      if is_active {
        ui.painter().rect_filled(
          egui::Rect::from_min_size(
            egui::pos2(tab_rect.left(), tab_rect.bottom() - 2.0),
            egui::vec2(tab_rect.width(), 2.0),
          ),
          0.0,
          egui::Color32::from_rgb(204, 253, 62),
        );
      }
    }

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      ui.spacing_mut().item_spacing.x = 0.0;

      // Export button with popup (same style as navigation buttons)
      let export_sense = if has_data {
        egui::Sense::click()
      } else {
        egui::Sense::hover()
      };

      let export_response = ui.allocate_response(NAV_BUTTON_SIZE, export_sense);
      let export_rect = export_response.rect;

      ui.painter().rect_filled(
        export_rect,
        egui::CornerRadius::ZERO,
        if has_data && export_response.hovered() {
          hover_bg
        } else {
          bg_color
        },
      );

      ui.put(
        export_rect,
        icon_to_image(&Icon::Download(Download::Folder))
          .fit_to_exact_size(NAV_ICON_SIZE)
          .tint(if has_data {
            enabled_tint
          } else {
            disabled_tint
          }),
      );

      divider::show(ui, Axis::Vertical);

      if !has_data && export_response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
      }

      if has_data && export_response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
      }

      if has_data
        && export_response.clicked()
        && let Some(popup_entity) = world
          .get_resource::<PopupResource>()
          .and_then(|r| r.sqlite_export_popup)
      {
        let anchor_rect = [
          export_rect.min.x,
          export_rect.max.y,
          export_rect.width(),
          export_rect.height(),
        ];

        world.write_message(PopupCommand {
          action: PopupAction::Toggle {
            entity: popup_entity,
            anchor_rect,
          },
        });
      }
    });
  });
}

/// Renders the data view tab.
fn show_data_view(ui: &mut egui::Ui, world: &World) {
  let Some(state) = world.get_resource::<SqlitePreviewState>() else {
    return;
  };

  if state.selected_table.is_none() {
    render_empty(ui, "Select a table to view data");
    return;
  }

  if let Some(error) = &state.data.error {
    render_error(ui, error);
    return;
  }

  if state.data.columns.is_empty() {
    render_empty(ui, "No data in this table");
    return;
  }

  render_data_table(ui, &state.data);
}

/// Renders the data table.
fn render_data_table(ui: &mut egui::Ui, data: &QueryResult) {
  let column_count = data.columns.len();

  let mut column_widths = vec![100.0_f32; column_count];
  let char_width = 7.0_f32;
  let padding = 30.0_f32;

  for (col_idx, header) in data.columns.iter().enumerate() {
    let header_width = (header.len() as f32 * char_width) + padding;
    column_widths[col_idx] = column_widths[col_idx].max(header_width);
  }

  for row in data.rows.iter() {
    for (col_idx, cell) in row.iter().enumerate() {
      if col_idx < column_count {
        let cell_width = (cell.len() as f32 * char_width) + padding;
        column_widths[col_idx] = column_widths[col_idx].max(cell_width);
      }
    }
  }

  for width in column_widths.iter_mut() {
    *width = width.min(400.0_f32);
  }

  // Row number column width
  let row_num_width = 50.0_f32;

  egui::ScrollArea::both()
    .id_salt("sqlite_data_table")
    .auto_shrink([false; 2])
    .show(ui, |ui| {
      let mut table = TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .min_scrolled_height(0.0)
        .sense(egui::Sense::hover());

      // Add row number column first
      table = table.column(Column::exact(row_num_width));

      // Add data columns
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
                  .size(14.0)
                  .color(egui::Color32::from_gray(120)),
              );
            });
          });

          // Data column headers
          for col_name in data.columns.iter() {
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
          body.rows(24.0, data.rows.len(), |mut row| {
            let row_index = row.index();
            let row_data = &data.rows[row_index];

            // Row number cell
            row.col(|ui| {
              ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                ui.label(
                  egui::RichText::new(format!("{}", row_index + 1))
                    .size(12.0)
                    .color(egui::Color32::from_gray(100)),
                );
              });
            });

            // Data cells
            for cell in row_data.iter() {
              row.col(|ui| {
                ui.horizontal_centered(|ui| {
                  ui.add_space(12.0);

                  let text_color = if is_number(cell) {
                    egui::Color32::from_rgb(181, 206, 168)
                  } else if cell.is_empty() || cell == "NULL" {
                    egui::Color32::from_gray(100)
                  } else {
                    egui::Color32::from_gray(220)
                  };

                  let display_text =
                    if cell.is_empty() { "NULL" } else { cell };

                  ui.label(
                    egui::RichText::new(display_text)
                      .color(text_color)
                      .size(13.0),
                  );
                });
              });
            }
          });
        });
    });
}

/// Renders the schema view tab.
fn show_schema_view(ui: &mut egui::Ui, world: &World) {
  let Some(state) = world.get_resource::<SqlitePreviewState>() else {
    return;
  };

  let Some(table_idx) = state.selected_table else {
    render_empty(ui, "Select a table to view schema");
    return;
  };

  let Some(table) = state.tables.get(table_idx) else {
    return;
  };

  // Colors
  let accent = egui::Color32::from_rgb(204, 253, 62);
  let type_color = egui::Color32::from_rgb(86, 182, 194); // Cyan for types
  let not_null_color = egui::Color32::from_rgb(224, 108, 117); // Red for NOT NULL
  let default_color = egui::Color32::from_gray(140);
  let pk_color = egui::Color32::from_rgb(229, 192, 123); // Gold for PK
  let border_color = egui::Color32::from_gray(60);

  egui::ScrollArea::vertical()
    .id_salt("sqlite_schema_view")
    .auto_shrink([false; 2])
    .show(ui, |ui| {
      ui.add_space(8.0);

      // ====================================================================
      // Block 1: Table Overview
      // ====================================================================
      egui::Frame::NONE
        .stroke(egui::Stroke::new(1.0, border_color))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
          // Table name
          ui.label(
            egui::RichText::new(&table.name)
              .color(egui::Color32::WHITE)
              .size(15.0)
              .strong(),
          );

          // Column count
          ui.label(
            egui::RichText::new(format!("{} columns", table.columns.len()))
              .color(egui::Color32::from_gray(120))
              .size(12.0),
          );

          ui.add_space(8.0);
          ui.separator();
          ui.add_space(4.0);

          // Column rows
          for col in &table.columns {
            ui.horizontal(|ui| {
              // Primary key icon
              if col.is_pk {
                ui.label(egui::RichText::new("🔑").size(12.0));
              }

              // Column name
              let name_color = if col.is_pk {
                pk_color
              } else {
                egui::Color32::WHITE
              };
              ui.label(
                egui::RichText::new(&col.name).color(name_color).size(13.0),
              );

              // Type badge
              render_badge(ui, &col.dtype, type_color);

              // NOT NULL constraint
              if col.is_not_null {
                ui.label(
                  egui::RichText::new("NOT NULL")
                    .color(not_null_color)
                    .size(11.0),
                );
              }

              // DEFAULT value
              if let Some(ref default_val) = col.default_value {
                ui.label(
                  egui::RichText::new("DEFAULT")
                    .color(default_color)
                    .size(11.0),
                );
                ui.label(
                  egui::RichText::new(default_val).color(accent).size(11.0),
                );
              }
            });
            ui.add_space(2.0);
          }
        });

      ui.add_space(16.0);

      // ====================================================================
      // Block 2: CREATE TABLE Statement
      // ====================================================================
      ui.label(
        egui::RichText::new("CREATE TABLE Statement")
          .color(egui::Color32::from_gray(150))
          .size(12.0),
      );
      ui.add_space(8.0);

      egui::Frame::NONE
        .fill(egui::Color32::from_gray(25))
        .stroke(egui::Stroke::new(1.0, border_color))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
          // Generate CREATE TABLE statement
          let create_sql = generate_create_table_sql(table);

          // Render syntax-highlighted SQL
          render_sql_syntax(ui, &create_sql);
        });
    });
}

/// Renders a small badge with text.
fn render_badge(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
  egui::Frame::NONE
    .fill(color.gamma_multiply(0.2))
    .corner_radius(3.0)
    .inner_margin(egui::Margin::symmetric(6, 2))
    .show(ui, |ui| {
      ui.label(egui::RichText::new(text).color(color).size(11.0));
    });
}

/// Generates CREATE TABLE SQL from table info.
fn generate_create_table_sql(
  table: &codelord_core::previews::sqlite::TableInfo,
) -> String {
  let mut sql = format!("CREATE TABLE {} (\n", table.name);

  for (i, col) in table.columns.iter().enumerate() {
    sql.push_str("  ");
    sql.push_str(&col.name);
    sql.push(' ');
    sql.push_str(&col.dtype);

    if col.is_pk {
      sql.push_str(" PRIMARY KEY");
    }
    if col.is_not_null {
      sql.push_str(" NOT NULL");
    }
    if let Some(ref default_val) = col.default_value {
      sql.push_str(" DEFAULT ");
      sql.push_str(default_val);
    }

    if i < table.columns.len() - 1 {
      sql.push(',');
    }
    sql.push('\n');
  }

  sql.push_str(");");
  sql
}

/// Renders syntax-highlighted SQL.
fn render_sql_syntax(ui: &mut egui::Ui, sql: &str) {
  let keyword_color = egui::Color32::from_rgb(198, 120, 221); // Purple
  let type_color = egui::Color32::from_rgb(86, 182, 194); // Cyan
  let name_color = egui::Color32::from_rgb(152, 195, 121); // Green
  let default_color = egui::Color32::WHITE;

  let keywords = [
    "CREATE", "TABLE", "PRIMARY", "KEY", "NOT", "NULL", "DEFAULT",
  ];
  let types = [
    "INTEGER", "TEXT", "REAL", "BLOB", "DATETIME", "BOOLEAN", "VARCHAR",
  ];

  for line in sql.lines() {
    ui.horizontal(|ui| {
      ui.spacing_mut().item_spacing.x = 0.0;

      let tokens: Vec<&str> = line.split_whitespace().collect();
      let mut first = true;

      // Handle indentation
      if line.starts_with("  ") {
        ui.label(egui::RichText::new("  ").size(12.0).monospace());
      }

      for token in tokens {
        if !first {
          ui.label(egui::RichText::new(" ").size(12.0).monospace());
        }
        first = false;

        // Remove trailing comma/semicolon for matching
        let clean_token = token.trim_end_matches([',', ';']);
        let suffix = &token[clean_token.len()..];

        let color = if keywords.contains(&clean_token.to_uppercase().as_str()) {
          keyword_color
        } else if types.contains(&clean_token.to_uppercase().as_str()) {
          type_color
        } else if clean_token.starts_with('(') || clean_token.ends_with(')') {
          default_color
        } else {
          name_color
        };

        ui.label(
          egui::RichText::new(clean_token)
            .color(color)
            .size(12.0)
            .monospace(),
        );

        if !suffix.is_empty() {
          ui.label(
            egui::RichText::new(suffix)
              .color(default_color)
              .size(12.0)
              .monospace(),
          );
        }
      }
    });
  }
}

/// Renders the footer with page info, row/column count, and actions.
fn show_footer(ui: &mut egui::Ui, world: &World) {
  let (row_count, col_count, current_page, total_pages) = world
    .get_resource::<SqlitePreviewState>()
    .map(|state| {
      (
        state.data.rows.len(),
        state.data.columns.len(),
        state.current_page,
        state.total_pages(),
      )
    })
    .unwrap_or((0, 0, 0, 0));

  ui.horizontal_centered(|ui| {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
      // Rows count
      ui.label(
        egui::RichText::new(format!("{row_count} rows"))
          .color(egui::Color32::from_gray(150))
          .size(10.0),
      );
      ui.add_space(8.0);

      // Columns count
      ui.label(
        egui::RichText::new(format!("{col_count} columns"))
          .color(egui::Color32::from_gray(150))
          .size(10.0),
      );
    });
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      // Page info
      let page_info = format!("Page {} of {total_pages}", current_page + 1);
      ui.label(
        egui::RichText::new(page_info)
          .color(egui::Color32::from_gray(150))
          .size(10.0),
      );
    });
  });
}

/// Renders the SQL view tab.
fn show_sql_view(ui: &mut egui::Ui, world: &mut World) {
  ui.add_space(8.0);

  ui.label(
    egui::RichText::new("Enter SQL query:")
      .color(egui::Color32::from_gray(150))
      .size(12.0),
  );
  ui.add_space(4.0);

  // Get mutable access for text input
  let custom_sql = {
    let Some(mut state) = world.get_resource_mut::<SqlitePreviewState>() else {
      return;
    };

    egui::ScrollArea::vertical()
      .id_salt("sqlite_sql_input")
      .max_height(100.0)
      .show(ui, |ui| {
        ui.add(
          egui::TextEdit::multiline(&mut state.custom_sql)
            .code_editor()
            .desired_width(f32::INFINITY)
            .desired_rows(4)
            .font(egui::TextStyle::Monospace),
        );
      });

    state.custom_sql.clone()
  };

  ui.add_space(8.0);

  ui.horizontal(|ui| {
    if ui
      .add_enabled(
        !custom_sql.trim().is_empty(),
        egui::Button::new(
          egui::RichText::new("Execute")
            .color(egui::Color32::from_rgb(204, 253, 62))
            .size(13.0),
        ),
      )
      .clicked()
    {
      world.spawn(ExecuteSqlRequest);
    }

    if ui.button("Clear").clicked()
      && let Some(mut state) = world.get_resource_mut::<SqlitePreviewState>()
    {
      state.custom_sql.clear();
      state.custom_sql_result = QueryResult::default();
    }
  });

  ui.separator();

  // Render results
  if let Some(state) = world.get_resource::<SqlitePreviewState>() {
    if let Some(error) = &state.custom_sql_result.error {
      render_error(ui, error);
    } else if !state.custom_sql_result.columns.is_empty() {
      render_data_table(ui, &state.custom_sql_result);
    }
  }
}

/// Renders a loading indicator.
fn render_loading(ui: &mut egui::Ui) {
  ui.centered_and_justified(|ui| {
    ui.vertical_centered(|ui| {
      ui.spinner();
      ui.add_space(16.0);
      ui.label(
        egui::RichText::new("Loading database...")
          .size(14.0)
          .color(egui::Color32::from_gray(180)),
      );
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
          .size(20.0),
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

/// Renders an empty state message.
fn render_empty(ui: &mut egui::Ui, message: &str) {
  ui.centered_and_justified(|ui| {
    ui.vertical_centered(|ui| {
      ui.label(
        egui::RichText::new(message)
          .size(14.0)
          .color(egui::Color32::from_gray(150)),
      );
    });
  });
}

/// Helper function to detect if a cell contains a number.
fn is_number(s: &str) -> bool {
  s.parse::<f64>().is_ok()
}
