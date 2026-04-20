use crate::assets::icon::icon_to_image;
use crate::assets::theme::{get_theme, syntax_color};
use crate::components::structure::divider;
use crate::components::structure::divider::Axis;

use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::ActiveAnimations;
use codelord_core::ecs::world::World;
use codelord_core::file_picker::components::{CachedPreview, SelectAction};
use codelord_core::file_picker::resources::{
  FilePickerMatcher, FilePickerResponse, FilePickerState, RowPaddingAnim,
  load_preview,
};
use codelord_core::icon::components::{Arrow, Icon};
use codelord_core::language::Language;
use codelord_core::navigation::resources::ExplorerState;
use codelord_core::theme::Theme;
use codelord_core::token::{Token, TokenExtractors};

use eazy::{Curve, Easing};

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui_extras::{Column, TableBuilder};

use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// Animation duration for open/close.
const ANIMATION_DURATION: f64 = 0.25;

/// Convert an absolute path to a relative display path.
/// Shows "project/path/to/file" instead of "/Users/.../project/path/to/file".
fn relative_display_path(path: &Path, root_paths: &[PathBuf]) -> String {
  for root in root_paths {
    if let Some(root_parent) = root.parent()
      && let Ok(stripped) = path.strip_prefix(root_parent)
    {
      return stripped.display().to_string();
    }
  }

  path.display().to_string()
}

