//! # Editor Content Renderer
//!
//! High-performance text editor rendering with syntax highlighting.
//!
//! ## Architecture Overview
//!
//! This module implements a virtualized text renderer optimized for large files
//! (tested with 2k+ line files at 120 FPS). The key insight is that both
//! rendering AND tokenization should only process visible/affected content.
//!
//! ## Performance Design Principles
//!
//! ### 1. Virtualized Rendering (O(visible_lines))
//!
//! Only visible lines are rendered. For a 2000-line file with 50 visible lines,
//! we process 50 lines, not 2000. The render loop:
//!
//! ```text
//! for line_idx in 0..total_lines {
//!   if line_idx < first_visible || line_idx > last_visible {
//!     continue; // Skip non-visible lines
//!   }
//!   // Render only this visible line
//! }
//! ```
//!
//! ### 2. Per-Line Tokenization (O(1) on edit)
//!
//! **Critical optimization**: Tokens are cached per-line, not per-file.
//! When the user types, only the edited line is re-tokenized.
//!
//! ```text
//! LineTokenCache {
//!   lines: HashMap<line_idx, (content_hash, Vec<Token>)>
//! }
//! ```
//!
//! - **File open**: Tokenize only visible lines (~50 lines)
//! - **Typing**: Re-tokenize only the edited line (1 line!)
//! - **Scrolling**: Tokenize newly visible lines on demand
//! - **Cache hit**: O(1) hash comparison per line
//!
//! ### 3. Galley Caching (O(1) for unchanged lines)
//!
//! egui galleys (rendered text layouts) are cached per-line using content hash.
//! Unchanged lines reuse their cached galley without re-layout.
//!
//! ### 4. Rope Data Structure (O(log n) edits)
//!
//! Text is stored in a rope (`ropey` crate), enabling:
//! - O(log n) character insertion/deletion
//! - O(log n) line-to-byte conversion
//! - O(1) line count queries
//!
//! ## Data Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ECS World (Bevy)                         │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Entity (Tab)                                               │
//! │  ├── TextBuffer    : Rope + cached line offsets            │
//! │  ├── Cursor        : Position + selection                  │
//! │  ├── FileTab       : Path + language detection             │
//! │  ├── TabSymbols    : Parsed symbols + fold state           │
//! │  └── TabBlame      : Git blame + animation state           │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    show<M>() Renderer                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │  1. Query active tab entity                                 │
//! │  2. Get/create LineTokenCache from egui memory              │
//! │  3. Calculate visible line range from scroll offset         │
//! │  4. For each visible line:                                  │
//! │     a. Get line text from rope (O(log n))                  │
//! │     b. Hash line text (O(line_length))                     │
//! │     c. Cache hit? Use cached tokens                        │
//! │        Cache miss? Tokenize just this line                 │
//! │     d. Build galley with syntax colors                     │
//! │     e. Paint galley                                        │
//! │  5. Save updated token cache to egui memory                │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Related Components (codelord-core)
//!
//! ### TextBuffer (`text_editor/components.rs`)
//! - `rope: Rope` - The actual text content
//! - `line(idx)` - O(log n) line access
//! - `char_to_line_col()` - O(log n) cursor position calculation
//!
//! ### Cursor (`text_editor/components.rs`)
//! - `position: usize` - Character index in buffer
//! - `selection()` - Optional (start, end) range
//!
//! ### TokenExtractors (`token.rs`)
//! - Registry of language-specific tokenizers
//! - `extract(language, text) -> Vec<Token>`
//!
//! ## Related Systems (codelord-core)
//!
//! ### insert_text_system / delete_text_system
//! - Handle text mutations via events
//! - Mark TabSymbols dirty for re-parsing
//!
//! ### move_cursor_system / set_cursor_system
//! - Handle cursor movement via events
//! - Support selection extension with shift key

use crate::assets::icon::icon_to_image;
use crate::assets::theme::syntax_color;
use crate::components::instructions;
use crate::components::navigation::symbol_track;

use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::animation::shimmer::ShimmerAnimation;
use codelord_core::color::{ColorExtractor, ColorInfo, ColorPickerState};
use codelord_core::ecs::component::Component;
use codelord_core::ecs::entity::Entity;
use codelord_core::ecs::prelude::With;
use codelord_core::ecs::world::World;
use codelord_core::events::{
  CursorMovement, DeleteTextEvent, InsertTextEvent, MoveCursorEvent,
  SetCursorEvent, ToggleFoldRequest,
};
use codelord_core::git::components::TabBlame;
use codelord_core::icon::components::{Arrow, Icon};
use codelord_core::keyboard::{FocusRequest, KeyboardFocus};
use codelord_core::playground::PlaygroundHoveredSpan;
use codelord_core::symbol::{
  StickyScrollSettings, SymbolAnchor, SymbolMap, TabSymbols,
};
use codelord_core::tabbar::components::{EditorTab, PlaygroundTab};
use codelord_core::text_editor::components::{Cursor, FileTab, TextBuffer};
use codelord_core::text_editor::resources::{
  ActiveIndentScope, IndentGuidesSettings,
};
use codelord_core::token::{Token, TokenExtractors};
use codelord_core::ui::component::Active;
use codelord_git::Blame;
use codelord_git::blame::relative_time;

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use rustc_hash::FxHashMap as HashMap;

use std::hash::{Hash, Hasher};

/// Gutter dimensions and layout configuration.
#[derive(Debug, Clone, Copy)]
struct GutterDimensions {
  left_padding: f32,
  line_number_width: f32,
  right_padding: f32,
  margin: f32,
}

impl GutterDimensions {
  fn calculate(total_lines: usize, char_width: f32) -> Self {
    let max_line_num = total_lines.max(1);

    let digit_count = if max_line_num == 0 {
      1
    } else {
      ((max_line_num as f32).log10().floor() as usize) + 1
    };

    let effective_digits = digit_count.max(2) + 1;
    let line_number_width = char_width * effective_digits as f32;
    let left_padding = char_width * 3.0;
    let right_padding = char_width * 3.0;
    let margin = char_width * 1.0;

    Self {
      left_padding,
      line_number_width,
      right_padding,
      margin,
    }
  }

  fn gutter_width(&self) -> f32 {
    self.left_padding + self.line_number_width + self.right_padding
  }

  fn full_width(&self) -> f32 {
    self.gutter_width() + self.margin
  }

  fn line_number_x(&self, gutter_left: f32) -> f32 {
    gutter_left + self.left_padding
  }

  /// X position where right padding (fold icons) starts.
  fn right_padding_x(&self, gutter_left: f32) -> f32 {
    gutter_left + self.left_padding + self.line_number_width
  }

  fn content_x(&self, gutter_left: f32) -> f32 {
    gutter_left + self.full_width()
  }
}

/// Data needed for rendering the editor.
struct EditorData<'a> {
  entity: Entity,
  buffer: &'a TextBuffer,
  cursor_pos: usize,
  selection: Option<(usize, usize)>,
  file_tab: Option<&'a FileTab>,
  blame: Option<&'a Blame>,
  blame_enabled: bool,
  /// Reference to TabSymbols for fold state.
  symbols: Option<&'a TabSymbols>,
}

impl<'a> EditorData<'a> {
  #[inline]
  fn line_count(&self) -> usize {
    self.buffer.len_lines()
  }

  #[inline]
  fn is_line_collapsed(&self, line: usize) -> bool {
    self
      .symbols
      .map(|s| s.folds.is_line_collapsed(line, &s.map.anchors))
      .unwrap_or(false)
  }

  #[inline]
  fn symbol_map(&self) -> &SymbolMap {
    self.symbols.map(|s| &s.map).unwrap_or(&EMPTY_SYMBOL_MAP)
  }

