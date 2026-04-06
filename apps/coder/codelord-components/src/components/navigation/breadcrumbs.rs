use crate::assets::icon::icon_to_image;
use crate::assets::theme::syntax_color;
use crate::components::atoms::icon_button;

use codelord_core::ecs::prelude::With;
use codelord_core::ecs::world::World;
use codelord_core::events::{
  SvgZoomInRequest, SvgZoomOutRequest, SvgZoomResetRequest,
  ToggleCsvPreviewRequest, ToggleHtmlPreviewRequest,
  ToggleMarkdownPreviewRequest,
};
use codelord_core::icon::components::{Arrow, Icon, Preview};
use codelord_core::navigation::resources::{BreadcrumbData, SegmentKind};
use codelord_core::panel::components::RightPanelView;
use codelord_core::panel::resources::RightPanelResource;
use codelord_core::previews::{
  CsvPreviewState, HtmlPreviewState, MarkdownPreviewState, SvgPreviewState,
};
use codelord_core::tabbar::components::EditorTab;
use codelord_core::text_editor::components::FileTab;
use codelord_core::token::TokenKind;
use codelord_core::ui::component::Active;

use eframe::egui;

/// Show breadcrumbs for the active editor tab.
///
/// Reads pre-computed segments from `BreadcrumbData` resource.
/// The `update_breadcrumbs_system` is responsible for computing segments.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  // Check if active tab is HTML, Markdown, or CSV for preview buttons
  // (SQLite and PDF files open viewer directly, no toggle needed)
  let (is_html, is_markdown, is_csv) = {
    let mut query =
      world.query_filtered::<&FileTab, (With<EditorTab>, With<Active>)>();

    query
      .iter(world)
      .next()
      .map(|file_tab| {
        (
          file_tab.is_html(),
          file_tab.is_markdown(),
          file_tab.is_csv(),
        )
      })
      .unwrap_or((false, false, false))
  };

  // Check if HTML preview is active
  let is_html_preview_active = world
    .get_resource::<HtmlPreviewState>()
    .map(|p| p.enabled)
    .unwrap_or(false)
    && world
      .get_resource::<RightPanelResource>()
      .map(|r| r.active_view == RightPanelView::WebView && r.is_visible)
      .unwrap_or(false);

  // Check if markdown preview is active
  let is_markdown_preview_active = world
    .get_resource::<MarkdownPreviewState>()
    .map(|p| p.enabled)
    .unwrap_or(false);

  // Check if CSV preview is active
  let is_csv_preview_active = world
    .get_resource::<CsvPreviewState>()
    .map(|p| p.enabled)
    .unwrap_or(false);

  // Get SVG preview state (enabled and zoom level)
  let (is_svg_preview_active, svg_zoom) = world
    .get_resource::<SvgPreviewState>()
    .map(|p| (p.enabled, p.zoom))
    .unwrap_or((false, 1.0));

  // Clone segments to avoid holding borrow during UI rendering
  let segments = world
    .get_resource::<BreadcrumbData>()
    .map(|b| b.segments.clone())
    .unwrap_or_default();

  if segments.is_empty() {
    return;
  }

  // Get theme colors before borrowing ui mutably
  let filename_color = ui.visuals().strong_text_color();
  let folder_color = ui.visuals().weak_text_color();
  let separator_color = ui.visuals().weak_text_color();
  let text_color = ui.visuals().text_color();

  // Track which preview button was clicked
  let mut toggle_html_preview = false;
  let mut toggle_markdown_preview = false;
  let mut toggle_csv_preview = false;
  let mut svg_zoom_in = false;
  let mut svg_zoom_out = false;
  let mut svg_zoom_reset = false;

  ui.horizontal_centered(|ui| {
    ui.spacing_mut().item_spacing.x = 4.0;

    for (i, segment) in segments.iter().enumerate() {
      if i > 0 {
        ui.add(
          icon_to_image(&Icon::Arrow(Arrow::AngleRightLine))
            .fit_to_exact_size(egui::vec2(10.0, 10.0))
            .tint(separator_color),
        );
      }

      // Determine color and rendering based on segment kind
      match &segment.kind {
        SegmentKind::Path { is_filename } => {
          let color = if *is_filename {
            filename_color
          } else {
            folder_color
          };
          ui.label(egui::RichText::new(&segment.text).color(color).size(10.0));
        }
        SegmentKind::Symbol { .. } => {
          // For symbols, render with syntax highlighting
          if segment.highlights.is_empty() {
            // No highlights, just render as text
            ui.label(
              egui::RichText::new(&segment.text)
                .color(filename_color)
                .size(10.0),
            );
          } else {
            // Render with syntax highlights using a LayoutJob
            let mut job = egui::text::LayoutJob::default();
            let mut last_end = 0;

            for (range, token_kind) in &segment.highlights {
              // Add any text before this highlight
              if range.start > last_end {
                let text_before = &segment.text[last_end..range.start];

                job.append(
                  text_before,
                  0.0,
                  egui::TextFormat {
                    font_id: egui::FontId::proportional(10.0),
                    color: folder_color,
                    ..Default::default()
                  },
                );
              }

              // Add highlighted text
              let highlight_text = &segment.text[range.clone()];
              let highlight_color = syntax_color(unsafe {
                std::mem::transmute::<u8, TokenKind>(*token_kind)
              });

              job.append(
                highlight_text,
                0.0,
                egui::TextFormat {
                  font_id: egui::FontId::proportional(10.0),
                  color: highlight_color,
                  ..Default::default()
                },
              );

              last_end = range.end;
            }

            // Add any remaining text
            if last_end < segment.text.len() {
              let remaining = &segment.text[last_end..];

              job.append(
                remaining,
                0.0,
                egui::TextFormat {
                  font_id: egui::FontId::proportional(10.0),
                  color: folder_color,
                  ..Default::default()
                },
              );
            }

            ui.label(job);
          }
        }
      }
    }

    // Spacer to push preview buttons to the right
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      // HTML preview toggle button (for .html files)
      if is_html {
        let tint = if is_html_preview_active {
          egui::Color32::from_rgb(204, 253, 62) // Green when active
        } else {
          text_color
        };

        if icon_button::show(ui, &Icon::Preview(Preview::Markdown), tint) {
          toggle_html_preview = true;
        }
      }

      // Markdown preview toggle button (for .md files)
      if is_markdown {
        let tint = if is_markdown_preview_active {
          egui::Color32::from_rgb(204, 253, 62) // Green when active
        } else {
          text_color
        };

        if icon_button::show(ui, &Icon::Preview(Preview::Markdown), tint) {
          toggle_markdown_preview = true;
        }
      }

      // CSV preview toggle button (for .csv files)
      if is_csv {
        let tint = if is_csv_preview_active {
          egui::Color32::from_rgb(204, 253, 62) // Green when active
        } else {
          text_color
        };

        if icon_button::show(ui, &Icon::Preview(Preview::Markdown), tint) {
          toggle_csv_preview = true;
        }
      }

      // SVG zoom controls (when SVG preview is active)
      if is_svg_preview_active {
        ui.add_space(8.0);

        // Zoom in (rightmost in right-to-left layout)
        if ui
          .add(egui::Button::new("+").min_size(egui::vec2(20.0, 16.0)))
          .clicked()
        {
          svg_zoom_in = true;
        }

        // Zoom percentage
        ui.label(
          egui::RichText::new(format!("{:.0}%", svg_zoom * 100.0))
            .size(10.0)
            .color(text_color),
        );

        // Zoom out
        if ui
          .add(egui::Button::new("-").min_size(egui::vec2(20.0, 16.0)))
          .clicked()
        {
          svg_zoom_out = true;
        }

        // Reset (leftmost)
        if ui
          .add(egui::Button::new("Reset").min_size(egui::vec2(36.0, 16.0)))
          .clicked()
        {
          svg_zoom_reset = true;
        }
      }
    });
  });

  if toggle_html_preview {
    world.spawn(ToggleHtmlPreviewRequest);
  }

  if toggle_markdown_preview {
    world.spawn(ToggleMarkdownPreviewRequest);
  }

  if toggle_csv_preview {
    world.spawn(ToggleCsvPreviewRequest);
  }

  if svg_zoom_in {
    world.spawn(SvgZoomInRequest);
  }

  if svg_zoom_out {
    world.spawn(SvgZoomOutRequest);
  }

  if svg_zoom_reset {
    world.spawn(SvgZoomResetRequest);
  }
}