/// Renders the file picker overlay.
/// Returns a response indicating what action to take.
pub fn show(ctx: &egui::Context, world: &mut World) -> FilePickerResponse {
  let mut response = FilePickerResponse::None;

  // Get state.
  let (is_visible, _show_preview, animation_start) = {
    let state = world.resource::<FilePickerState>();
    (state.visible, state.show_preview, state.animation_start)
  };

  if !is_visible {
    return response;
  }

  let current_time = ctx.input(|i| i.time);

  // Initialize animation start time.
  let animation_start = animation_start.unwrap_or_else(|| {
    world.resource_mut::<FilePickerState>().animation_start =
      Some(current_time);

    current_time
  });

  // Calculate animation progress.
  let elapsed = current_time - animation_start;
  let animation_t = (elapsed / ANIMATION_DURATION).min(1.0) as f32;
  let eased = Easing::OutCubic.y(animation_t);

  // Track open/close animation via ActiveAnimations.
  if animation_t < 1.0
    && let Some(mut anim) = world.get_resource_mut::<ActiveAnimations>()
  {
    anim.increment();
  }

  let theme = get_theme(world);
  let screen_rect = ctx.input(|i| i.content_rect());

  // Full-screen semi-transparent overlay.
  egui::Area::new(egui::Id::new("file_picker_overlay"))
    .fixed_pos(egui::pos2(0.0, 0.0))
    .order(egui::Order::Foreground)
    .show(ctx, |ui| {
      let overlay_alpha = (180.0 * eased) as u8;

      ui.painter().rect_filled(
        screen_rect,
        10.0,
        egui::Color32::from_black_alpha(overlay_alpha),
      );

      // Handle click outside to close.
      let overlay_response =
        ui.allocate_rect(screen_rect, egui::Sense::click());

      if overlay_response.clicked() {
        response = FilePickerResponse::Close;
      }
    });

  // Dialog: screen minus 32px margin on each side.
  let margin = 56.0;
  let dialog_rect = screen_rect.shrink(margin);
  let dialog_width = dialog_rect.width();
  let dialog_height = dialog_rect.height();

  egui::Area::new(egui::Id::new("file_picker_dialog"))
    .fixed_pos(dialog_rect.min)
    .order(egui::Order::Tooltip)
    .show(ctx, |ui| {
      // Background colors from theme.
      let bg_color = egui::Color32::from_rgba_unmultiplied(
        theme.base[0],
        theme.base[1],
        theme.base[2],
        255,
      );

      let border_color = egui::Color32::from_gray(30);

      egui::Frame::new()
        .fill(bg_color)
        .stroke(egui::Stroke::new(1.0, border_color))
        .corner_radius(8.0)
        .show(ui, |ui| {
          ui.set_min_size(egui::vec2(dialog_width, dialog_height));
          ui.set_max_size(egui::vec2(dialog_width, dialog_height));

          // Search input area.
          let input_response = render_search_input(ui, world, dialog_width);

          if let Some(new_query) = input_response.query_changed {
            world.resource_mut::<FilePickerState>().set_query(new_query);
          }

          ui.separator();

          // Content area: two columns (list + preview), each 50%.
          let content_rect = ui.available_rect_before_wrap();
          let half_width = content_rect.width() / 2.0;

          // Vertical divider between columns.
          let divider_x = content_rect.min.x + half_width;
          let divider_rect = egui::Rect::from_min_size(
            egui::pos2(divider_x, content_rect.min.y),
            egui::vec2(1.0, content_rect.height()),
          );

          let mut divider_ui =
            ui.new_child(egui::UiBuilder::new().max_rect(divider_rect));
          divider::show(&mut divider_ui, Axis::Vertical);

          // Left column: file list.
          // Shrink bottom by corner radius to prevent content overflow.
          let corner_radius = 8.0;
          let left_rect = egui::Rect::from_min_max(
            content_rect.min,
            egui::pos2(
              content_rect.min.x + half_width,
              content_rect.max.y - corner_radius,
            ),
          );

          let mut left_ui = ui.new_child(
            egui::UiBuilder::new()
              .max_rect(left_rect)
              .layout(egui::Layout::top_down(egui::Align::LEFT)),
          );

          left_ui.set_clip_rect(left_rect);
          left_ui.set_min_size(left_rect.size());
          left_ui.set_max_size(left_rect.size());

          let list_response = render_file_list(&mut left_ui, world, theme);

          if let Some(r) = list_response {
            response = r;
          }

          // Right column: preview.
          // Shrink bottom by corner radius to prevent content overflow.
          let right_rect = egui::Rect::from_min_max(
            egui::pos2(content_rect.min.x + half_width, content_rect.min.y),
            egui::pos2(content_rect.max.x, content_rect.max.y - corner_radius),
          );

          let mut right_ui = ui.new_child(
            egui::UiBuilder::new()
              .max_rect(right_rect)
              .layout(egui::Layout::top_down(egui::Align::LEFT)),
          );
          right_ui.set_clip_rect(right_rect);
          right_ui.set_min_size(right_rect.size());
          right_ui.set_max_size(right_rect.size());

          render_preview(&mut right_ui, world, theme);
        });
    });

  // Handle keyboard input.
  let keyboard_response = handle_keyboard(ctx, world);

  if let Some(r) = keyboard_response {
    response = r;
  }

  response
}

/// Input response from the search field.
struct SearchInputResponse {
  query_changed: Option<String>,
}

/// Render the search input field.
fn render_search_input(
  ui: &mut egui::Ui,
  world: &mut World,
  width: f32,
) -> SearchInputResponse {
  let mut response = SearchInputResponse {
    query_changed: None,
  };

  let mut search_input =
    world.resource::<FilePickerState>().search_input.clone();

  // Get delta time for animation.
  let dt = world
    .get_resource::<DeltaTime>()
    .map(|d| d.delta)
    .unwrap_or(0.016);

  ui.horizontal(|ui| {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
      ui.add_space(8.0);

      let text_edit = egui::TextEdit::singleline(&mut search_input)
        .desired_width(width - 120.0)
        .hint_text("Search files...")
        .frame(egui::Frame::NONE)
        .font(egui::TextStyle::Body);

      let edit_response = ui.add(text_edit);

      // Auto-focus on first frame.
      edit_response.request_focus();

      if edit_response.changed() {
        response.query_changed = Some(search_input.clone());
      }
    });

    // Match count.
    if let Some(matcher) = world.resource::<FilePickerMatcher>().get() {
      let count = matcher.matched_count() as f64;
      let total = matcher.total_count();

      // Update animation target and tick.
      let (animated_count, is_animating) = {
        let mut state = world.resource_mut::<FilePickerState>();

        // Set target if count changed.
        if (state.match_count_anim.target - count).abs() > 0.5 {
          state.match_count_anim.set_target(count);
        }

        // Update animation.
        state.match_count_anim.update(dt);

        (
          *state.match_count_anim.value(),
          !state.match_count_anim.is_complete,
        )
      };

      // Track active animation via ECS resource.
      if is_animating
        && let Some(mut anim) = world.get_resource_mut::<ActiveAnimations>()
      {
        anim.increment();
      }

      let display_count = animated_count.round() as usize;

      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        ui.label(
          egui::RichText::new(format!("{display_count}/{total}"))
            .size(12.0)
            .color(egui::Color32::GRAY),
        );
      });
    }
  });

  response
}

