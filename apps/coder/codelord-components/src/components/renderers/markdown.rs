//! Egui-specific markdown renderer implementation.
//!
//! This module implements the swisskit-renderer MarkdownRenderer trait
//! for rendering markdown to egui widgets.

use crate::assets::font;

use eframe::egui;
use pulldown_cmark::{CodeBlockKind, HeadingLevel, Tag, TagEnd};
use swisskit::renderer::markdown::parser::create_parser;
use swisskit::renderer::markdown::{MarkdownRenderer, RenderState};

/// Represents an [`egui`] markdown renderer.
struct MdRenderer;

impl MdRenderer {
  /// Renders an H1 heading with a subtle grid background.
  fn render_h1_with_grid(ui: &mut egui::Ui, text: &str, size: f32) {
    const GRID_SIZE: f32 = 10.0;
    const GRID_COLOR: egui::Color32 = egui::Color32::from_gray(30);
    const PADDING: f32 = 32.0;

    let available_width = ui.available_width();

    // Calculate height needed for the text with padding
    let text_height = size + PADDING * 2.0;

    // Allocate space for the grid background
    let (rect, _) = ui.allocate_exact_size(
      egui::vec2(available_width, text_height),
      egui::Sense::hover(),
    );

    // Draw grid background
    let painter = ui.painter();

    // Draw vertical lines
    let mut x = rect.left();
    while x <= rect.right() {
      painter.line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        egui::Stroke::new(1.0, GRID_COLOR),
      );
      x += GRID_SIZE;
    }

    // Draw horizontal lines
    let mut y = rect.top();
    while y <= rect.bottom() {
      painter.line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        egui::Stroke::new(1.0, GRID_COLOR),
      );
      y += GRID_SIZE;
    }

    // Draw the text on top of the grid
    let text_pos = egui::pos2(rect.left() + 16.0, rect.center().y - size / 2.0);

    painter.text(
      text_pos,
      egui::Align2::LEFT_TOP,
      text,
      font::cirka(size),
      egui::Color32::from_rgb(204, 253, 62),
    );
  }
}
impl MarkdownRenderer for MdRenderer {
  type Context = egui::Ui;

  /// Handles start tags.
  fn handle_start_tag(_ui: &mut egui::Ui, state: &mut RenderState, tag: Tag) {
    match tag {
      Tag::Heading { level, .. } => {
        state.current_heading_level = Some(level);
        state.text_buffer.clear();
      }
      Tag::Paragraph => {
        state.in_paragraph = true;
        state.text_buffer.clear();
      }
      Tag::BlockQuote(_) => {
        state.in_blockquote = true;
      }
      Tag::CodeBlock(kind) => {
        state.in_code_block = true;
        state.text_buffer.clear();

        if let CodeBlockKind::Fenced(lang) = kind {
          state.code_block_lang = Some(lang.into());
        }
      }
      Tag::List(start_number) => {
        state.list_depth += 1;

        if let Some(num) = start_number {
          state.is_ordered_list = true;
          state.list_counter = num as usize;
        } else {
          state.is_ordered_list = false;
        }
      }
      Tag::Item => {
        // Items are handled when we render their content
      }
      Tag::Image {
        dest_url, title, ..
      } => {
        state.in_image = true;
        state.image_url = Some(dest_url.to_string());
        state.image_title = title.to_string();
        state.image_alt_text.clear();
      }
      Tag::Strong | Tag::Emphasis => {
        // These will be handled in text rendering
      }
      _ => {}
    }
  }