  #[inline]
  fn foldable_symbol_at(&self, line: usize) -> Option<(usize, &SymbolAnchor)> {
    self.symbols.and_then(|s| {
      s.map
        .anchors
        .iter()
        .enumerate()
        .find(|(_, sym)| sym.is_foldable && sym.line == line)
    })
  }

  #[inline]
  fn content_len(&self) -> usize {
    self.buffer.rope.len_bytes()
  }
}

// Empty symbol map for when there are no symbols.
static EMPTY_SYMBOL_MAP: std::sync::LazyLock<SymbolMap> =
  std::sync::LazyLock::new(SymbolMap::default);

/// Blame animation state for rendering.
struct BlameAnimState {
  opacity: f32,
  text: Option<String>,
  animating: bool,
}

enum EventToSpawn {
  Insert(String),
  DeleteBefore,
  DeleteAfter,
  Move(CursorMovement, bool),
  SetCursor(usize, bool),
  ToggleFold(usize),
}

/// Cached tokens per line - only tokenize what we need.
#[derive(Clone, Default)]
struct LineTokenCache {
  /// Tokens for each line, keyed by (line_index, line_hash).
  lines: HashMap<usize, (u64, Vec<Token>)>,
}

/// Cached colors per line - only extract what we need.
#[derive(Clone, Default)]
struct LineColorCache {
  /// Colors for each line, keyed by line_index with content hash.
  lines: HashMap<usize, (u64, Vec<ColorInfo>)>,
}