/// Render the file list using TableBuilder for proper virtualization.
fn render_file_list(
  ui: &mut egui::Ui,
  world: &mut World,
  theme: &Theme,
) -> Option<FilePickerResponse> {
  let selection = world.resource::<FilePickerState>().selection;
  let prev_selection = world.resource::<FilePickerState>().prev_selection;

  // Get delta time for animations.
  let dt = world
    .get_resource::<DeltaTime>()
    .map(|d| d.delta)
    .unwrap_or(0.016);

  // Tick matcher.
  if let Some(matcher) = world.resource_mut::<FilePickerMatcher>().get_mut() {
    matcher.tick();
  }

  let matched_count = world
    .resource::<FilePickerMatcher>()
    .get()
    .map(|m| m.matched_count())
    .unwrap_or(0);

  if matched_count == 0 {
    ui.centered_and_justified(|ui| {
      ui.label(
        egui::RichText::new("No files found")
          .size(14.0)
          .color(egui::Color32::GRAY),
      );
    });

    return None;
  }

  // Handle selection change - update animations.
  if prev_selection != Some(selection) {
    let mut state = world.resource_mut::<FilePickerState>();

    // Deselect previous row.
    if let Some(prev) = prev_selection {
      state
        .row_padding_anims
        .entry(prev)
        .or_insert_with(|| RowPaddingAnim::new(false))
        .set_selected(false);
    }

    // Select new row.
    state
      .row_padding_anims
      .entry(selection)
      .or_insert_with(|| RowPaddingAnim::new(true))
      .set_selected(true);

    state.prev_selection = Some(selection);
  }

  // Update all row animations and track if any are active.
  let mut any_animating = false;
  {
    let mut state = world.resource_mut::<FilePickerState>();

    for anim in state.row_padding_anims.values_mut() {
      if anim.update(dt) {
        any_animating = true;
      }
    }
  }

  // Track active animations via ECS resource.
  if any_animating
    && let Some(mut anim) = world.get_resource_mut::<ActiveAnimations>()
  {
    anim.increment();
  }

  // Get padding values for visible rows.
  let get_row_padding = |idx: usize| -> f32 {
    world
      .resource::<FilePickerState>()
      .row_padding_anims
      .get(&idx)
      .map(|a| a.value())
      .unwrap_or(RowPaddingAnim::MIN_PADDING)
  };

  let row_height = 24.0;
  let visuals = ui.style().visuals.clone();

  let text_color = egui::Color32::from_rgba_unmultiplied(
    theme.text[0],
    theme.text[1],
    theme.text[2],
    theme.text[3],
  );

  // Theme colors for hover (same as explorer).
  let hover_bg = visuals.widgets.hovered.weak_bg_fill;
  let hover_fg = visuals.widgets.active.weak_bg_fill;

  // Use RefCell to communicate response out of closures.
  let response: RefCell<Option<FilePickerResponse>> = RefCell::new(None);
  let clicked_idx: RefCell<Option<usize>> = RefCell::new(None);

  TableBuilder::new(ui)
    .id_salt("file_picker_table")
    .striped(false)
    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
    .column(Column::remainder())
    .sense(egui::Sense::click())
    .scroll_to_row(selection, None) // Auto-scroll to keep selection visible.
    .body(|body| {
      body.rows(row_height, matched_count, |mut row| {
        let idx = row.index();
        let is_selected = idx == selection;
        let padding = get_row_padding(idx);

        // Get item before col() to avoid borrow issues.
        let item = world
          .resource::<FilePickerMatcher>()
          .get()
          .and_then(|m| m.get(idx).cloned());

        let col_resp = row.col(|ui| {
          ui.style_mut().interaction.selectable_labels = false;

          let row_rect = ui.max_rect();
          let is_hovered = ui.rect_contains_pointer(row_rect);

          // Draw hover background (same as explorer).
          if is_hovered {
            ui.painter().rect_filled(
              row_rect,
              egui::CornerRadius::ZERO,
              hover_bg,
            );
          }

          if let Some(ref item) = item {
            ui.horizontal_centered(|ui| {
              // Animated padding.
              ui.add_space(padding);

              // Selection indicator.
              if is_selected {
                ui.add(
                  icon_to_image(&Icon::Arrow(Arrow::DoubleRight))
                    .fit_to_exact_size(egui::vec2(10.0, 10.0))
                    .tint(egui::Color32::from_rgb(204, 253, 62)),
                );
              } else {
                ui.add_space(8.0);
              }

              ui.add_space(4.0);

              // Colors: inverted on hover, full for selected, dimmed otherwise.
              let (icon_color, name_color, parent_color) = if is_hovered {
                (hover_fg, hover_fg, hover_fg)
              } else if is_selected {
                (
                  egui::Color32::GRAY,
                  egui::Color32::WHITE,
                  egui::Color32::GRAY,
                )
              } else {
                let alpha = 120;
                (
                  egui::Color32::from_rgba_unmultiplied(128, 128, 128, alpha),
                  egui::Color32::from_rgba_unmultiplied(
                    text_color.r(),
                    text_color.g(),
                    text_color.b(),
                    alpha,
                  ),
                  egui::Color32::from_rgba_unmultiplied(128, 128, 128, alpha),
                )
              };

              // File icon.
              let icon = if item.is_dir { " " } else { "" };

              ui.label(egui::RichText::new(icon).size(14.0).color(icon_color));

              ui.add_space(8.0);

              // Filename.
              ui.label(
                egui::RichText::new(&item.name).size(13.0).color(name_color),
              );

              // Parent path (dimmed).
              if !item.parent.is_empty() {
                ui.label(
                  egui::RichText::new(format!(" {}", item.parent))
                    .size(11.0)
                    .color(parent_color),
                );
              }
            });
          }

          // Cursor icon on hover.
          if is_hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
          }
        });

        // Handle clicks using the column response.
        let resp = col_resp.1;

        if let Some(item) = item {
          if resp.clicked() {
            *clicked_idx.borrow_mut() = Some(idx);
          }

          if resp.double_clicked() {
            *response.borrow_mut() = Some(FilePickerResponse::Select(
              item.path.clone(),
              SelectAction::Replace,
            ));
          }
        }
      });
    });

  // Update selection if clicked.
  if let Some(idx) = clicked_idx.borrow().as_ref() {
    world.resource_mut::<FilePickerState>().selection = *idx;
  }

  response.borrow_mut().take()
}

