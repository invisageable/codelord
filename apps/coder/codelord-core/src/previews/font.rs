//! Font preview state resource.
//!
//! Displays font atlas when user opens font files (ttf, otf, woff2).

use bevy_ecs::prelude::Resource;

use std::path::{Path, PathBuf};

/// Resource for tracking font preview state.
#[derive(Resource, Default)]
pub struct FontPreviewState {
  /// Whether font preview is currently enabled.
  pub enabled: bool,
  /// The file path currently being previewed.
  pub current_file: Option<PathBuf>,
  /// Cached font bytes for rendering.
  pub font_data: Option<Vec<u8>>,
  /// Font family name for display.
  pub font_name: String,
  /// Error message if font failed to load.
  pub error: Option<String>,
  /// Generation counter for detecting changes.
  pub generation: u64,
}

impl FontPreviewState {
  /// Opens a font file for preview.
  pub fn open(&mut self, path: &Path) {
    self.enabled = true;
    self.current_file = Some(path.to_path_buf());
    self.font_name = path
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or("Unknown")
      .to_string();

    // Load font bytes
    match std::fs::read(path) {
      Ok(data) => {
        self.font_data = Some(data);
        self.error = None;
      }
      Err(e) => {
        self.font_data = None;
        self.error = Some(e.to_string());
      }
    }

    self.generation += 1;
  }

  /// Closes the font preview.
  pub fn close(&mut self) {
    self.enabled = false;
    self.current_file = None;
    self.font_data = None;
    self.font_name.clear();
    self.error = None;
  }

  /// Checks if preview is active for a specific file.
  pub fn is_active_for(&self, file: &Path) -> bool {
    self.enabled && self.current_file.as_deref() == Some(file)
  }
}
