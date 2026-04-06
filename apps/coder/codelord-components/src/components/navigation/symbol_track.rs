//! Symbol track scrollbar - minimal symbol overview on right edge.
//!
//! Grok-style ultra-narrow scrollbar showing symbols as markers.
//! Matches codelord's implementation exactly.

use crate::assets::theme::syntax_color;

use codelord_core::symbol::{
  SymbolAnchor, SymbolKind, SymbolMap, SymbolStatus,
};
use codelord_core::token::TokenKind;

use eframe::egui;

/// Width of the symbol track in pixels.
pub const SYMBOL_TRACK_WIDTH: f32 = 8.0;

/// Right margin from editor edge.
const RIGHT_MARGIN: f32 = 8.0;

/// Spacing between symbol markers.
const SYMBOL_SPACING: f32 = 16.0;

/// Visual height of each marker area.
const MARKER_HEIGHT: f32 = 12.0;

/// Threshold for finding nearest symbol on background click.
const CLICK_THRESHOLD_LINES: usize = 10;

/// Persistent state for the symbol track.
#[derive(Default, Clone)]
struct SymbolTrackState {
  /// Currently active symbol line (persists after click).
  active_symbol_line: Option<usize>,
  /// Vertical scroll offset for the symbol track itself.
  scroll_offset: f32,
}

/// Result from rendering the symbol track.
pub struct SymbolTrackResult {
  /// Line number if user clicked a symbol.
  pub clicked_line: Option<usize>,
  /// Whether mouse is hovering the symbol track.
  pub is_hovered: bool,
}