/// Render the preview panel.
fn render_preview(ui: &mut egui::Ui, world: &mut World, theme: &Theme) {
  let selection = world.resource::<FilePickerState>().selection;

  // Get selected item's path.
  let selected_path: Option<PathBuf> = world
    .resource::<FilePickerMatcher>()
    .get()
    .and_then(|m| m.get(selection))
    .map(|item| item.path.clone());

  let Some(path) = selected_path else {
    ui.centered_and_justified(|ui| {
      ui.label(
        egui::RichText::new("No selection")
          .size(14.0)
          .color(egui::Color32::GRAY),
      );
    });

    return;
  };

  // Load or get cached preview.
  let preview = {
    let state = world.resource::<FilePickerState>();

    state.get_preview(&path).cloned()
  };

  let preview = preview.unwrap_or_else(|| {
    let loaded = load_preview(&path);

    world
      .resource_mut::<FilePickerState>()
      .cache_preview(path.clone(), loaded.clone());

    loaded
  });

  // Preview header - show path relative to project root.
  let display_path = {
    let explorer = world.resource::<ExplorerState>();

    relative_display_path(&path, &explorer.roots)
  };

  // Get delta time for animation.
  let dt = world
    .get_resource::<DeltaTime>()
    .map(|d| d.delta)
    .unwrap_or(0.016);

  ui.horizontal(|ui| {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
      ui.add_space(8.0);
      ui.label(
        egui::RichText::new(display_path)
          .size(12.0)
          .strong()
          .color(egui::Color32::WHITE),
      );
    });

    if let CachedPreview::Document { ref content, .. } = preview {
      let line_count = content.lines().count() as f64;

      // Update animation target and tick.
      let (animated_count, is_animating) = {
        let mut state = world.resource_mut::<FilePickerState>();

        // Set target if line count changed.
        if (state.line_count_anim.target - line_count).abs() > 0.5 {
          state.line_count_anim.set_target(line_count);
        }

        // Update animation.
        state.line_count_anim.update(dt);

        (
          *state.line_count_anim.value(),
          !state.line_count_anim.is_complete,
        )
      };

      // Track active animation via ECS resource.
      if is_animating
        && let Some(mut anim) = world.get_resource_mut::<ActiveAnimations>()
      {
        anim.increment();
      }

      let display_count = animated_count.round() as usize;

      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        ui.label(
          egui::RichText::new(format!("{display_count} lines"))
            .size(11.0)
            .color(egui::Color32::GRAY),
        );
      });
    }
  });

  ui.separator();

  // Get token extractors for syntax highlighting.
  let extractors = world.get_resource::<TokenExtractors>();

  // Preview content.
  match preview {
    CachedPreview::Document { content, language } => {
      render_document_preview(
        ui,
        &content,
        language.as_deref(),
        theme,
        extractors,
      );
    }
    CachedPreview::Directory(entries) => {
      render_directory_preview(ui, &entries, theme);
    }
    CachedPreview::Binary => {
      ui.centered_and_justified(|ui| {
        ui.label(
          egui::RichText::new("<Binary file>")
            .size(14.0)
            .color(egui::Color32::GRAY),
        );
      });
    }
    CachedPreview::LargeFile => {
      ui.centered_and_justified(|ui| {
        ui.label(
          egui::RichText::new("<File too large to preview>")
            .size(14.0)
            .color(egui::Color32::GRAY),
        );
      });
    }
    CachedPreview::NotFound => {
      ui.centered_and_justified(|ui| {
        ui.label(
          egui::RichText::new("<File not found>")
            .size(14.0)
            .color(egui::Color32::GRAY),
        );
      });
    }
    CachedPreview::Loading => {
      ui.centered_and_justified(|ui| {
        ui.spinner();
        ui.label(
          egui::RichText::new("Loading...")
            .size(14.0)
            .color(egui::Color32::GRAY),
        );
      });
    }
  }
}