  /// Handles end tags.
  fn handle_end_tag(ui: &mut egui::Ui, state: &mut RenderState, tag: TagEnd) {
    match tag {
      TagEnd::Heading(level) => {
        let text = std::mem::take(&mut state.text_buffer);
        let size = match level {
          HeadingLevel::H1 => 32.0,
          HeadingLevel::H2 => 28.0,
          HeadingLevel::H3 => 24.0,
          HeadingLevel::H4 => 20.0,
          HeadingLevel::H5 => 18.0,
          HeadingLevel::H6 => 16.0,
        };

        ui.add_space(8.0);

        // Capture heading position for scroll progress tracking
        let y_pos = ui.cursor().top();
        state.headings.push((text.clone(), y_pos));

        // For H1 headings, render a grid background
        if matches!(level, HeadingLevel::H1) {
          Self::render_h1_with_grid(ui, &text, size);
        } else {
          ui.label(
            egui::RichText::new(text)
              .size(size)
              .color(egui::Color32::from_rgb(204, 253, 62))
              .family(egui::FontFamily::Name(font::CIRKA.into())),
          );
        }

        ui.add_space(4.0);

        state.current_heading_level = None;
      }
      TagEnd::Paragraph => {
        let text = std::mem::take(&mut state.text_buffer);

        if !text.trim().is_empty() {
          if state.in_blockquote {
            ui.horizontal(|ui| {
              ui.painter().vline(
                ui.cursor().left(),
                ui.available_rect_before_wrap().y_range(),
                egui::Stroke::new(3.0, egui::Color32::from_rgb(204, 253, 62)),
              );
              ui.add_space(12.0);
              ui.label(
                egui::RichText::new(text).color(egui::Color32::from_gray(180)),
              );
            });
          } else {
            ui.label(egui::RichText::new(text).color(egui::Color32::WHITE));
          }
          ui.add_space(8.0);
        }

        state.in_paragraph = false;
      }
      TagEnd::BlockQuote(_) => {
        state.in_blockquote = false;
      }
      TagEnd::CodeBlock => {
        let code = std::mem::take(&mut state.text_buffer);
        let code = code.trim_end();

        egui::Frame::NONE
          .fill(egui::Color32::from_gray(20))
          .inner_margin(8.0)
          .show(ui, |ui| {
            ui.label(
              egui::RichText::new(code)
                .family(egui::FontFamily::Monospace)
                .color(egui::Color32::from_gray(220)),
            );
          });

        state.in_code_block = false;
        state.code_block_lang = None;
      }
      TagEnd::List(_) => {
        state.list_depth = state.list_depth.saturating_sub(1);
        ui.add_space(4.0);
      }
      TagEnd::Item => {
        let text = std::mem::take(&mut state.text_buffer);
        let indent = "  ".repeat(state.list_depth.saturating_sub(1));

        let bullet = if state.is_ordered_list {
          format!("{indent}{}. ", state.list_counter)
        } else {
          format!("{indent}• ")
        };

        if state.is_ordered_list {
          state.list_counter += 1;
        }

        ui.label(
          egui::RichText::new(format!("{bullet}{text}"))
            .color(egui::Color32::WHITE),
        );
      }
      TagEnd::Image => {
        let url = state.image_url.take().unwrap_or_default();
        let title = std::mem::take(&mut state.image_title);
        let alt_text = std::mem::take(&mut state.image_alt_text);

        Self::handle_image(ui, state, &url, &title, &alt_text);
        state.in_image = false;
      }
      _ => {}
    }
  }

  /// Handles text — appends a text onto the end of a [`String`].
  fn handle_text(_ui: &mut egui::Ui, state: &mut RenderState, text: &str) {
    if state.in_image {
      state.image_alt_text.push_str(text);
    } else {
      state.text_buffer.push_str(text);
    }
  }

  /// handles inline code — appends a code onto the end of a [`String`].
  fn handle_inline_code(
    _ui: &mut egui::Ui,
    state: &mut RenderState,
    code: &str,
  ) {
    state.text_buffer.push('`');
    state.text_buffer.push_str(code);
    state.text_buffer.push('`');
  }

  fn handle_soft_break(_ui: &mut egui::Ui, state: &mut RenderState) {
    if state.in_code_block {
      state.text_buffer.push('\n');
    } else {
      state.text_buffer.push(' ');
    }
  }