/// Generic editor content renderer.
/// M is the marker component that identifies which tabs to query.
pub fn show<M: Component>(
  ui: &mut egui::Ui,
  world: &mut World,
  scroll_id: &str,
) {
  // Query editor data.
  let mut query = world.query_filtered::<(
    Entity,
    &TextBuffer,
    &Cursor,
    Option<&FileTab>,
    Option<&TabBlame>,
    Option<&TabSymbols>,
  ), (With<M>, With<Active>)>();

  let query_result = query.iter(world).next();
  let Some((entity, buffer, cursor, file_tab, tab_blame, tab_symbols)) =
    query_result
  else {
    // Only show instructions for EditorTab, not PlaygroundTab.
    let is_editor =
      std::any::TypeId::of::<M>() == std::any::TypeId::of::<EditorTab>();
    show_empty_state(ui, world, is_editor);
    return;
  };

  let (blame, blame_enabled) = tab_blame
    .map(|b| (b.blame.as_ref(), b.enabled))
    .unwrap_or((None, false));

  let data = EditorData {
    entity,
    buffer,
    cursor_pos: cursor.position,
    selection: cursor.selection(),
    file_tab,
    blame,
    blame_enabled,
    symbols: tab_symbols,
  };

  // Get per-line token cache from egui memory.
  let token_cache_id = ui.id().with(("line_token_cache", entity));
  let mut line_cache: LineTokenCache = ui
    .memory(|mem| mem.data.get_temp(token_cache_id))
    .unwrap_or_default();

  // Get token extractors reference for per-line tokenization.
  let extractors = world.get_resource::<TokenExtractors>();
  let language = data.file_tab.map(|f| f.language()).unwrap_or_default();

  // Get color extractor for inline color previews.
  let color_extractor = world.get_resource::<ColorExtractor>();

  // Per-line color cache (similar to token cache).
  let color_cache_id = ui.id().with(("line_color_cache", entity));
  let mut line_color_cache: LineColorCache = ui
    .memory(|mem| mem.data.get_temp(color_cache_id))
    .unwrap_or_default();

  // Extract values we need before any mutable world operations.
  // These are all cheap (Copy or O(1)) extractions.
  let entity = data.entity;
  let (cursor_line, cursor_col) = calculate_cursor_position(&data);
  let visuals = ui.style().visuals.clone();
  let font_id = egui::TextStyle::Monospace.resolve(ui.style());
  let line_height = ui.fonts_mut(|f| f.row_height(&font_id));
  let char_width = ui.fonts_mut(|f| {
    f.layout_no_wrap("0".to_string(), font_id.clone(), egui::Color32::WHITE)
      .rect
      .width()
  });

  let show_blame = data.blame_enabled && data.blame.is_some();
  let line_count = data.line_count();
  let gutter = GutterDimensions::calculate(line_count, char_width);

  // Read indent guides settings.
  let indent_settings = world
    .get_resource::<IndentGuidesSettings>()
    .cloned()
    .unwrap_or_default();

  // Calculate active indent scope for cursor (used for highlighting).
  let active_indent_scope =
    if indent_settings.enabled && indent_settings.highlight_active_scope {
      find_active_indent_scope(
        data.buffer,
        cursor_line,
        indent_settings.indent_size,
      )
    } else {
      ActiveIndentScope::default()
    };

  // Read hovered span for playground highlighting (only for PlaygroundTab).
  let is_playground =
    std::any::TypeId::of::<M>() == std::any::TypeId::of::<PlaygroundTab>();

  let hovered_span: Option<(usize, usize)> = if is_playground {
    world
      .get_resource::<PlaygroundHoveredSpan>()
      .and_then(|r| r.span)
  } else {
    None
  };

  // Use total line count for height - we handle collapsed lines during render.
  // This avoids O(n) iteration over all lines every frame.
  let total_height = line_count as f32 * line_height;

  // Extract blame entry for current line (clone just the small amount we need).
  // This allows us to drop 'data' before mutable world operations.
  let blame_entry_for_line: Option<(String, i64)> = data
    .blame
    .and_then(|b| b.entry_for_line(cursor_line + 1))
    .map(|e| (e.author.clone(), e.timestamp));

  // Read blame animation state (immutable - doesn't conflict with data's
  // borrow).
  let blame_anim = read_blame_state(world, entity);

  let mut events_to_spawn: Vec<EventToSpawn> = Vec::new();

  let has_focus = world
    .get_resource::<KeyboardFocus>()
    .map(|f| f.has_focus(entity))
    .unwrap_or(false);

  let mut request_focus = false;
  let mut cursor_blink_active = false;
  let mut shimmer_active = false;

  let available_rect = ui.available_rect_before_wrap();

  // Check if mouse is hovering over the symbol track area (right edge)
  // to disable main scroll and let symbol track handle it.
  let symbol_track_rect = egui::Rect::from_min_size(
    egui::pos2(
      available_rect.right() - symbol_track::SYMBOL_TRACK_WIDTH - 8.0,
      available_rect.top(),
    ),
    egui::vec2(
      symbol_track::SYMBOL_TRACK_WIDTH + 8.0,
      available_rect.height(),
    ),
  );
  let mouse_pos = ui.input(|i| i.pointer.hover_pos());
  let is_hovering_symbol_track =
    mouse_pos.is_some_and(|pos| symbol_track_rect.contains(pos));

  // Check for pending scroll-to-line request from previous frame.
  let pending_scroll_id = ui.id().with("pending_scroll_to_line");
  let pending_scroll: Option<usize> = ui.memory_mut(|mem| {
    mem.data.get_temp::<usize>(pending_scroll_id).inspect(|_| {
      mem.data.remove::<usize>(pending_scroll_id);
    })
  });

  // Build scroll area with optional vertical offset.
  // Disable scrolling when hovering symbol track to let it handle scroll.
  let scroll_source = if is_hovering_symbol_track {
    egui::scroll_area::ScrollSource::NONE
  } else {
    egui::scroll_area::ScrollSource::default()
  };
  let mut scroll_area = egui::ScrollArea::both()
    .id_salt(scroll_id)
    .auto_shrink([false, false])
    .scroll_source(scroll_source);

  // Apply pending scroll offset if requested.
  if let Some(target_line) = pending_scroll {
    let target_offset = target_line as f32 * line_height;
    // Center the target line in viewport.
    let viewport_height = available_rect.height();
    let centered_offset = (target_offset - viewport_height / 2.0).max(0.0);
    scroll_area = scroll_area.vertical_scroll_offset(centered_offset);
  }

  let scroll_output = scroll_area.show(ui, |ui| {
    let content_width = ui.available_width();
    let available_height = ui.available_height();
    // Ensure clickable area covers at least the viewport (for empty files).
    let desired_size =
      egui::vec2(content_width, total_height.max(available_height));
    let (response, painter) =
      ui.allocate_painter(desired_size, egui::Sense::click_and_drag());
    let rect = response.rect;

    if response.clicked() {
      request_focus = true;
    }

    painter.rect_filled(rect, 0.0, visuals.extreme_bg_color);

    let gutter_rect = egui::Rect::from_min_size(
      rect.min,
      egui::vec2(gutter.gutter_width(), rect.height()),
    );
    painter.rect_filled(gutter_rect, 0.0, visuals.faint_bg_color);

    // Virtualized rendering: only render visible lines.
    let clip_rect = ui.clip_rect();
    let first_visible_row =
      ((clip_rect.min.y - rect.min.y) / line_height).floor() as i32;
    let last_visible_row =
      ((clip_rect.max.y - rect.min.y) / line_height).ceil() as i32;

    // Track visual row separately from source line index.
    let mut visual_row = 0i32;
    let mut fold_clicked: Option<usize> = None;

    // Track cumulative char count only when we have a selection.
    // Computed lazily as we iterate through visible lines.
    let has_selection = data.selection.is_some();
    let mut cumulative_chars = 0usize;

    for line_idx in 0..line_count {
      // Skip collapsed lines (lines inside a folded block).
      // Using lazy check per visible line instead of O(n) precomputation.
      if data.is_line_collapsed(line_idx) {
        continue;
      }

      // Early exit: past visible area.
      if visual_row > last_visible_row {
        break;
      }

      // Skip lines above visible area (but still track visual_row).
      if visual_row < first_visible_row - 1 {
        visual_row += 1;
        continue;
      }

      // Get line text from rope - small allocation per visible line only.
      let line_rope = data.buffer.line(line_idx);
      let line_string: String =
        line_rope.map(|l| l.to_string()).unwrap_or_default();
      let line_text = line_string.trim_end_matches(&['\n', '\r'][..]);

      let y = rect.min.y + visual_row as f32 * line_height;

      let line_num_color = if line_idx == cursor_line {
        egui::Color32::WHITE
      } else {
        egui::Color32::from_gray(100)
      };

      painter.text(
        egui::pos2(
          gutter.line_number_x(rect.min.x) + gutter.line_number_width,
          y,
        ),
        egui::Align2::RIGHT_TOP,
        format!("{}", line_idx + 1),
        font_id.clone(),
        line_num_color,
      );

      if line_idx == cursor_line {
        let content_x = gutter.content_x(rect.min.x);
        let line_rect = egui::Rect::from_min_size(
          egui::pos2(content_x, y),
          egui::vec2(rect.width() - gutter.full_width(), line_height),
        );
        painter.rect_stroke(
          line_rect,
          0.0,
          egui::Stroke::new(
            1.0_f32,
            visuals.widgets.noninteractive.bg_stroke.color,
          ),
          egui::StrokeKind::Outside,
        );
      }

      // Selection rendering uses cumulative char tracking.
      // Only compute for lines that might have selection.
      if has_selection {
        let (sel_start, sel_end) = data.selection.unwrap();
        let line_char_count = line_text.chars().count();
        let line_end_char = cumulative_chars + line_char_count;

        if sel_start < line_end_char && sel_end > cumulative_chars {
          let sel_col_start = sel_start.saturating_sub(cumulative_chars);
          let sel_col_end = (sel_end - cumulative_chars).min(line_char_count);

          let content_x = gutter.content_x(rect.min.x);
          let sel_x_start = content_x + sel_col_start as f32 * char_width;
          let sel_x_end = content_x + sel_col_end as f32 * char_width;

          let sel_rect = egui::Rect::from_min_max(
            egui::pos2(sel_x_start, y),
            egui::pos2(sel_x_end, y + line_height),
          );
          painter.rect_filled(sel_rect, 0.0, visuals.selection.bg_fill);
        }

        // Track cumulative chars for next line.
        cumulative_chars = line_end_char + 1; // +1 for newline
      }

      // Hovered span highlight (playground only).
      // Renders background for lexeme when hovering token row.
      if let Some((span_start, span_end)) = hovered_span {
        let line_byte_start = data.buffer.line_to_byte(line_idx);
        let line_byte_end = line_byte_start + line_text.len();

        // Check if span overlaps with this line.
        if span_start < line_byte_end && span_end > line_byte_start {
          // Calculate column offsets within this line (in bytes -> chars).
          let local_start = span_start.saturating_sub(line_byte_start);
          let local_end = (span_end - line_byte_start).min(line_text.len());

          // Convert byte offsets to character columns.
          let col_start = line_text[..local_start].chars().count();
          let col_end = line_text[..local_end].chars().count();

          let content_x = gutter.content_x(rect.min.x);
          let highlight_x_start = content_x + col_start as f32 * char_width;
          let highlight_x_end = content_x + col_end as f32 * char_width;

          let highlight_rect = egui::Rect::from_min_max(
            egui::pos2(highlight_x_start, y),
            egui::pos2(highlight_x_end, y + line_height),
          );

          // Yellow-ish highlight color for hovered token.
          let highlight_color =
            egui::Color32::from_rgba_unmultiplied(255, 200, 50, 60);
          painter.rect_filled(highlight_rect, 0.0, highlight_color);
        }
      }

      // Get or compute tokens for this line only.
      let mut line_hasher = std::collections::hash_map::DefaultHasher::new();
      line_text.hash(&mut line_hasher);
      let line_hash = line_hasher.finish();

      let line_tokens: &[Token] =
        if let Some((cached_hash, tokens)) = line_cache.lines.get(&line_idx) {
          if *cached_hash == line_hash {
            tokens.as_slice()
          } else {
            // Line changed - re-tokenize just this line.
            let tokens = extractors
              .map(|ext| ext.extract(language, line_text))
              .unwrap_or_default();
            line_cache.lines.insert(line_idx, (line_hash, tokens));
            &line_cache.lines.get(&line_idx).unwrap().1
          }
        } else {
          // No cache for this line - tokenize it.
          let tokens = extractors
            .map(|ext| ext.extract(language, line_text))
            .unwrap_or_default();
          line_cache.lines.insert(line_idx, (line_hash, tokens));
          &line_cache.lines.get(&line_idx).unwrap().1
        };

      // Render indent guide lines (before text so they appear behind).
      if indent_settings.enabled {
        // For empty lines, compute indent from adjacent non-empty lines.
        let is_empty =
          compute_indent_level(line_text, indent_settings.indent_size) < 0;
        let max_indent_for_empty = if is_empty {
          get_empty_line_indent(
            data.buffer,
            line_idx,
            indent_settings.indent_size,
          )
        } else {
          0
        };

        render_indent_guides(
          &painter,
          line_text,
          y,
          line_height,
          gutter.content_x(rect.min.x),
          char_width,
          &indent_settings,
          line_idx,
          &active_indent_scope,
          max_indent_for_empty,
        );
      }

      // Get or compute colors for this line (for inline color previews).
      let line_colors: &[ColorInfo] = if let Some((cached_hash, colors)) =
        line_color_cache.lines.get(&line_idx)
      {
        if *cached_hash == line_hash {
          colors.as_slice()
        } else {
          // Line changed - re-extract colors.
          let colors = color_extractor
            .map(|ext| ext.extract(line_text))
            .unwrap_or_default();
          line_color_cache.lines.insert(line_idx, (line_hash, colors));
          &line_color_cache.lines.get(&line_idx).unwrap().1
        }
      } else {
        // No cache for this line - extract colors.
        let colors = color_extractor
          .map(|ext| ext.extract(line_text))
          .unwrap_or_default();
        line_color_cache.lines.insert(line_idx, (line_hash, colors));
        &line_color_cache.lines.get(&line_idx).unwrap().1
      };

      // Check for hover over color values (for tooltip display).
      // We collect hovered color info for deferred tooltip rendering.
      // Use a fixed ID (not ui.id()) so it can be read outside the scroll area.
      let content_x = gutter.content_x(rect.min.x);
      let hovered_color_id = egui::Id::new(("editor_hovered_color", entity));
      if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
        for color in line_colors.iter() {
          let color_start_x = content_x + color.column as f32 * char_width;
          let color_end_x =
            color_start_x + color.text.chars().count() as f32 * char_width;
          let color_rect = egui::Rect::from_min_max(
            egui::pos2(color_start_x, y),
            egui::pos2(color_end_x, y + line_height),
          );

          if color_rect.contains(hover_pos) {
            // Store hovered color for tooltip rendering after scroll area.
            ui.memory_mut(|mem| {
              mem.data.insert_temp(
                hovered_color_id,
                (color.clone(), (color_start_x, y + line_height)),
              );
            });
          }
        }
      }

      // Render line with syntax highlighting (tokens are line-local, 0-based).
      let galley = build_highlighted_galley(
        ui,
        line_text,
        &font_id,
        line_tokens,
        0, // Line-local start
        line_text.len(),
        visuals.text_color(),
        line_idx,
      );

      let galley_width = galley.rect.width();
      painter.galley(
        egui::pos2(gutter.content_x(rect.min.x), y),
        galley,
        visuals.text_color(),
      );

      // Render inline blame at end of line (only on cursor line).
      if show_blame
        && line_idx == cursor_line
        && let Some(ref text) = blame_anim.text
      {
        // Apply opacity from animation (fade to 60% alpha for subtle look).
        let base_alpha = 150u8;
        let alpha = (blame_anim.opacity * base_alpha as f32) as u8;
        let blame_color =
          egui::Color32::from_rgba_unmultiplied(120, 120, 120, alpha);

        // Position: end of line text + padding, or min_column (60), whichever
        // is greater.
        let padding = char_width * 4.0;
        let min_column = 60.0 * char_width;
        let content_x = gutter.content_x(rect.min.x);
        let line_end_x = content_x + galley_width + padding;
        let min_x = content_x + min_column;
        let blame_x = line_end_x.max(min_x);

        painter.text(
          egui::pos2(blame_x, y),
          egui::Align2::LEFT_TOP,
          text,
          font_id.clone(),
          blame_color,
        );
      }

      // Render fold indicator if this line has a foldable symbol.
      if let Some((symbol_idx, symbol)) = data.foldable_symbol_at(line_idx) {
        // Check if next line is collapsed to determine fold state.
        let next_line_collapsed = if symbol.end_line > symbol.line {
          data.is_line_collapsed(symbol.line + 1)
        } else {
          false
        };

        // Render the fold icon (AngleRightLine, rotated 90° when expanded).
        let icon_size = egui::vec2(line_height * 0.6, line_height * 0.6);

        // Position in the right padding area of gutter (after line numbers).
        let right_padding_start = gutter.right_padding_x(rect.min.x);
        let icon_x =
          right_padding_start + (gutter.right_padding - icon_size.x) * 0.5;
        let icon_y = y + (line_height - icon_size.y) * 0.5;

        let icon_rect =
          egui::Rect::from_min_size(egui::pos2(icon_x, icon_y), icon_size);

        let mut icon_image = icon_to_image(&Icon::Arrow(Arrow::AngleRightLine));

        // Rotate down when expanded (90 degrees).
        if !next_line_collapsed {
          icon_image = icon_image
            .rotate(std::f32::consts::FRAC_PI_2, egui::Vec2::splat(0.5));
        }

        // Check for hover/click on fold icon.
        let click_rect = egui::Rect::from_min_size(
          egui::pos2(right_padding_start, y),
          egui::vec2(gutter.right_padding, line_height),
        );

        let fold_response = ui.interact(
          click_rect,
          ui.id().with(("fold_icon", symbol_idx)),
          egui::Sense::click(),
        );

        // Tint based on hover state.
        let tint_color = if fold_response.hovered() {
          egui::Color32::from_gray(220)
        } else {
          egui::Color32::from_gray(150)
        };

        icon_image.tint(tint_color).paint_at(ui, icon_rect);

        // Handle click.
        if fold_response.clicked() {
          fold_clicked = Some(symbol_idx);
        }

        // Hover highlight.
        if fold_response.hovered() {
          ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }
      }

      visual_row += 1;
    }

    if data.content_len() == 0 {
      shimmer_active = render_empty_buffer(
        ui,
        &painter,
        rect,
        &gutter,
        &font_id,
        line_height,
        &visuals,
      );
    }

    if has_focus || request_focus {
      cursor_blink_active = render_cursor(
        ui,
        &painter,
        rect,
        &gutter,
        cursor_line,
        cursor_col,
        char_width,
        line_height,
        &visuals,
      );
    }

    (rect, response, fold_clicked)
  });

  let (rect, response, fold_clicked) = scroll_output.inner;

  // Save updated line token cache.
  ui.memory_mut(|mem| {
    mem.data.insert_temp(token_cache_id, line_cache);
  });

  // Save updated line color cache.
  ui.memory_mut(|mem| {
    mem.data.insert_temp(color_cache_id, line_color_cache);
  });

  // Render symbol track on right edge (only if we have symbols).
  let mut scroll_to_line: Option<usize> = None;
  let symbol_map = data.symbol_map();
  if !symbol_map.anchors.is_empty() {
    // Pass the full available rect - symbol track calculates its own position.
    ui.scope_builder(egui::UiBuilder::new().max_rect(available_rect), |ui| {
      let result = symbol_track::render(
        ui,
        available_rect,
        scroll_output.state.offset.y,
        total_height,
        available_rect.height(),
        symbol_map,
        line_count,
      );

      if let Some(line) = result.clicked_line {
        scroll_to_line = Some(line);
      }
    });
  }

  // Render sticky scroll overlay at top of editor.
  let sticky_settings = world
    .get_resource::<StickyScrollSettings>()
    .map(|s| (s.enabled, s.max_lines))
    .unwrap_or((true, 5));

  if sticky_settings.0 && !symbol_map.anchors.is_empty() {
    // Calculate first visible line from scroll offset.
    let first_visible_line =
      (scroll_output.state.offset.y / line_height).floor() as usize;

    // Find sticky lines (symbols whose start is above viewport).
    let sticky_symbols =
      symbol_map.find_sticky_lines(first_visible_line, sticky_settings.1);

    if !sticky_symbols.is_empty() {
      let sticky_clicked = render_sticky_scroll(
        ui,
        &data,
        &sticky_symbols,
        available_rect,
        &gutter,
        &font_id,
        line_height,
        &visuals,
        extractors,
        language,
      );

      // Use same scroll handling as symbol track.
      if sticky_clicked.is_some() {
        scroll_to_line = sticky_clicked;
      }
    }
  }

  // Handle symbol track / sticky scroll click - scroll to line and set cursor.
  if let Some(target_line) = scroll_to_line {
    // Store scroll request for next frame.
    ui.memory_mut(|mem| {
      mem.data.insert_temp(pending_scroll_id, target_line);
    });

    // Calculate character position at start of target line using rope (O(log
    // n)).
    let char_pos = if target_line < data.buffer.len_lines() {
      data.buffer.rope.line_to_char(target_line)
    } else {
      data.buffer.len_chars()
    };
    events_to_spawn.push(EventToSpawn::SetCursor(char_pos, false));
  }

  // Handle fold icon click.
  if let Some(symbol_idx) = fold_clicked {
    events_to_spawn.push(EventToSpawn::ToggleFold(symbol_idx));
  }

  // Don't clear focus just because a click landed outside the editor
  // rect. Sibling focus-grabbing widgets (terminal, filescope, …) set
  // focus to themselves on click via `KeyboardFocus::set`, which
  // implicitly drops ours. Proactively spawning `ClearFocusRequest`
  // here races with those `set` calls — the request lands a frame
  // later and clobbers the new focus, so typing into the terminal
  // after clicking it would silently do nothing.

  if has_focus || request_focus {
    collect_keyboard_events(ui, &mut events_to_spawn);
  }

  if response.clicked() {
    handle_mouse_click(
      ui,
      &response,
      rect,
      &gutter,
      &data,
      line_height,
      char_width,
      &mut events_to_spawn,
    );
  }

  // === MUTABLE WORLD OPERATIONS START HERE ===
  // data's borrow ends here (last use was above).

  spawn_events(world, entity, events_to_spawn);

  // Update blame animation state (deferred from read phase).
  update_blame_animation(world, entity, cursor_line, blame_entry_for_line);

  // Render color tooltip if hovering over a color value.
  // Use the same fixed ID as inside the scroll area.
  let hovered_color_id = egui::Id::new(("editor_hovered_color", entity));
  let hovered_color: Option<(ColorInfo, (f32, f32))> =
    ui.memory(|mem| mem.data.get_temp(hovered_color_id));

  // Clear hover state each frame (it will be re-set if still hovering).
  ui.memory_mut(|mem| {
    mem.data.remove::<(ColorInfo, (f32, f32))>(hovered_color_id);
  });

  if let Some((color_info, tooltip_pos)) = hovered_color {
    // Render color tooltip and check for click.
    if let Some(clicked) =
      render_color_tooltip(ui, &color_info, tooltip_pos.0, tooltip_pos.1)
      && let Some(mut picker_state) =
        world.get_resource_mut::<ColorPickerState>()
    {
      picker_state.open(entity, &clicked, tooltip_pos);
    }
  }

  if request_focus {
    world.spawn(FocusRequest::new(entity));
  }

  if cursor_blink_active
    && let Some(mut anim) = world.get_resource_mut::<ContinuousAnimations>()
  {
    anim.set_cursor_blink_active();
  }

  if shimmer_active
    && let Some(mut anim) = world.get_resource_mut::<ContinuousAnimations>()
  {
    anim.set_shimmer_active();
  }

  if blame_anim.animating
    && let Some(mut anim) = world.get_resource_mut::<ContinuousAnimations>()
  {
    anim.set_blame_active();
  }
}