/// Render document preview with line numbers and syntax highlighting.
fn render_document_preview(
  ui: &mut egui::Ui,
  content: &str,
  language: Option<&str>,
  theme: &Theme,
  extractors: Option<&TokenExtractors>,
) {
  let row_height = 16.0;
  let lines: Vec<&str> = content.lines().collect();
  let available_width = ui.available_width();

  // Parse language from extension string.
  let lang = language.map(Language::from).unwrap_or_default();

  let default_color = egui::Color32::from_rgba_unmultiplied(
    theme.text[0],
    theme.text[1],
    theme.text[2],
    theme.text[3],
  );

  let font_id = egui::FontId::monospace(11.0);

  egui::ScrollArea::both()
    .id_salt("file_picker_document_preview")
    .auto_shrink([false, false])
    .show_rows(ui, row_height, lines.len().min(500), |ui, range| {
      ui.set_min_width(available_width);

      for line_num in range {
        let line = lines.get(line_num).unwrap_or(&"");

        ui.horizontal(|ui| {
          // Line number gutter.
          ui.label(
            egui::RichText::new(format!("{:4} ", line_num + 1))
              .size(11.0)
              .color(egui::Color32::GRAY)
              .monospace(),
          );

          // Tokenize and render with syntax highlighting.
          let tokens = extractors
            .map(|ext| ext.extract(lang, line))
            .unwrap_or_default();

          let galley = build_highlighted_galley(
            ui,
            line,
            &font_id,
            &tokens,
            default_color,
          );

          ui.label(galley);
        });
      }
    });
}

