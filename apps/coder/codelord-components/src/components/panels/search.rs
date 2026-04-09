//! Search UI component for find/replace functionality.
//!
//! This module provides the search panel UI following DOD principles.
//! The UI is stateless - it takes search data and produces events.

use crate::assets::icon::icon_to_image;

use codelord_core::ecs::world::World;
use codelord_core::events::{
  FindNextRequest, FindPreviousRequest, HideSearchRequest,
  ToggleSearchOptionRequest, UpdateSearchQueryRequest,
};
use codelord_core::icon::components::{Arrow, Icon};
use codelord_core::search::{SearchOption, SearchState};

use eazy::{Curve, Easing};
use eframe::egui;

/// Search type words that rotate in the animated placeholder.
const SEARCH_WORDS: &[&str] = &["file", "folder", "commit", "symbol", "text"];

/// Duration for each word to stay visible (in seconds).
const WORD_DISPLAY_DURATION: f32 = 2.0;

/// Duration of the slide animation (in seconds).
const SLIDE_DURATION: f32 = 0.5;

/// Calculate the current animated hint text state.
fn calculate_hint_animation(ctx: &egui::Context) -> (usize, f32) {
  let time = ctx.input(|i| i.time);
  let cycle_duration = WORD_DISPLAY_DURATION + SLIDE_DURATION;
  let total_time = time as f32;

  let current_cycle = (total_time / cycle_duration) as usize;
  let time_in_cycle = total_time % cycle_duration;

  let current_word_index = current_cycle % SEARCH_WORDS.len();

  let raw_progress = if time_in_cycle > WORD_DISPLAY_DURATION {
    ((time_in_cycle - WORD_DISPLAY_DURATION) / SLIDE_DURATION).min(1.0)
  } else {
    0.0
  };

  let eased_progress = Easing::InOutCubic.y(raw_progress);

  (current_word_index, eased_progress)
}

/// Render animated hint text with vertical slide transition.
fn render_animated_hint(ui: &mut egui::Ui, rect: egui::Rect) {
  let clip_rect = ui.clip_rect().intersect(rect);

  let (current_index, progress) = calculate_hint_animation(ui.ctx());
  let next_index = (current_index + 1) % SEARCH_WORDS.len();

  let current_word = SEARCH_WORDS[current_index];
  let next_word = SEARCH_WORDS[next_index];

  let font_id = egui::FontId::proportional(14.0);
  let prefix_color = egui::Color32::from_gray(100);
  let word_color = egui::Color32::from_rgb(204, 253, 62); // Green

  let line_height = 16.0;
  let current_offset = -progress * line_height;
  let next_offset = line_height * (1.0 - progress);

  let prefix = "SEARCH ";
  let prefix_galley = ui.painter().layout_no_wrap(
    prefix.to_string(),
    font_id.clone(),
    prefix_color,
  );
  let prefix_pos = rect.left_top() + egui::vec2(6.0, 2.0);

  if clip_rect.contains(prefix_pos) {
    ui.painter()
      .galley(prefix_pos, prefix_galley.clone(), prefix_color);
  }

  let prefix_width = prefix_galley.rect.width();

  let word_rect = egui::Rect::from_min_size(
    rect.left_top() + egui::vec2(prefix_width + 8.0, 0.5),
    egui::vec2(100.0, line_height),
  );

  let word_clip_rect = clip_rect.intersect(word_rect);

  ui.scope(|ui| {
    ui.set_clip_rect(word_clip_rect);
    let painter = ui.painter();

    let current_alpha = (1.0 - progress).clamp(0.0, 1.0);
    let current_color = word_color.linear_multiply(current_alpha);
    let current_galley = painter.layout_no_wrap(
      current_word.to_string(),
      font_id.clone(),
      current_color,
    );
    let current_pos =
      word_rect.left_top() + egui::vec2(0.0, current_offset + 1.0);
    painter.galley(current_pos, current_galley, current_color);

    let next_alpha = progress.clamp(0.0, 1.0);
    let next_color = word_color.linear_multiply(next_alpha);
    let next_galley =
      painter.layout_no_wrap(next_word.to_string(), font_id, next_color);
    let next_pos = word_rect.left_top() + egui::vec2(0.0, next_offset + 1.0);
    painter.galley(next_pos, next_galley, next_color);
  });
}