fn show_empty_state(
  ui: &mut egui::Ui,
  world: &mut World,
  show_instructions: bool,
) {
  use egui::emath::GuiRounding as _;

  let available_rect = ui.available_rect_before_wrap();

  if !show_instructions {
    // Make full area clickable for double-click to create new tab.
    let response = ui.allocate_rect(available_rect, egui::Sense::click());
    if response.double_clicked() {
      world.spawn(codelord_core::events::NewEditorTabRequest);
    }
    return;
  }

  const INSTRUCTIONS_WIDTH: f32 = 400.0;
  const INSTRUCTIONS_HEIGHT: f32 = 360.0;

  let centered_rect = egui::Rect::from_center_size(
    available_rect.center(),
    egui::vec2(INSTRUCTIONS_WIDTH, INSTRUCTIONS_HEIGHT),
  )
  .round_ui();

  // Background: clickable area for double-click to create new tab.
  let response = ui.interact(
    available_rect,
    ui.id().with("empty_state_bg"),
    egui::Sense::click(),
  );
  if response.double_clicked() {
    world.spawn(codelord_core::events::NewEditorTabRequest);
  }

  ui.scope_builder(egui::UiBuilder::new().max_rect(centered_rect), |ui| {
    ui.set_width(INSTRUCTIONS_WIDTH);
    instructions::show(ui, world);
  });
}