  fn handle_hard_break(_ui: &mut egui::Ui, state: &mut RenderState) {
    state.text_buffer.push('\n');
  }

  fn handle_rule(ui: &mut egui::Ui, _state: &mut RenderState) {
    let rect = ui.available_rect_before_wrap();

    ui.horizontal(|ui| {
      ui.add_space(4.0);
      ui.painter().line_segment(
        [
          egui::pos2(rect.left(), ui.cursor().top()),
          egui::pos2(rect.right(), ui.cursor().top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(204, 253, 62)),
      );
      ui.add_space(4.0);
    });
  }

  fn handle_image(
    ui: &mut egui::Ui,
    state: &mut RenderState,
    url: &str,
    title: &str,
    alt_text: &str,
  ) {
    ui.add_space(4.0);

    // Resolve the image path
    let image_uri = if url.starts_with("http://") || url.starts_with("https://")
    {
      url.into()
    } else {
      // Local file path - resolve relative to base_path if available
      if let Some(base_path) = &state.base_path {
        if let Some(parent) = base_path.parent() {
          let resolved = parent.join(url);

          if let Ok(canonical) = resolved.canonicalize() {
            format!("file://{}", canonical.display())
          } else {
            format!("file://{}", resolved.display())
          }
        } else {
          format!("file://{url}")
        }
      } else {
        format!("file://{url}")
      }
    };

    ui.vertical(|ui| {
      let image = egui::Image::new(&image_uri);

      ui.add(image.max_width(ui.available_width() * 0.9));

      if !title.is_empty() {
        ui.label(
          egui::RichText::new(title)
            .size(12.0)
            .italics()
            .color(egui::Color32::from_gray(180)),
        );
      } else if !alt_text.is_empty() {
        ui.label(
          egui::RichText::new(alt_text)
            .size(12.0)
            .italics()
            .color(egui::Color32::from_gray(180)),
        );
      }
    });

    ui.add_space(4.0);
  }
}

/// Mini markdown renderer for slide thumbnails.
struct MiniMdRenderer;

impl MarkdownRenderer for MiniMdRenderer {
  type Context = egui::Ui;

  fn handle_start_tag(_ui: &mut egui::Ui, state: &mut RenderState, tag: Tag) {
    match tag {
      Tag::Heading { level, .. } => {
        state.current_heading_level = Some(level);
        state.text_buffer.clear();
      }
      Tag::Paragraph => {
        state.in_paragraph = true;
        state.text_buffer.clear();
      }
      Tag::CodeBlock(_) => {
        state.in_code_block = true;
        state.text_buffer.clear();
      }
      Tag::List(start_number) => {
        state.list_depth += 1;
        if let Some(num) = start_number {
          state.is_ordered_list = true;
          state.list_counter = num as usize;
        } else {
          state.is_ordered_list = false;
        }
      }
      Tag::Item => {}
      Tag::Image { .. } => {
        state.in_image = true;
      }
      _ => {}
    }
  }

