//! Symbol types for code navigation and breadcrumbs.
//!
//! Pure data types - no tree-sitter dependency. Extraction happens
//! in codelord-language, extractors registered via SymbolExtractors resource.

pub mod resources;
pub mod systems;

/// Insert sticky-scroll settings. `SymbolExtractors` registration is
/// handled by the caller (it needs language-specific extract fns from
/// `codelord-language`, which `codelord-core` does not depend on).
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(StickyScrollSettings::default());
}

/// Register symbol-extraction system.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems(systems::extract_symbols_system);
}

use bevy_ecs::component::Component;
use bevy_ecs::resource::Resource;

use std::ops::Range;

/// Settings for sticky scroll feature.
#[derive(Resource, Debug, Clone)]
pub struct StickyScrollSettings {
  /// Whether sticky scroll is enabled.
  pub enabled: bool,
  /// Maximum number of sticky lines to show.
  pub max_lines: usize,
}

impl Default for StickyScrollSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      max_lines: 5,
    }
  }
}

/// Type of code symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
  Function,
  Struct,
  Enum,
  Trait,
  Impl,
  Import,
  Const,
  Module,
}

impl SymbolKind {
  /// Get the keyword string for this symbol kind (for breadcrumb display).
  pub const fn keyword(&self) -> &'static str {
    match self {
      SymbolKind::Function => "fn",
      SymbolKind::Struct => "struct",
      SymbolKind::Enum => "enum",
      SymbolKind::Trait => "trait",
      SymbolKind::Impl => "impl",
      SymbolKind::Import => "use",
      SymbolKind::Const => "const",
      SymbolKind::Module => "mod",
    }
  }
}

/// Visual status of a symbol (controls color intensity/overlay).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymbolStatus {
  #[default]
  Default,
  Error,
  Warning,
  Modified,
}

/// A single symbol anchor point in a source file.
#[derive(Debug, Clone)]
pub struct SymbolAnchor {
  /// Type of symbol (determines base color).
  pub kind: SymbolKind,
  /// Line number where symbol is defined (start line, 0-indexed).
  pub line: usize,
  /// Column number where symbol starts (0-indexed).
  pub col: usize,
  /// Display name (for tooltips).
  pub name: String,
  /// Byte range in document (for click-to-jump).
  pub byte_range: Range<usize>,
  /// Visual status (determines color/overlay).
  pub status: SymbolStatus,
  /// End line of the symbol (for code folding).
  pub end_line: usize,
  /// Whether this symbol can be folded (multi-line blocks only).
  pub is_foldable: bool,
  /// Whether to include the end line in the fold (true for Python, false for
  /// Rust/JS). For curly-brace languages, the end line is `}` and should be
  /// shown. For indentation-based languages like Python, the end line is
  /// content and should be hidden.
  pub fold_end_inclusive: bool,
  /// Full display text for breadcrumb (e.g., "impl Coder", "fn main").
  pub display_text: String,
  /// Syntax highlight spans within display_text.
  /// Format: (byte_range_in_display_text, token_type_as_u8).
  pub highlight_ranges: Vec<(Range<usize>, u8)>,
}

/// Complete symbol map for a file.
#[derive(Debug, Clone, Default)]
pub struct SymbolMap {
  /// All symbols in the file, sorted by line number.
  pub anchors: Vec<SymbolAnchor>,
  /// Generation this map was built from (for cache invalidation).
  pub generation: u64,
}

impl SymbolMap {
  /// Create empty symbol map.
  pub fn new(generation: u64) -> Self {
    Self {
      anchors: Vec::new(),
      generation,
    }
  }

  /// Get symbols in a line range (for viewport filtering if needed).
  pub fn symbols_in_range(
    &self,
    start_line: usize,
    end_line: usize,
  ) -> Vec<&SymbolAnchor> {
    self
      .anchors
      .iter()
      .filter(|anchor| anchor.line >= start_line && anchor.line <= end_line)
      .collect()
  }

  /// Sort symbols by line number (should be called after building).
  pub fn sort(&mut self) {
    self.anchors.sort_by_key(|a| a.line);
  }

  /// Add a symbol anchor.
  pub fn add(&mut self, anchor: SymbolAnchor) {
    self.anchors.push(anchor);
  }

  /// Find all symbols whose range contains the given line.
  /// Returns symbols from outermost to innermost scope.
  pub fn find_containing(&self, cursor_line: usize) -> Vec<&SymbolAnchor> {
    let mut containing: Vec<&SymbolAnchor> = self
      .anchors
      .iter()
      .filter(|a| cursor_line >= a.line && cursor_line <= a.end_line)
      .collect();

    // Sort by span size (largest first = outermost first)
    containing.sort_by_key(|a| std::cmp::Reverse(a.end_line - a.line));

    containing
  }