fn calculate_cursor_position(data: &EditorData) -> (usize, usize) {
  // Use TextBuffer's O(log n) method instead of O(n) iteration.
  data.buffer.char_to_line_col(data.cursor_pos)
}

fn render_empty_buffer(
  ui: &egui::Ui,
  painter: &egui::Painter,
  rect: egui::Rect,
  gutter: &GutterDimensions,
  font_id: &egui::FontId,
  line_height: f32,
  visuals: &egui::Visuals,
) -> bool {
  let y = rect.min.y;

  painter.text(
    egui::pos2(
      gutter.line_number_x(rect.min.x) + gutter.line_number_width,
      y,
    ),
    egui::Align2::RIGHT_TOP,
    "1",
    font_id.clone(),
    egui::Color32::WHITE,
  );

  let content_x = gutter.content_x(rect.min.x);
  let line_rect = egui::Rect::from_min_size(
    egui::pos2(content_x, y),
    egui::vec2(rect.width() - gutter.full_width(), line_height),
  );
  painter.rect_stroke(
    line_rect,
    0.0,
    egui::Stroke::new(1.0_f32, visuals.widgets.noninteractive.bg_stroke.color),
    egui::StrokeKind::Outside,
  );

  let hint_text = "Start typing to dismiss.";
  let hint_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 120);
  let hint_pos = egui::pos2(content_x, y);
  let time = ui.input(|i| i.time) as f32;
  let hint_font = egui::FontId::monospace(line_height * 0.8);

  let hint_galley = painter.layout_no_wrap(
    hint_text.to_string(),
    hint_font.clone(),
    hint_color,
  );

  let hint_width = hint_galley.rect.width();

  let shimmer = ShimmerAnimation::with_timing(1.0, 50.0).with_intensity(0.6);
  let (shimmer_center, _) =
    shimmer.calculate_position_with_pause(time, hint_width, 1.0);

  let mut x_offset = 0.0;
  for ch in hint_text.chars() {
    let char_galley =
      painter.layout_no_wrap(ch.to_string(), hint_font.clone(), hint_color);

    let cw = char_galley.rect.width();
    let char_center_x = x_offset + cw * 0.5;

    let intensity = shimmer.calculate_intensity(char_center_x, shimmer_center);

    let final_color = egui::Color32::from_rgba_unmultiplied(
      (hint_color.r() as f32 * (1.0 - intensity) + 255.0 * intensity) as u8,
      (hint_color.g() as f32 * (1.0 - intensity) + 255.0 * intensity) as u8,
      (hint_color.b() as f32 * (1.0 - intensity) + 255.0 * intensity) as u8,
      hint_color.a(),
    );

    painter.galley_with_override_text_color(
      hint_pos + egui::vec2(x_offset, 0.0),
      char_galley,
      final_color,
    );

    x_offset += cw;
  }

  true
}

#[allow(clippy::too_many_arguments)]
fn render_cursor(
  ui: &egui::Ui,
  painter: &egui::Painter,
  rect: egui::Rect,
  gutter: &GutterDimensions,
  cursor_line: usize,
  cursor_col: usize,
  char_width: f32,
  line_height: f32,
  visuals: &egui::Visuals,
) -> bool {
  let blink = (ui.input(|i| i.time) * 2.0).floor() as i32 % 2 == 0;

  if blink {
    let cursor_x =
      gutter.content_x(rect.min.x) + cursor_col as f32 * char_width;

    let cursor_y = rect.min.y + cursor_line as f32 * line_height;

    painter.rect_filled(
      egui::Rect::from_min_size(
        egui::pos2(cursor_x, cursor_y),
        egui::vec2(2.0, line_height),
      ),
      0.0,
      visuals.text_color(),
    );
  }

  true
}