  fn handle_end_tag(ui: &mut egui::Ui, state: &mut RenderState, tag: TagEnd) {
    match tag {
      TagEnd::Heading(level) => {
        let text = std::mem::take(&mut state.text_buffer);
        let size = match level {
          HeadingLevel::H1 => 10.0,
          HeadingLevel::H2 => 9.0,
          HeadingLevel::H3 => 8.0,
          _ => 7.0,
        };

        ui.horizontal_wrapped(|ui| {
          ui.label(
            egui::RichText::new(text)
              .size(size)
              .color(egui::Color32::from_rgb(204, 253, 62)),
          );
        });
        ui.end_row();

        state.current_heading_level = None;
      }
      TagEnd::Paragraph => {
        let text = std::mem::take(&mut state.text_buffer);
        if !text.trim().is_empty() {
          ui.horizontal_wrapped(|ui| {
            ui.label(
              egui::RichText::new(text)
                .size(7.0)
                .color(egui::Color32::from_gray(200)),
            );
          });
          ui.end_row();
        }
        state.in_paragraph = false;
      }
      TagEnd::CodeBlock => {
        let code = std::mem::take(&mut state.text_buffer);
        ui.horizontal_wrapped(|ui| {
          ui.label(
            egui::RichText::new(code.trim_end())
              .size(6.0)
              .family(egui::FontFamily::Monospace)
              .color(egui::Color32::from_gray(180)),
          );
        });
        ui.end_row();
        state.in_code_block = false;
      }
      TagEnd::List(_) => {
        state.list_depth = state.list_depth.saturating_sub(1);
      }
      TagEnd::Item => {
        let text = std::mem::take(&mut state.text_buffer);
        let bullet = if state.is_ordered_list {
          format!("{}. ", state.list_counter)
        } else {
          "• ".into()
        };
        if state.is_ordered_list {
          state.list_counter += 1;
        }
        ui.horizontal_wrapped(|ui| {
          ui.label(
            egui::RichText::new(format!("{bullet}{text}"))
              .size(7.0)
              .color(egui::Color32::from_gray(200)),
          );
        });
        ui.end_row();
      }
      TagEnd::Image => {
        state.in_image = false;
      }
      _ => {}
    }
  }

  fn handle_text(_ui: &mut egui::Ui, state: &mut RenderState, text: &str) {
    if !state.in_image {
      state.text_buffer.push_str(text);
    }
  }

  fn handle_inline_code(
    _ui: &mut egui::Ui,
    state: &mut RenderState,
    code: &str,
  ) {
    state.text_buffer.push('`');
    state.text_buffer.push_str(code);
    state.text_buffer.push('`');
  }

  fn handle_soft_break(_ui: &mut egui::Ui, state: &mut RenderState) {
    state.text_buffer.push(' ');
  }

  fn handle_hard_break(_ui: &mut egui::Ui, state: &mut RenderState) {
    state.text_buffer.push('\n');
  }

  fn handle_rule(_ui: &mut egui::Ui, _state: &mut RenderState) {
    // Skip horizontal rules in mini view
  }

  fn handle_image(
    _ui: &mut egui::Ui,
    _state: &mut RenderState,
    _url: &str,
    _title: &str,
    _alt_text: &str,
  ) {
    // Skip images in mini view
  }
}

/// Render mini markdown preview for slide thumbnails.
pub fn render_mini(ui: &mut egui::Ui, input: &str) {
  let parser = create_parser(input);
  let mut state = RenderState::default();

  ui.spacing_mut().item_spacing.y = 1.0;

  for event in parser {
    MiniMdRenderer::process_event(ui, &mut state, event);
  }
}

/// Render markdown content to egui.
pub fn render(
  ui: &mut egui::Ui,
  input: &str,
) -> (Vec<(String, f32)>, f32, f32) {
  render_with_base_path(ui, input, None)
}

/// Render markdown content to egui with a base path for resolving relative
/// URLs.
///
/// Returns: (headings, scroll_offset, content_height).
pub fn render_with_base_path(
  ui: &mut egui::Ui,
  input: &str,
  base_path: Option<&std::path::Path>,
) -> (Vec<(String, f32)>, f32, f32) {
  let parser = create_parser(input);

  let mut state = RenderState {
    base_path: base_path.map(|p| p.to_path_buf()),
    ..Default::default()
  };

  let scroll_output = egui::ScrollArea::vertical()
    .id_salt("markdown_content_scroll")
    .auto_shrink([false; 2])
    .show(ui, |ui| {
      egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(64, 0))
        .show(ui, |ui| {
          ui.spacing_mut().item_spacing.y = 4.0;

          for event in parser {
            MdRenderer::process_event(ui, &mut state, event);
          }
        });
    });

  let scroll_offset = scroll_output.state.offset.y;
  let content_height = scroll_output.inner_rect.height();

  (state.headings, scroll_offset, content_height)
}