/// Build a syntax-highlighted galley for a line.
fn build_highlighted_galley(
  _ui: &egui::Ui,
  line_text: &str,
  font_id: &egui::FontId,
  tokens: &[Token],
  default_color: egui::Color32,
) -> egui::WidgetText {
  let mut job = LayoutJob::default();

  if line_text.is_empty() {
    job.append(
      "",
      0.0,
      TextFormat {
        font_id: font_id.clone(),
        color: default_color,
        ..Default::default()
      },
    );
    return job.into();
  }

  if tokens.is_empty() {
    job.append(
      line_text,
      0.0,
      TextFormat {
        font_id: font_id.clone(),
        color: default_color,
        ..Default::default()
      },
    );
    return job.into();
  }

  let mut pos = 0;

  for token in tokens {
    let tok_start = token.start;
    let tok_end = token.end.min(line_text.len());

    if tok_end <= pos {
      continue;
    }

    let effective_start = tok_start.max(pos);

    // Gap before token.
    if effective_start > pos {
      job.append(
        &line_text[pos..effective_start],
        0.0,
        TextFormat {
          font_id: font_id.clone(),
          color: default_color,
          ..Default::default()
        },
      );
    }

    // Token with syntax color.
    if tok_end > effective_start && effective_start < line_text.len() {
      let token_text =
        &line_text[effective_start..tok_end.min(line_text.len())];
      let color = syntax_color(token.kind);

      job.append(
        token_text,
        0.0,
        TextFormat {
          font_id: font_id.clone(),
          color,
          ..Default::default()
        },
      );

      pos = tok_end;
    }
  }

  // Remaining text after last token.
  if pos < line_text.len() {
    job.append(
      &line_text[pos..],
      0.0,
      TextFormat {
        font_id: font_id.clone(),
        color: default_color,
        ..Default::default()
      },
    );
  }

  job.into()
}

/// Render directory preview.
fn render_directory_preview(
  ui: &mut egui::Ui,
  entries: &[codelord_core::file_picker::components::DirEntry],
  theme: &Theme,
) {
  egui::ScrollArea::vertical()
    .id_salt("file_picker_directory_preview")
    .auto_shrink([false, false])
    .show(ui, |ui| {
      for entry in entries.iter().take(100) {
        let icon = if entry.is_dir { " " } else { "" };

        let name = if entry.is_dir {
          format!("{}/", entry.name)
        } else {
          entry.name.clone()
        };

        ui.horizontal(|ui| {
          ui.add_space(8.0);

          ui.label(
            egui::RichText::new(icon)
              .size(14.0)
              .color(egui::Color32::GRAY),
          );

          ui.add_space(8.0);

          let text_color = egui::Color32::from_rgba_unmultiplied(
            theme.text[0],
            theme.text[1],
            theme.text[2],
            theme.text[3],
          );

          ui.label(egui::RichText::new(name).size(12.0).color(text_color));
        });
      }
    });
}