fn collect_keyboard_events(ui: &egui::Ui, events: &mut Vec<EventToSpawn>) {
  if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
    events.push(EventToSpawn::Insert("  ".into()));
  }

  ui.input(|input| {
    let shift = input.modifiers.shift;
    let ctrl = input.modifiers.command;

    for event in &input.events {
      match event {
        egui::Event::Text(text) if !ctrl => {
          events.push(EventToSpawn::Insert(text.clone()));
        }
        egui::Event::Key {
          key, pressed: true, ..
        } => match key {
          egui::Key::Backspace => events.push(EventToSpawn::DeleteBefore),
          egui::Key::Delete => events.push(EventToSpawn::DeleteAfter),
          egui::Key::Enter => events.push(EventToSpawn::Insert("\n".into())),
          egui::Key::Tab => {}
          egui::Key::ArrowLeft => {
            let movement = if ctrl {
              CursorMovement::WordLeft
            } else {
              CursorMovement::Left
            };
            events.push(EventToSpawn::Move(movement, shift));
          }
          egui::Key::ArrowRight => {
            let movement = if ctrl {
              CursorMovement::WordRight
            } else {
              CursorMovement::Right
            };
            events.push(EventToSpawn::Move(movement, shift));
          }
          egui::Key::ArrowUp => {
            events.push(EventToSpawn::Move(CursorMovement::Up, shift))
          }
          egui::Key::ArrowDown => {
            events.push(EventToSpawn::Move(CursorMovement::Down, shift))
          }
          egui::Key::Home => {
            let movement = if ctrl {
              CursorMovement::BufferStart
            } else {
              CursorMovement::LineStart
            };
            events.push(EventToSpawn::Move(movement, shift));
          }
          egui::Key::End => {
            let movement = if ctrl {
              CursorMovement::BufferEnd
            } else {
              CursorMovement::LineEnd
            };
            events.push(EventToSpawn::Move(movement, shift));
          }
          _ => {}
        },
        _ => {}
      }
    }
  });
}

#[allow(clippy::too_many_arguments)]
fn handle_mouse_click(
  ui: &egui::Ui,
  response: &egui::Response,
  rect: egui::Rect,
  gutter: &GutterDimensions,
  data: &EditorData,
  line_height: f32,
  char_width: f32,
  events: &mut Vec<EventToSpawn>,
) {
  let Some(pos) = response.interact_pointer_pos() else {
    return;
  };

  let local_pos = pos - rect.min.to_vec2();

  let clicked_line = ((local_pos.y / line_height) as usize)
    .min(data.line_count().saturating_sub(1));

  let clicked_col =
    ((local_pos.x - gutter.full_width()).max(0.0) / char_width) as usize;

  // Use TextBuffer's O(log n) method instead of O(n) iteration.
  let char_idx = data.buffer.line_col_to_char(clicked_line, clicked_col);

  let shift = ui.input(|i| i.modifiers.shift);

  events.push(EventToSpawn::SetCursor(char_idx, shift));
}

fn spawn_events(world: &mut World, entity: Entity, events: Vec<EventToSpawn>) {
  for event in events {
    match event {
      EventToSpawn::Insert(text) => {
        world.spawn(InsertTextEvent::new(entity, text));
      }
      EventToSpawn::DeleteBefore => {
        world.spawn(DeleteTextEvent::backspace(entity));
      }
      EventToSpawn::DeleteAfter => {
        world.spawn(DeleteTextEvent::delete(entity));
      }
      EventToSpawn::Move(movement, extend) => {
        world.spawn(MoveCursorEvent::new(entity, movement, extend));
      }
      EventToSpawn::SetCursor(pos, extend) => {
        world.spawn(SetCursorEvent::new(entity, pos, extend));
      }
      EventToSpawn::ToggleFold(symbol_idx) => {
        world.spawn(ToggleFoldRequest::new(entity, symbol_idx));
      }
    }
  }
}

/// Cached galley entry.
#[derive(Clone)]
struct CachedGalley {
  galley: std::sync::Arc<egui::Galley>,
  hash: u64,
}

/// Build a galley with syntax-highlighted text, with caching.
#[allow(clippy::too_many_arguments)]
fn build_highlighted_galley(
  ui: &egui::Ui,
  line_text: &str,
  font_id: &egui::FontId,
  tokens: &[Token],
  line_start: usize,
  line_end: usize,
  default_color: egui::Color32,
  line_idx: usize,
) -> std::sync::Arc<egui::Galley> {
  // Compute hash of line content AND tokens for cache key.
  // Including tokens.len() ensures cache invalidates when tokenization
  // completes.
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  line_text.hash(&mut hasher);
  line_start.hash(&mut hasher);
  tokens.len().hash(&mut hasher);
  let content_hash = hasher.finish();

  // Check cache.
  let cache_id = ui.id().with(("galley_cache", line_idx));
  let cached: Option<CachedGalley> =
    ui.memory(|mem| mem.data.get_temp(cache_id));

  if let Some(cached) = cached
    && cached.hash == content_hash
  {
    return cached.galley;
  }

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

    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.memory_mut(|mem| {
      mem.data.insert_temp(
        cache_id,
        CachedGalley {
          galley: galley.clone(),
          hash: content_hash,
        },
      );
    });
    return galley;
  }

  // Find tokens that overlap with this line using binary search.
  // Tokens are sorted by start position, so we can find the first
  // potential match and iterate only until we pass line_end.
  let first_token_idx = tokens.partition_point(|t| t.end <= line_start);

  // Collect only overlapping tokens (no allocation if we iterate directly).
  let mut line_tokens_iter = tokens[first_token_idx..]
    .iter()
    .take_while(|t| t.start < line_end)
    .filter(|t| t.end > line_start)
    .peekable();

  if line_tokens_iter.peek().is_none() {
    // No tokens - render entire line with default color.
    job.append(
      line_text,
      0.0,
      TextFormat {
        font_id: font_id.clone(),
        color: default_color,
        ..Default::default()
      },
    );

    return ui.fonts_mut(|f| f.layout_job(job));
  }

  // Build sections with token colors.
  let mut pos = 0;

  for token in line_tokens_iter {
    // Token bounds relative to line.
    let tok_start = token.start.saturating_sub(line_start);
    let tok_end = (token.end - line_start).min(line_text.len());

    // Skip tokens that are fully covered by previous tokens.
    if tok_end <= pos {
      continue;
    }

    // Adjust start to not re-render already covered text.
    let effective_start = tok_start.max(pos);

    // Add any gap before this token with default color.
    if effective_start > pos {
      let gap_text = &line_text[pos..effective_start];

      job.append(
        gap_text,
        0.0,
        TextFormat {
          font_id: font_id.clone(),
          color: default_color,
          ..Default::default()
        },
      );
    }

    // Add token text with syntax color.
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

  // Add any remaining text after last token.
  if pos < line_text.len() {
    let remaining = &line_text[pos..];

    job.append(
      remaining,
      0.0,
      TextFormat {
        font_id: font_id.clone(),
        color: default_color,
        ..Default::default()
      },
    );
  }

  let galley = ui.fonts_mut(|f| f.layout_job(job));

  // Cache the galley.
  ui.memory_mut(|mem| {
    mem.data.insert_temp(
      cache_id,
      CachedGalley {
        galley: galley.clone(),
        hash: content_hash,
      },
    );
  });

  galley
}

/// Format blame text for gutter display.
fn format_blame_text(author: &str, timestamp: i64) -> String {
  // Truncate author name if too long
  let author_display = if author.len() > 12 {
    format!("{}…", &author[..11])
  } else {
    author.to_string()
  };

  let time = relative_time(timestamp);
  format!("{author_display} · {time}")
}