  /// Find symbols for sticky scroll display.
  ///
  /// Returns symbols whose:
  /// - Start line is ABOVE the first visible line (header scrolled past)
  /// - End line is BELOW the first visible line (scope still active)
  /// - Limited to `max_lines` symbols
  ///
  /// Results are ordered from outermost to innermost scope.
  pub fn find_sticky_lines(
    &self,
    first_visible_line: usize,
    max_lines: usize,
  ) -> Vec<&SymbolAnchor> {
    // Find symbols that contain the first visible line
    // but whose start line is above the viewport.
    let mut sticky: Vec<&SymbolAnchor> = self
      .anchors
      .iter()
      .filter(|a| {
        a.is_foldable
          && a.line < first_visible_line
          && a.end_line >= first_visible_line
      })
      .collect();

    // Sort by span size (largest first = outermost scope first)
    sticky.sort_by_key(|a| std::cmp::Reverse(a.end_line - a.line));

    // Limit to max_lines
    sticky.truncate(max_lines);

    sticky
  }
}

/// Fold state for code folding - a thin view over SymbolMap.
///
/// This structure stores which symbols are collapsed, using indices
/// into the SymbolMap.anchors array. Follows Data-Oriented Design
/// by not duplicating symbol data.
#[derive(Debug, Clone, Default)]
pub struct FoldState {
  /// Indices of collapsed symbols (sorted for binary search).
  collapsed: Vec<usize>,
  /// Generation when this state was last updated.
  pub generation: u64,
}

impl FoldState {
  /// Create new empty fold state.
  pub fn new() -> Self {
    Self {
      collapsed: Vec::new(),
      generation: 0,
    }
  }

  /// Toggle fold state for a symbol at given index.
  pub fn toggle(&mut self, symbol_index: usize) {
    if let Some(pos) = self.collapsed.iter().position(|&i| i == symbol_index) {
      self.collapsed.swap_remove(pos);
      self.collapsed.sort_unstable();
    } else {
      self.collapsed.push(symbol_index);
      self.collapsed.sort_unstable();
    }
  }

  /// Check if a symbol at given index is collapsed.
  pub fn is_collapsed(&self, symbol_index: usize) -> bool {
    self.collapsed.binary_search(&symbol_index).is_ok()
  }

  /// Check if a line is inside any collapsed symbol.
  pub fn is_line_collapsed(
    &self,
    line: usize,
    symbols: &[SymbolAnchor],
  ) -> bool {
    for &idx in &self.collapsed {
      if let Some(symbol) = symbols.get(idx) {
        let in_fold = if symbol.fold_end_inclusive {
          // Python-style: hide all lines including end_line
          line > symbol.line && line <= symbol.end_line
        } else {
          // Rust/JS-style: show end_line (the closing brace)
          line > symbol.line && line < symbol.end_line
        };

        if in_fold {
          return true;
        }
      }
    }

    false
  }

  /// Get the collapsed symbol containing a given line (if any).
  pub fn get_collapsed_block_containing<'a>(
    &self,
    line: usize,
    symbols: &'a [SymbolAnchor],
  ) -> Option<&'a SymbolAnchor> {
    for &idx in &self.collapsed {
      if let Some(symbol) = symbols.get(idx) {
        let in_fold = if symbol.fold_end_inclusive {
          line > symbol.line && line <= symbol.end_line
        } else {
          // For curly-brace languages, include end_line in this check
          // so clicking on `}` still shows the fold indicator
          line > symbol.line && line <= symbol.end_line
        };

        if in_fold {
          return Some(symbol);
        }
      }
    }

    None
  }

  /// Find symbol index at a given line (for fold icon clicks).
  pub fn find_symbol_at_line(
    &self,
    line: usize,
    symbols: &[SymbolAnchor],
  ) -> Option<usize> {
    symbols
      .iter()
      .enumerate()
      .find(|(_, s)| s.line == line && s.is_foldable)
      .map(|(idx, _)| idx)
  }

  /// Get count of collapsed lines.
  pub fn collapsed_line_count(&self, symbols: &[SymbolAnchor]) -> usize {
    self
      .collapsed
      .iter()
      .filter_map(|&idx| symbols.get(idx))
      .map(|s| s.end_line.saturating_sub(s.line).saturating_sub(1))
      .sum()
  }

  /// Clear all collapsed state.
  pub fn clear(&mut self) {
    self.collapsed.clear();
  }
}

/// Symbol extraction state for a text buffer.
/// Stored as component on tab entity alongside TextBuffer.
#[derive(Component, Debug, Clone, Default)]
pub struct TabSymbols {
  /// The extracted symbol map (sorted by line number).
  pub map: SymbolMap,
  /// Fold state for code folding.
  pub folds: FoldState,
  /// Flag indicating symbols need re-extraction.
  pub dirty: bool,
}

impl TabSymbols {
  /// Create new empty tab symbols.
  pub fn new() -> Self {
    Self {
      map: SymbolMap::default(),
      folds: FoldState::new(),
      dirty: true, // Start dirty to trigger initial extraction
    }
  }

  /// Mark symbols as needing re-extraction.
  pub fn mark_dirty(&mut self) {
    self.dirty = true;
  }

  /// Update the symbol map and clear dirty flag.
  pub fn update(&mut self, map: SymbolMap) {
    self.map = map;
    self.dirty = false;
  }
}
