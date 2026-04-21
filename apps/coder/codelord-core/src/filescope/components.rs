use bevy_ecs::component::Component;

use std::ops::Range;
use std::path::PathBuf;

/// Unique identifier for filescope instances.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FilescopeId(pub usize);

impl FilescopeId {
  pub fn new() -> Self {
    static COUNTER: std::sync::atomic::AtomicUsize =
      std::sync::atomic::AtomicUsize::new(0);
    Self(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
  }
}

impl Default for FilescopeId {
  fn default() -> Self {
    Self::new()
  }
}

/// Filescope item for display in the list.
#[derive(Debug, Clone)]
pub struct FilescopeItem {
  /// File path.
  pub path: PathBuf,
  /// Display name (filename only).
  pub name: String,
  /// Parent directory for context.
  pub parent: String,
  /// Whether this is a directory.
  pub is_dir: bool,
}

impl FilescopeItem {
  pub fn new(path: PathBuf) -> Self {
    Self::new_with_root(path, None)
  }

  pub fn new_with_root(path: PathBuf, root: Option<&PathBuf>) -> Self {
    let name = path
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_default();

    // Strip root's parent from path, keeping the project folder name.
    // e.g., /Users/foo/projects/zo/src/main.rs -> zo/src
    let parent = path
      .parent()
      .map(|p| {
        if let Some(root) = root {
          // Strip the root's parent to keep project name.
          if let Some(root_parent) = root.parent() {
            p.strip_prefix(root_parent)
              .map(|rel| rel.to_string_lossy().to_string())
              .unwrap_or_else(|_| p.to_string_lossy().to_string())
          } else {
            p.to_string_lossy().to_string()
          }
        } else {
          p.to_string_lossy().to_string()
        }
      })
      .unwrap_or_default();

    let is_dir = path.is_dir();

    Self {
      path,
      name,
      parent,
      is_dir,
    }
  }

  /// Returns display text for fuzzy matching (relative to root if provided).
  pub fn display_text(&self) -> String {
    // Use name + parent for matching (parent is already relative).
    if self.parent.is_empty() {
      self.name.clone()
    } else {
      format!("{}/{}", self.parent, self.name)
    }
  }
}

/// How to handle file selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectAction {
  /// Replace current view.
  #[default]
  Replace,
  /// Open in horizontal split.
  HSplit,
  /// Open in vertical split.
  VSplit,
  /// Open in new tab.
  NewTab,
}

/// Line range for preview highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
  pub start: usize,
  pub end: usize,
}

impl LineRange {
  pub fn single(line: usize) -> Self {
    Self {
      start: line,
      end: line,
    }
  }

  pub fn new(start: usize, end: usize) -> Self {
    Self { start, end }
  }
}

/// Cached preview content.
#[derive(Debug, Clone)]
pub enum CachedPreview {
  /// Text file content.
  Document {
    content: String,
    language: Option<String>,
  },
  /// Directory listing.
  Directory(Vec<DirEntry>),
  /// Binary file placeholder.
  Binary,
  /// File too large (> 10MB).
  LargeFile,
  /// File not found.
  NotFound,
  /// Loading in progress.
  Loading,
}

/// Directory entry for preview.
#[derive(Debug, Clone)]
pub struct DirEntry {
  pub name: String,
  pub is_dir: bool,
}

/// Multi-column query parser result.
#[derive(Debug, Clone, Default)]
pub struct PickerQuery {
  /// Primary search term.
  pub primary: String,
  /// Column-specific patterns.
  pub columns: rustc_hash::FxHashMap<String, String>,
  /// Byte ranges for active column detection.
  pub ranges: Vec<(Range<usize>, Option<String>)>,
}

impl PickerQuery {
  /// Parse query string into column patterns.
  /// Supports: "term", "term %column:value", "%col1:a %col2:b"
  pub fn parse(input: &str, column_names: &[String]) -> Self {
    let mut query = PickerQuery::default();
    let mut current_column: Option<&str> = None;
    let mut current_value = String::new();
    let mut range_start = 0;

    for (idx, token) in input.split_whitespace().enumerate() {
      let byte_start = if idx == 0 {
        0
      } else {
        input.find(token).unwrap_or(range_start)
      };

      if let Some(col_spec) = token.strip_prefix('%') {
        // Save previous column value.
        if let Some(col) = current_column {
          query
            .columns
            .insert(col.to_string(), current_value.trim().to_string());
          query
            .ranges
            .push((range_start..byte_start, Some(col.to_string())));

          current_value.clear();
        } else if !current_value.is_empty() {
          query.primary = current_value.trim().to_string();

          query.ranges.push((range_start..byte_start, None));
          current_value.clear();
        }

        // Start new column.
        current_column = column_names
          .iter()
          .find(|c| c.starts_with(col_spec))
          .map(|s| s.as_str());

        range_start = byte_start;
      } else {
        current_value.push_str(token);
        current_value.push(' ');
      }
    }

    // Save final value.
    if let Some(col) = current_column {
      query
        .columns
        .insert(col.to_string(), current_value.trim().to_string());
      query
        .ranges
        .push((range_start..input.len(), Some(col.to_string())));
    } else {
      query.primary = current_value.trim().to_string();

      if !query.primary.is_empty() {
        query.ranges.push((range_start..input.len(), None));
      }
    }

    query
  }

  /// Get active column based on cursor position.
  pub fn active_column(&self, cursor: usize) -> Option<&str> {
    self
      .ranges
      .iter()
      .find(|(range, _)| range.contains(&cursor))
      .and_then(|(_, col)| col.as_deref())
  }
}

/// Column definition for multi-column display.
#[derive(Debug, Clone)]
pub struct Column {
  /// Column name (for multi-column queries).
  pub name: String,
  /// Include in fuzzy matching.
  pub filterable: bool,
  /// Column width constraint.
  pub width: ColumnWidth,
}

/// Column width constraint.
#[derive(Debug, Default, Clone, Copy)]
pub enum ColumnWidth {
  Fixed(f32),
  Percent(f32),
  #[default]
  Fill,
}
