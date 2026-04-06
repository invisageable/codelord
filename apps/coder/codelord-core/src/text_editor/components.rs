use crate::language::Language;

use bevy_ecs::component::Component;
use ropey::Rope;

use std::path::PathBuf;

/// The actual file content for editing using rope data structure.
///
/// Uses ropey for O(log n) edits and efficient line access.
/// Syntax tokens are cached per-line in the renderer (egui memory),
/// not stored here.
#[derive(Component, Debug, Clone)]
pub struct TextBuffer {
  pub rope: Rope,
  pub modified: bool,
}

impl TextBuffer {
  pub fn new(content: impl AsRef<str>) -> Self {
    let rope = Rope::from_str(content.as_ref());
    Self {
      rope,
      modified: false,
    }
  }

  pub fn empty() -> Self {
    Self {
      rope: Rope::new(),
      modified: false,
    }
  }

  /// Insert text at character index.
  pub fn insert(&mut self, char_idx: usize, text: &str) {
    let idx = char_idx.min(self.rope.len_chars());
    self.rope.insert(idx, text);
    self.modified = true;
  }

  /// Insert a single character at character index.
  pub fn insert_char(&mut self, char_idx: usize, ch: char) {
    let idx = char_idx.min(self.rope.len_chars());
    self.rope.insert_char(idx, ch);
    self.modified = true;
  }

  /// Delete character at index (backspace behavior).
  pub fn delete_char_before(&mut self, char_idx: usize) {
    if char_idx > 0 && char_idx <= self.rope.len_chars() {
      self.rope.remove(char_idx - 1..char_idx);
      self.modified = true;
    }
  }

  /// Delete character after index (delete key behavior).
  pub fn delete_char_after(&mut self, char_idx: usize) {
    if char_idx < self.rope.len_chars() {
      self.rope.remove(char_idx..char_idx + 1);
      self.modified = true;
    }
  }

  /// Delete a range of characters.
  pub fn delete_range(&mut self, start: usize, end: usize) {
    let start = start.min(self.rope.len_chars());
    let end = end.min(self.rope.len_chars());
    if start < end {
      self.rope.remove(start..end);
      self.modified = true;
    }
  }

  /// Get total character count.
  pub fn len_chars(&self) -> usize {
    self.rope.len_chars()
  }

  /// Get total line count.
  pub fn len_lines(&self) -> usize {
    self.rope.len_lines()
  }

  /// Get line at index.
  pub fn line(&self, line_idx: usize) -> Option<ropey::RopeSlice<'_>> {
    if line_idx < self.rope.len_lines() {
      Some(self.rope.line(line_idx))
    } else {
      None
    }
  }

  /// Convert char index to (line, col).
  pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
    let idx = char_idx.min(self.rope.len_chars());
    let line = self.rope.char_to_line(idx);
    let line_start = self.rope.line_to_char(line);
    let col = idx - line_start;
    (line, col)
  }

  /// Convert line index to byte offset.
  pub fn line_to_byte(&self, line_idx: usize) -> usize {
    if line_idx >= self.rope.len_lines() {
      return self.rope.len_bytes();
    }
    self.rope.line_to_byte(line_idx)
  }

  /// Convert (line, col) to char index.
  pub fn line_col_to_char(&self, line: usize, col: usize) -> usize {
    if line >= self.rope.len_lines() {
      return self.rope.len_chars();
    }
    let line_start = self.rope.line_to_char(line);
    let line_len = self.rope.line(line).len_chars();
    // Subtract 1 for newline char if not last line
    let max_col = if line < self.rope.len_lines() - 1 {
      line_len.saturating_sub(1)
    } else {
      line_len
    };
    line_start + col.min(max_col)
  }
}

impl std::fmt::Display for TextBuffer {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.rope)
  }
}

/// Cursor position in the text buffer.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Cursor {
  /// Character index position.
  pub position: usize,
  /// Selection anchor (if selecting). None means no selection.
  pub anchor: Option<usize>,
}

impl Cursor {
  pub fn new(position: usize) -> Self {
    Self {
      position,
      anchor: None,
    }
  }

  /// Get selection range (start, end) if selecting.
  pub fn selection(&self) -> Option<(usize, usize)> {
    self.anchor.map(|anchor| {
      if anchor < self.position {
        (anchor, self.position)
      } else {
        (self.position, anchor)
      }
    })
  }

  /// Check if there's an active selection.
  pub fn has_selection(&self) -> bool {
    self.anchor.is_some() && self.anchor != Some(self.position)
  }

  /// Start selection at current position.
  pub fn start_selection(&mut self) {
    self.anchor = Some(self.position);
  }

  /// Clear selection.
  pub fn clear_selection(&mut self) {
    self.anchor = None;
  }

  /// Move cursor, optionally extending selection.
  pub fn move_to(&mut self, position: usize, extend_selection: bool) {
    if extend_selection && self.anchor.is_none() {
      self.anchor = Some(self.position);
    } else if !extend_selection {
      self.anchor = None;
    }
    self.position = position;
  }
}

/// Editor-specific tab data - associates a tab with a file path.
#[derive(Component, Debug, Clone)]
pub struct FileTab {
  pub path: PathBuf,
}

impl FileTab {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into() }
  }

  /// Returns true if this file is an HTML file.
  pub fn is_html(&self) -> bool {
    self.path.extension().is_some_and(|ext| {
      let ext = ext.to_string_lossy().to_lowercase();
      ext == "html" || ext == "htm"
    })
  }

  /// Returns true if this file is a Markdown file.
  pub fn is_markdown(&self) -> bool {
    self.path.extension().is_some_and(|ext| {
      let ext = ext.to_string_lossy().to_lowercase();
      ext == "md" || ext == "markdown"
    })
  }

  /// Returns true if this file is a CSV file.
  pub fn is_csv(&self) -> bool {
    self.path.extension().is_some_and(|ext| {
      let ext = ext.to_string_lossy().to_lowercase();
      ext == "csv" || ext == "tsv"
    })
  }

  /// Returns true if this file is a PDF file.
  pub fn is_pdf(&self) -> bool {
    self.path.extension().is_some_and(|ext| {
      let ext = ext.to_string_lossy().to_lowercase();
      ext == "pdf"
    })
  }

  /// Returns true if this file is a SQLite database file.
  pub fn is_sqlite(&self) -> bool {
    self.path.extension().is_some_and(|ext| {
      let ext = ext.to_string_lossy().to_lowercase();
      matches!(ext.as_str(), "sqlite" | "sqlite3" | "db")
    })
  }

  /// Returns true if this file is an Excel/spreadsheet file.
  pub fn is_xls(&self) -> bool {
    self.path.extension().is_some_and(|ext| {
      let ext = ext.to_string_lossy().to_lowercase();
      matches!(ext.as_str(), "xls" | "xlsx" | "xlsm" | "xlsb" | "ods")
    })
  }

  /// Get the language type for this file based on extension.
  pub fn language(&self) -> Language {
    // First try the standard extension
    if let Some(ext) = self.path.extension().and_then(|e| e.to_str()) {
      return Language::from(ext);
    }

    // Handle dotfiles like .env, .gitignore, etc.
    if let Some(name) = self.path.file_name().and_then(|n| n.to_str())
      && let Some(stripped) = name.strip_prefix('.')
    {
      return Language::from(stripped);
    }

    Language::default()
  }
}

/// Marker component for the currently focused editor.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct EditorFocused;