/// Render the symbol track scrollbar.
pub fn render(
  ui: &mut egui::Ui,
  scroll_area_rect: egui::Rect,
  scroll_offset_y: f32,
  content_height: f32,
  viewport_height: f32,
  symbol_map: &SymbolMap,
  total_lines: usize,
) -> SymbolTrackResult {
  // Load persistent state.
  let state_id = ui.id().with("symbol_track_state");
  let mut state = ui.memory_mut(|mem| {
    mem
      .data
      .get_temp::<SymbolTrackState>(state_id)
      .unwrap_or_default()
  });

  // Calculate scrollbar rect with right margin.
  let scrollbar_rect = egui::Rect::from_min_size(
    egui::pos2(
      scroll_area_rect.right() - SYMBOL_TRACK_WIDTH - RIGHT_MARGIN,
      scroll_area_rect.top(),
    ),
    egui::vec2(SYMBOL_TRACK_WIDTH, scroll_area_rect.height()),
  );

  // Allocate for interaction.
  let response = ui.allocate_rect(scrollbar_rect, egui::Sense::click());

  // Draw background (nearly black).
  ui.painter()
    .rect_filled(scrollbar_rect, 0.0, egui::Color32::from_gray(5));

  let mut clicked_line_result: Option<usize> = None;
  let mut hovered_anchor: Option<(&SymbolAnchor, f32)> = None;

  if !symbol_map.anchors.is_empty() {
    // Calculate content height for symbol track scrolling.
    let total_symbol_height = symbol_map.anchors.len() as f32 * SYMBOL_SPACING;
    let track_viewport = scrollbar_rect.height();

    // Handle scroll input on the scrollbar.
    let scroll_response = ui.interact(
      scrollbar_rect,
      ui.id().with("symbol_track_scroll"),
      egui::Sense::click_and_drag(),
    );

    if scroll_response.dragged() {
      state.scroll_offset -= scroll_response.drag_delta().y;
    }

    // Handle scroll wheel when mouse is over scrollbar.
    let mouse_pos = ui.input(|i| i.pointer.hover_pos());
    let is_mouse_over =
      mouse_pos.is_some_and(|pos| scrollbar_rect.contains(pos));

    if is_mouse_over {
      let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
      if scroll_delta.abs() > 0.0 {
        state.scroll_offset -= scroll_delta;
      }
    }

    // Clamp scroll offset.
    let max_scroll = (total_symbol_height - track_viewport).max(0.0);
    state.scroll_offset = state.scroll_offset.clamp(0.0, max_scroll);

    // Render each symbol marker.
    for (index, anchor) in symbol_map.anchors.iter().enumerate() {
      // Calculate Y position with spacing and scroll offset.
      let y_unscrolled = index as f32 * SYMBOL_SPACING + MARKER_HEIGHT / 2.0;
      let y = scrollbar_rect.min.y + y_unscrolled - state.scroll_offset;

      // Skip symbols outside viewport.
      if y < scrollbar_rect.min.y - MARKER_HEIGHT
        || y > scrollbar_rect.max.y + MARKER_HEIGHT
      {
        continue;
      }

      // Get base color from symbol kind or status.
      let base_color = match anchor.status {
        SymbolStatus::Error => {
          egui::Color32::from_rgba_premultiplied(220, 80, 80, 255)
        }
        SymbolStatus::Warning => {
          egui::Color32::from_rgba_premultiplied(220, 150, 80, 230)
        }
        _ => symbol_kind_color(anchor.kind),
      };

      // Create hover rect.
      let hover_rect = egui::Rect::from_min_size(
        egui::pos2(scrollbar_rect.left(), y - 5.0),
        egui::vec2(SYMBOL_TRACK_WIDTH, 10.0),
      );

      let marker_response = ui.interact(
        hover_rect,
        ui.id().with(("symbol_marker", anchor.byte_range.start)),
        egui::Sense::click_and_drag(),
      );

      let is_hovered = marker_response.hovered();
      let is_active = state.active_symbol_line == Some(anchor.line);

      // Handle click.
      if marker_response.clicked() {
        state.active_symbol_line = Some(anchor.line);
        clicked_line_result = Some(anchor.line);
      }

      // Draw marker based on symbol kind.
      let painter = ui.painter();
      match anchor.kind {
        SymbolKind::Function => {
          // Function: horizontal bar growing from right.
          // Normal: 4px, Hovered: 8px, Active: 16px
          let (bar_width, color) = if is_active {
            (16.0, egui::Color32::WHITE)
          } else if is_hovered {
            (8.0, egui::Color32::WHITE)
          } else {
            (4.0, base_color)
          };

          let bar_rect = egui::Rect::from_min_size(
            egui::pos2(scrollbar_rect.right() - bar_width, y - 0.5),
            egui::vec2(bar_width, 1.0),
          );
          painter.rect_filled(bar_rect, 0.5, color);
        }
        _ => {
          // Others: circular dots.
          // Normal: 2px radius, Hovered/Active: 4px radius
          let (radius, color) = if is_active || is_hovered {
            (4.0, egui::Color32::WHITE)
          } else {
            (2.0, base_color)
          };

          let center = egui::pos2(scrollbar_rect.right() - radius, y);
          painter.circle_filled(center, radius, color);
        }
      }

      // Track hovered anchor for tooltip (rendered after painting).
      if is_hovered {
        hovered_anchor = Some((anchor, y));
      }
    }
  }

  // Draw viewport indicator (subtle white overlay).
  if content_height > 0.0 {
    let start_ratio = scroll_offset_y / content_height;
    let height_ratio = viewport_height / content_height;

    let indicator_rect = egui::Rect::from_min_size(
      egui::pos2(
        scrollbar_rect.min.x,
        scrollbar_rect.min.y + start_ratio * scrollbar_rect.height(),
      ),
      egui::vec2(SYMBOL_TRACK_WIDTH, height_ratio * scrollbar_rect.height()),
    );

    ui.painter().rect_filled(
      indicator_rect,
      0.0,
      egui::Color32::from_white_alpha(15),
    );
  }

  // Render tooltip after all painting is done (requires mutable ui borrow).
  if let Some((anchor, y)) = hovered_anchor {
    render_tooltip(ui, anchor, scrollbar_rect.left(), y);
  }

  // Handle click on background (not on a symbol marker).
  if response.clicked()
    && clicked_line_result.is_none()
    && let Some(click_pos) = response.interact_pointer_pos()
  {
    let click_y = click_pos.y - scrollbar_rect.min.y;
    let click_ratio = click_y / scrollbar_rect.height();
    let clicked_line = (click_ratio * total_lines as f32) as usize;

    // Find closest symbol within threshold.
    let closest_symbol = symbol_map
      .anchors
      .iter()
      .filter(|a| {
        (a.line as i32 - clicked_line as i32).abs()
          <= CLICK_THRESHOLD_LINES as i32
      })
      .min_by_key(|a| (a.line as i32 - clicked_line as i32).abs());

    if let Some(symbol) = closest_symbol {
      state.active_symbol_line = Some(symbol.line);
      clicked_line_result = Some(symbol.line);
    } else {
      state.active_symbol_line = None;
      clicked_line_result =
        Some(clicked_line.min(total_lines.saturating_sub(1)));
    }
  }

  // Save state.
  ui.memory_mut(|mem| {
    mem.data.insert_temp(state_id, state);
  });

  SymbolTrackResult {
    clicked_line: clicked_line_result,
    is_hovered: response.hovered(),
  }
}