/// Read current blame animation state (immutable - no world mutation).
/// This can be called while holding references from world queries.
fn read_blame_state(world: &World, entity: Entity) -> BlameAnimState {
  let Some(tab_blame) = world.get::<TabBlame>(entity) else {
    return BlameAnimState {
      opacity: 1.0,
      text: None,
      animating: false,
    };
  };

  BlameAnimState {
    opacity: tab_blame.opacity(),
    text: tab_blame.animated_text(),
    animating: tab_blame.is_animating(),
  }
}

/// Update blame animation state (mutable - requires exclusive world access).
/// Call this AFTER all immutable borrows from world are dropped.
fn update_blame_animation(
  world: &mut World,
  entity: Entity,
  cursor_line: usize,
  blame_entry: Option<(String, i64)>,
) {
  // Get delta time for animation update.
  let dt = world
    .get_resource::<DeltaTime>()
    .map(|t| t.delta())
    .unwrap_or(1.0 / 60.0);

  // Get mutable TabBlame to update animation.
  let Some(mut tab_blame) = world.get_mut::<TabBlame>(entity) else {
    return;
  };

  // Check if cursor moved to a new line.
  let needs_new_animation = tab_blame.animated_line != Some(cursor_line);

  if needs_new_animation {
    // Start new animation if we have blame data for this line.
    if let Some((author, timestamp)) = blame_entry {
      let blame_text = format_blame_text(&author, timestamp);
      tab_blame.start_line_animation(cursor_line, &blame_text);
    }
  }

  // Update animation tick.
  tab_blame.update_animation(dt);
}

/// Render sticky scroll overlay at the top of the editor.
///
/// Shows lines from containing scopes that are scrolled past,
/// giving context about which function/class/block you're inside.
/// Returns clicked line if user clicked a sticky item.
#[allow(clippy::too_many_arguments)]
fn render_sticky_scroll(
  ui: &mut egui::Ui,
  data: &EditorData,
  sticky_symbols: &[&SymbolAnchor],
  available_rect: egui::Rect,
  gutter: &GutterDimensions,
  font_id: &egui::FontId,
  line_height: f32,
  visuals: &egui::Visuals,
  extractors: Option<&TokenExtractors>,
  language: codelord_core::language::Language,
) -> Option<usize> {
  let mut clicked_line: Option<usize> = None;
  let sticky_count = sticky_symbols.len();
  let sticky_height = sticky_count as f32 * line_height;

  // Create overlay rect at top of editor.
  let sticky_rect = egui::Rect::from_min_size(
    available_rect.min,
    egui::vec2(available_rect.width(), sticky_height),
  );

  // Paint background with slight transparency.
  let bg_color = visuals.extreme_bg_color.gamma_multiply(0.95);
  let painter = ui.painter_at(sticky_rect);
  painter.rect_filled(sticky_rect, 0.0, bg_color);

  // Paint gutter background.
  let gutter_rect = egui::Rect::from_min_size(
    sticky_rect.min,
    egui::vec2(gutter.gutter_width(), sticky_height),
  );
  painter.rect_filled(gutter_rect, 0.0, visuals.faint_bg_color);

  // Render each sticky line.
  for (idx, symbol) in sticky_symbols.iter().enumerate() {
    let y = sticky_rect.min.y + idx as f32 * line_height;
    let line_idx = symbol.line;

    // Get line text from buffer.
    let line_rope = data.buffer.line(line_idx);
    let line_string: String =
      line_rope.map(|l| l.to_string()).unwrap_or_default();
    let line_text = line_string.trim_end_matches(&['\n', '\r'][..]);

    // Render line number.
    let line_num_color = egui::Color32::from_gray(100);
    painter.text(
      egui::pos2(
        gutter.line_number_x(sticky_rect.min.x) + gutter.line_number_width,
        y,
      ),
      egui::Align2::RIGHT_TOP,
      format!("{}", line_idx + 1),
      font_id.clone(),
      line_num_color,
    );

    // Tokenize line for syntax highlighting.
    let tokens = extractors
      .map(|ext| ext.extract(language, line_text))
      .unwrap_or_default();

    // Build and render galley with syntax colors.
    let galley = build_sticky_galley(
      ui,
      line_text,
      font_id,
      &tokens,
      visuals.text_color(),
    );

    painter.galley(
      egui::pos2(gutter.content_x(sticky_rect.min.x), y),
      galley,
      visuals.text_color(),
    );

    // Handle click to jump to line.
    let line_rect = egui::Rect::from_min_size(
      egui::pos2(sticky_rect.min.x, y),
      egui::vec2(sticky_rect.width(), line_height),
    );

    let response = ui.interact(
      line_rect,
      ui.id().with(("sticky_line", line_idx)),
      egui::Sense::click(),
    );

    if response.clicked() {
      clicked_line = Some(symbol.line);
    }

    if response.hovered() {
      // Highlight on hover.
      let hover_color = visuals.selection.bg_fill.linear_multiply(0.2);
      painter.rect_filled(line_rect, 0.0, hover_color);
      ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
    }
  }

  // Draw bottom separator line (same style as divider component).
  let separator_color = visuals.widgets.noninteractive.bg_stroke.color;
  painter.line_segment(
    [
      egui::pos2(sticky_rect.min.x, sticky_rect.max.y - 0.5),
      egui::pos2(sticky_rect.max.x, sticky_rect.max.y - 0.5),
    ],
    egui::Stroke::new(0.5_f32, separator_color),
  );

  clicked_line
}

/// Build a simple syntax-highlighted galley for sticky scroll lines.
fn build_sticky_galley(
  ui: &egui::Ui,
  line_text: &str,
  font_id: &egui::FontId,
  tokens: &[Token],
  default_color: egui::Color32,
) -> std::sync::Arc<egui::Galley> {
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
    return ui.fonts_mut(|f| f.layout_job(job));
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
    return ui.fonts_mut(|f| f.layout_job(job));
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

  // Remaining text.
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

  ui.fonts_mut(|f| f.layout_job(job))
}

/// Compute the indent level (in spaces) for a line.
///
/// Returns -1 for empty/whitespace-only lines.
/// Returns the actual visual column position of first non-whitespace character.
fn compute_indent_level(line: &str, tab_size: usize) -> i32 {
  let mut indent = 0usize;

  for ch in line.chars() {
    match ch {
      ' ' => indent += 1,
      '\t' => {
        // Tab aligns to next tab stop
        indent = indent - (indent % tab_size) + tab_size;
      }
      '\n' | '\r' => return -1,  // Empty line
      _ => return indent as i32, // First non-whitespace
    }
  }

  -1 // Line is only whitespace
}

/// Find the active indent scope for the cursor position.
///
/// Searches bidirectionally from cursor_line to find the boundaries
/// of the current indent scope.
fn find_active_indent_scope(
  buffer: &TextBuffer,
  cursor_line: usize,
  indent_size: usize,
) -> ActiveIndentScope {
  let line_count = buffer.len_lines();

  if line_count == 0 {
    return ActiveIndentScope::default();
  }

  // Get cursor line's indent level
  let cursor_indent = buffer
    .line(cursor_line)
    .map(|l| compute_indent_level(&l.to_string(), indent_size))
    .unwrap_or(-1);

  // If cursor is on empty line, look at adjacent lines
  let base_indent = if cursor_indent < 0 {
    // Look up for a non-empty line
    let mut found = -1i32;
    for i in (0..cursor_line).rev() {
      if let Some(line) = buffer.line(i) {
        let indent = compute_indent_level(&line.to_string(), indent_size);
        if indent >= 0 {
          found = indent;
          break;
        }
      }
    }
    found
  } else {
    cursor_indent
  };

  if base_indent < 0 {
    return ActiveIndentScope::default();
  }

  let base_level = (base_indent as usize) / indent_size;

  // Search upward for scope start
  let mut start_line = cursor_line;
  for i in (0..cursor_line).rev() {
    if let Some(line) = buffer.line(i) {
      let indent = compute_indent_level(&line.to_string(), indent_size);
      // -1 means empty line, continue through it
      if indent >= 0 {
        let level = (indent as usize) / indent_size;
        if level < base_level {
          break; // Found scope boundary
        }
        start_line = i;
      }
    }
  }

  // Search downward for scope end
  let mut end_line = cursor_line;
  for i in (cursor_line + 1)..line_count {
    if let Some(line) = buffer.line(i) {
      let indent = compute_indent_level(&line.to_string(), indent_size);
      // -1 means empty line, continue through it
      if indent >= 0 {
        let level = (indent as usize) / indent_size;
        if level < base_level {
          break; // Found scope boundary
        }
        end_line = i;
      }
    }
  }

  ActiveIndentScope {
    start_line,
    end_line,
    indent_level: base_level,
  }
}