/// Handle keyboard navigation.
fn handle_keyboard(
  ctx: &egui::Context,
  world: &mut World,
) -> Option<FilePickerResponse> {
  let mut response = None;

  ctx.input(|i| {
    for event in &i.events {
      if let egui::Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
      } = event
      {
        match (key, modifiers.ctrl, modifiers.shift) {
          // Navigation.
          (egui::Key::ArrowUp, false, _) | (egui::Key::P, true, _) => {
            let count = world
              .resource::<FilePickerMatcher>()
              .get()
              .map(|m| m.matched_count())
              .unwrap_or(0);

            world
              .resource_mut::<FilePickerState>()
              .move_selection(-1, count);
          }
          (egui::Key::ArrowDown, false, _) | (egui::Key::N, true, _) => {
            let count = world
              .resource::<FilePickerMatcher>()
              .get()
              .map(|m| m.matched_count())
              .unwrap_or(0);

            world
              .resource_mut::<FilePickerState>()
              .move_selection(1, count);
          }
          (egui::Key::PageUp, _, _) | (egui::Key::U, true, _) => {
            let count = world
              .resource::<FilePickerMatcher>()
              .get()
              .map(|m| m.matched_count())
              .unwrap_or(0);

            world.resource_mut::<FilePickerState>().page_up(count);
          }
          (egui::Key::PageDown, _, _) | (egui::Key::D, true, _) => {
            let count = world
              .resource::<FilePickerMatcher>()
              .get()
              .map(|m| m.matched_count())
              .unwrap_or(0);

            world.resource_mut::<FilePickerState>().page_down(count);
          }
          (egui::Key::Home, _, _) => {
            world.resource_mut::<FilePickerState>().selection = 0;
          }
          (egui::Key::End, _, _) => {
            let count = world
              .resource::<FilePickerMatcher>()
              .get()
              .map(|m| m.matched_count())
              .unwrap_or(0);

            world.resource_mut::<FilePickerState>().selection =
              count.saturating_sub(1);
          }

          // Selection.
          (egui::Key::Enter, false, false) => {
            let selection = world.resource::<FilePickerState>().selection;

            if let Some(item) = world
              .resource::<FilePickerMatcher>()
              .get()
              .and_then(|m| m.get(selection))
            {
              response = Some(FilePickerResponse::Select(
                item.path.clone(),
                SelectAction::Replace,
              ));
            }
          }
          (egui::Key::Enter, false, true) => {
            // Shift+Enter: open in new tab.
            let selection = world.resource::<FilePickerState>().selection;

            if let Some(item) = world
              .resource::<FilePickerMatcher>()
              .get()
              .and_then(|m| m.get(selection))
            {
              response = Some(FilePickerResponse::Select(
                item.path.clone(),
                SelectAction::NewTab,
              ));
            }
          }

          // Split actions.
          (egui::Key::S, true, _) => {
            let selection = world.resource::<FilePickerState>().selection;

            if let Some(item) = world
              .resource::<FilePickerMatcher>()
              .get()
              .and_then(|m| m.get(selection))
            {
              response = Some(FilePickerResponse::Select(
                item.path.clone(),
                SelectAction::HSplit,
              ));
            }
          }
          (egui::Key::V, true, _) => {
            let selection = world.resource::<FilePickerState>().selection;

            if let Some(item) = world
              .resource::<FilePickerMatcher>()
              .get()
              .and_then(|m| m.get(selection))
            {
              response = Some(FilePickerResponse::Select(
                item.path.clone(),
                SelectAction::VSplit,
              ));
            }
          }

          // Toggle preview.
          (egui::Key::T, true, _) => {
            let mut state = world.resource_mut::<FilePickerState>();

            state.show_preview = !state.show_preview;
          }

          // Close.
          (egui::Key::Escape, _, _) => {
            response = Some(FilePickerResponse::Close);
          }

          _ => {}
        }
      }
    }
  });

  response
}