/// Render tooltip with slide animation.
fn render_tooltip(
  ui: &mut egui::Ui,
  anchor: &SymbolAnchor,
  left_x: f32,
  y: f32,
) {
  let tooltip_id = ui.id().with(("tooltip", anchor.line));

  // Animate slide: 0.0 to 1.0 over 150ms.
  let anim_progress = ui.ctx().animate_bool_with_time(tooltip_id, true, 0.15);
  let slcodelord_offset = (1.0 - anim_progress) * 30.0;

  let tooltip_pos = egui::pos2(left_x - 10.0 - slcodelord_offset, y);

  egui::Area::new(tooltip_id)
    .order(egui::Order::Tooltip)
    .fixed_pos(tooltip_pos)
    .pivot(egui::Align2::RIGHT_CENTER)
    .show(ui.ctx(), |ui| {
      egui::Frame::NONE
        .corner_radius(egui::CornerRadius::ZERO)
        .fill(egui::Color32::BLACK)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .outer_margin(egui::Margin {
          top: 0,
          right: 8,
          bottom: 0,
          left: 0,
        })
        .show(ui, |ui| {
          ui.style_mut().spacing.item_spacing = egui::vec2(8.0, 4.0);

          ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            // Render syntax-highlighted display text.
            render_highlighted_text(
              ui,
              &anchor.display_text,
              &anchor.highlight_ranges,
            );

            // Add location info.
            ui.label(
              egui::RichText::new(format!(
                ":{}:{}",
                anchor.line + 1,
                anchor.col
              ))
              .color(egui::Color32::from_gray(160))
              .size(11.0),
            );
          });
        });
    });
}

/// Render text with syntax highlighting.
fn render_highlighted_text(
  ui: &mut egui::Ui,
  text: &str,
  highlights: &[(std::ops::Range<usize>, u8)],
) {
  if text.is_empty() {
    return;
  }

  if highlights.is_empty() {
    ui.label(
      egui::RichText::new(text)
        .color(egui::Color32::from_gray(200))
        .size(11.0),
    );

    return;
  }

  let mut last_end = 0;
  for (range, token_type_u8) in highlights {
    // Render gap before this span.
    if range.start > last_end && last_end < text.len() {
      let gap = &text[last_end..range.start.min(text.len())];

      ui.label(
        egui::RichText::new(gap)
          .color(egui::Color32::from_gray(200))
          .size(11.0),
      );
    }

    // Render highlighted span.
    if range.end <= text.len() {
      let span_text = &text[range.clone()];
      let color = syntax_color(unsafe {
        std::mem::transmute::<u8, TokenKind>(*token_type_u8)
      });

      ui.label(egui::RichText::new(span_text).color(color).size(11.0));
    }

    last_end = range.end;
  }

  // Render remaining text.
  if last_end < text.len() {
    let remainder = &text[last_end..];

    ui.label(
      egui::RichText::new(remainder)
        .color(egui::Color32::from_gray(200))
        .size(11.0),
    );
  }
}

/// Map symbol kind to syntax highlighting color.
fn symbol_kind_color(kind: SymbolKind) -> egui::Color32 {
  syntax_color(match kind {
    SymbolKind::Function => TokenKind::IdentifierFunction,
    SymbolKind::Struct => TokenKind::IdentifierType,
    SymbolKind::Enum => TokenKind::Attribute,
    SymbolKind::Trait => TokenKind::Keyword,
    SymbolKind::Impl => TokenKind::LiteralBool,
    SymbolKind::Import => TokenKind::LiteralString,
    SymbolKind::Const => TokenKind::IdentifierConstant,
    SymbolKind::Module => TokenKind::LiteralNumber,
  })
}