/// Compute indent level for an empty line by looking at adjacent non-empty
/// lines. Returns the minimum indent of surrounding non-empty lines.
fn get_empty_line_indent(
  buffer: &TextBuffer,
  line_idx: usize,
  indent_size: usize,
) -> usize {
  let line_count = buffer.len_lines();

  // Look backward for previous non-empty line.
  let mut prev_indent: Option<usize> = None;
  for i in (0..line_idx).rev() {
    if let Some(line) = buffer.line(i) {
      let indent = compute_indent_level(&line.to_string(), indent_size);
      if indent >= 0 {
        prev_indent = Some((indent as usize) / indent_size);
        break;
      }
    }
  }

  // Look forward for next non-empty line.
  let mut next_indent: Option<usize> = None;
  for i in (line_idx + 1)..line_count {
    if let Some(line) = buffer.line(i) {
      let indent = compute_indent_level(&line.to_string(), indent_size);
      if indent >= 0 {
        next_indent = Some((indent as usize) / indent_size);
        break;
      }
    }
  }

  // Use minimum of surrounding indents (to continue guides through scope).
  match (prev_indent, next_indent) {
    (Some(p), Some(n)) => p.min(n),
    (Some(p), None) => p,
    (None, Some(n)) => n,
    (None, None) => 0,
  }
}

/// Render indent guide lines for a visible line.
#[allow(clippy::too_many_arguments)]
fn render_indent_guides(
  painter: &egui::Painter,
  line_text: &str,
  y: f32,
  line_height: f32,
  content_x: f32,
  char_width: f32,
  settings: &IndentGuidesSettings,
  line_idx: usize,
  active_scope: &ActiveIndentScope,
  max_indent_for_empty: usize,
) {
  let indent_spaces = compute_indent_level(line_text, settings.indent_size);

  // For empty/whitespace lines, continue guides from context
  let indent_levels = if indent_spaces < 0 {
    // Use the max indent level passed in (from adjacent lines)
    max_indent_for_empty
  } else {
    (indent_spaces as usize) / settings.indent_size
  };

  if indent_levels == 0 {
    return;
  }

  // Colors for guides
  let inactive_color = egui::Color32::from_gray(30);
  let active_color = egui::Color32::from_gray(60);

  // Check if this line is within the active scope
  let in_active_scope =
    line_idx >= active_scope.start_line && line_idx <= active_scope.end_line;

  // Draw vertical line for each indent level
  for level in 1..=indent_levels {
    // Column position: (level - 1) * indent_size gives the column
    let column = (level - 1) * settings.indent_size;
    let x = content_x + column as f32 * char_width;

    // Determine if this guide is active
    let is_active = settings.highlight_active_scope
      && in_active_scope
      && level == active_scope.indent_level;

    let (color, width) = if is_active {
      (active_color, 0.5_f32)
    } else {
      (inactive_color, 0.5_f32)
    };

    // Draw vertical line
    painter.line_segment(
      [egui::pos2(x, y), egui::pos2(x, y + line_height)],
      egui::Stroke::new(width, color),
    );
  }
}

/// Renders a floating color tooltip below the hovered color text.
///
/// Shows a color square with the value text. Click opens the color picker.
/// Returns the ColorInfo if the tooltip was clicked.
fn render_color_tooltip(
  ui: &mut egui::Ui,
  color: &ColorInfo,
  x: f32,
  y: f32,
) -> Option<ColorInfo> {
  let mut clicked = None;

  // Tooltip dimensions.
  let square_size = 24.0;
  let padding = 8.0;
  let gap = 8.0;

  // Measure text width.
  let font_id = egui::TextStyle::Monospace.resolve(ui.style());
  let text_galley = ui.fonts_mut(|f| {
    f.layout_no_wrap(color.text.clone(), font_id.clone(), egui::Color32::WHITE)
  });
  let text_width = text_galley.rect.width();
  let text_height = text_galley.rect.height();

  // Calculate tooltip size.
  let tooltip_width = padding + square_size + gap + text_width + padding;
  let tooltip_height = padding + square_size.max(text_height) + padding;

  // Position tooltip below the color text.
  let tooltip_rect = egui::Rect::from_min_size(
    egui::pos2(x, y + 4.0),
    egui::vec2(tooltip_width, tooltip_height),
  );

  // Draw tooltip background.
  let painter = ui.painter();
  let bg_color = ui.style().visuals.window_fill;
  let border_color = ui.style().visuals.window_stroke.color;

  painter.rect(
    tooltip_rect,
    4.0,
    bg_color,
    egui::Stroke::new(1.0_f32, border_color),
    egui::StrokeKind::Inside,
  );

  // Draw color square.
  let square_rect = egui::Rect::from_min_size(
    egui::pos2(
      tooltip_rect.min.x + padding,
      tooltip_rect.min.y + (tooltip_height - square_size) * 0.5,
    ),
    egui::vec2(square_size, square_size),
  );

  // Draw checkerboard for alpha preview.
  if color.rgba[3] < 255 {
    draw_mini_checkerboard(painter, square_rect, 4.0);
  }

  let fill_color = egui::Color32::from_rgba_unmultiplied(
    color.rgba[0],
    color.rgba[1],
    color.rgba[2],
    color.rgba[3],
  );
  painter.rect_filled(square_rect, 3.0, fill_color);
  painter.rect_stroke(
    square_rect,
    3.0,
    egui::Stroke::new(1.0_f32, egui::Color32::from_gray(80)),
    egui::StrokeKind::Inside,
  );

  // Draw color text.
  let text_pos = egui::pos2(
    square_rect.max.x + gap,
    tooltip_rect.min.y + (tooltip_height - text_height) * 0.5,
  );
  painter.galley(text_pos, text_galley, egui::Color32::WHITE);

  // Handle click on tooltip.
  let response = ui.interact(
    tooltip_rect,
    ui.id().with("color_tooltip"),
    egui::Sense::click(),
  );

  if response.clicked() {
    clicked = Some(color.clone());
  }

  if response.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  clicked
}

/// Draws a tiny checkerboard pattern for alpha preview.
fn draw_mini_checkerboard(
  painter: &egui::Painter,
  rect: egui::Rect,
  cell_size: f32,
) {
  let light = egui::Color32::from_gray(200);
  let dark = egui::Color32::from_gray(120);

  let cols = (rect.width() / cell_size).ceil() as i32;
  let rows = (rect.height() / cell_size).ceil() as i32;

  for row in 0..rows {
    for col in 0..cols {
      let color = if (row + col) % 2 == 0 { light } else { dark };
      let cell_rect = egui::Rect::from_min_size(
        egui::pos2(
          rect.min.x + col as f32 * cell_size,
          rect.min.y + row as f32 * cell_size,
        ),
        egui::vec2(cell_size, cell_size),
      )
      .intersect(rect);

      painter.rect_filled(cell_rect, 0.0, color);
    }
  }
}