/// Render the search panel if visible.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  // Track focus request per search session using egui's temp data storage
  let search_input_id = ui.id().with("search_input");
  let focus_requested_id = ui.id().with("search_focus_requested");

  // Get search state
  let Some(search_state) = world.get_resource::<SearchState>() else {
    return;
  };

  if !search_state.visible {
    // Reset the focus request flag when search is hidden
    ui.ctx()
      .data_mut(|d| d.insert_temp(focus_requested_id, false));
    return;
  }

  // Request focus only once when search becomes visible
  let focus_already_requested = ui
    .ctx()
    .data_mut(|d| d.get_temp::<bool>(focus_requested_id).unwrap_or(false));

  if !focus_already_requested {
    ui.ctx()
      .memory_mut(|mem| mem.request_focus(search_input_id));
    ui.ctx()
      .data_mut(|d| d.insert_temp(focus_requested_id, true));
  }

  // Clone values to avoid borrow issues
  let mut query = search_state.query.clone();
  let total_matches = search_state.total_matches;
  let current_match_index = search_state.current_match_index;
  let case_sensitive = search_state.case_sensitive;
  let whole_word = search_state.whole_word;
  let regex_mode = search_state.regex_mode;

  // Track events to spawn
  let mut hcodelord_search = false;
  let mut find_next = false;
  let mut find_prev = false;
  let mut toggle_case = false;
  let mut toggle_whole = false;
  let mut toggle_regex = false;
  let mut query_changed = false;

  // Get theme colors
  let text_color = ui.visuals().text_color();
  let weak_color = ui.visuals().weak_text_color();
  let green = egui::Color32::from_rgb(204, 253, 62);
  let red = egui::Color32::from_rgb(255, 100, 100);

  egui::Frame::new()
    .inner_margin(egui::Margin::same(8))
    .show(ui, |ui| {
      let available_width = ui.available_width();

      ui.allocate_ui_with_layout(
        egui::vec2(available_width, 24.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
          // Close button
          let button_close = ui.add(
            egui::Button::image(
              icon_to_image(&Icon::Close)
                .fit_to_exact_size(egui::vec2(12.0, 12.0))
                .tint(text_color),
            )
            .frame(false),
          );
          if button_close.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
          }
          if button_close.clicked() {
            hcodelord_search = true;
          }

          ui.separator();

          // Search icon
          ui.add(
            icon_to_image(&Icon::Search)
              .fit_to_exact_size(egui::vec2(16.0, 16.0))
              .tint(text_color),
          );

          // Search input with options
          egui::Frame::NONE
            .stroke(egui::Stroke::new(1.0, weak_color))
            .corner_radius(0.0)
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
              ui.set_width(464.0);
              ui.set_height(16.0);

              ui.horizontal(|ui| {
                let available_width = ui.available_width() - 120.0;
                let (rect, _) = ui.allocate_exact_size(
                  egui::vec2(available_width, 16.0),
                  egui::Sense::hover(),
                );

                if query.is_empty() {
                  render_animated_hint(ui, rect);
                }

                let text_edit_id = ui.id().with("search_input");

                let input_color = if !query.is_empty() && total_matches == 0 {
                  red
                } else {
                  text_color
                };

                let text_edit_response = ui.put(
                  rect,
                  egui::TextEdit::singleline(&mut query)
                    .id(text_edit_id)
                    .background_color(egui::Color32::TRANSPARENT)
                    .text_color(input_color)
                    .frame(egui::Frame::NONE)
                    .hint_text(""),
                );

                if text_edit_response.changed() {
                  query_changed = true;
                }

                // Handle Enter/Escape
                if text_edit_response.has_focus() {
                  if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    find_next = true;
                  }
                  if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    hcodelord_search = true;
                  }
                }

                ui.with_layout(
                  egui::Layout::right_to_left(egui::Align::Center),
                  |ui| {
                    // Regex mode button
                    let regex_color =
                      if regex_mode { green } else { text_color };
                    let button_regex = ui.add(
                      egui::Button::new(
                        egui::RichText::new(".*").color(regex_color),
                      )
                      .frame(false),
                    );
                    if button_regex.clicked() {
                      toggle_regex = true;
                    }

                    // Whole word button
                    let whole_color =
                      if whole_word { green } else { text_color };
                    let button_whole = ui.add(
                      egui::Button::new(
                        egui::RichText::new("Ab").color(whole_color),
                      )
                      .frame(false),
                    );
                    if button_whole.clicked() {
                      toggle_whole = true;
                    }

                    // Case sensitive button
                    let case_color =
                      if case_sensitive { green } else { text_color };
                    let button_case = ui.add(
                      egui::Button::new(
                        egui::RichText::new("Aa").color(case_color),
                      )
                      .frame(false),
                    );
                    if button_case.clicked() {
                      toggle_case = true;
                    }
                  },
                );
              });
            });

          // Navigate buttons
          let prev_enabled = total_matches > 0;
          let next_enabled = total_matches > 0;

          if ui
            .add_enabled(
              prev_enabled,
              egui::Button::new(
                icon_to_image(&Icon::Arrow(Arrow::AngleLeftLine))
                  .fit_to_exact_size(egui::vec2(16.0, 16.0))
                  .tint(text_color),
              )
              .fill(egui::Color32::TRANSPARENT)
              .frame(false)
              .stroke(egui::Stroke::new(1.0, weak_color))
              .corner_radius(0.0)
              .min_size(egui::vec2(32.0, 32.0)),
            )
            .clicked()
          {
            find_prev = true;
          }

          if ui
            .add_enabled(
              next_enabled,
              egui::Button::new(
                icon_to_image(&Icon::Arrow(Arrow::AngleRightLine))
                  .fit_to_exact_size(egui::vec2(16.0, 16.0))
                  .tint(text_color),
              )
              .fill(egui::Color32::TRANSPARENT)
              .frame(false)
              .stroke(egui::Stroke::new(1.0, weak_color))
              .corner_radius(0.0)
              .min_size(egui::vec2(32.0, 32.0)),
            )
            .clicked()
          {
            find_next = true;
          }

          // Match counter
          if total_matches > 0 {
            ui.label(
              egui::RichText::new(format!(
                "{}/{}",
                current_match_index + 1,
                total_matches
              ))
              .color(text_color),
            );
          } else if !query.is_empty() {
            ui.colored_label(ui.style().visuals.warn_fg_color, "No matches");
          }
        },
      );
    });

  // Spawn events outside the UI closure
  if hcodelord_search {
    world.spawn(HideSearchRequest);
  }
  if find_next {
    world.spawn(FindNextRequest);
  }
  if find_prev {
    world.spawn(FindPreviousRequest);
  }
  if toggle_case {
    world.spawn(ToggleSearchOptionRequest::new(SearchOption::CaseSensitive));
  }
  if toggle_whole {
    world.spawn(ToggleSearchOptionRequest::new(SearchOption::WholeWord));
  }
  if toggle_regex {
    world.spawn(ToggleSearchOptionRequest::new(SearchOption::RegexMode));
  }
  if query_changed {
    world.spawn(UpdateSearchQueryRequest::new(query));
  }
}

/// Find all matches in text using simple string search.
pub fn find_matches(
  text: &str,
  query: &str,
  case_sensitive: bool,
  whole_word: bool,
  _regex_mode: bool,
) -> Vec<(usize, usize)> {
  if query.is_empty() {
    return Vec::new();
  }

  let mut matches = Vec::new();

  let search_text = if case_sensitive {
    text.to_string()
  } else {
    text.to_lowercase()
  };

  let search_query = if case_sensitive {
    query.to_string()
  } else {
    query.to_lowercase()
  };

  let mut start = 0;
  while let Some(pos) = search_text[start..].find(&search_query) {
    let match_start = start + pos;
    let match_end = match_start + search_query.len();

    if whole_word {
      let before_ok = match_start == 0
        || !text
          .chars()
          .nth(match_start - 1)
          .unwrap_or(' ')
          .is_alphanumeric();
      let after_ok = match_end >= text.len()
        || !text.chars().nth(match_end).unwrap_or(' ').is_alphanumeric();

      if before_ok && after_ok {
        matches.push((match_start, match_end));
      }
    } else {
      matches.push((match_start, match_end));
    }

    start = match_end;
  }

  matches
}
