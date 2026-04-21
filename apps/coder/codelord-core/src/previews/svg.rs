//! SVG preview state resource.
//!
//! Displays rendered SVG when user opens .svg files.

use bevy_ecs::prelude::Resource;

use std::path::{Path, PathBuf};

/// Resource for tracking SVG preview state.
#[derive(Resource, Default)]
pub struct SvgPreviewState {
  /// Whether SVG preview is currently enabled.
  pub enabled: bool,
  /// The file path currently being previewed.
  pub current_file: Option<PathBuf>,
  /// Raw SVG bytes for rendering.
  pub svg_data: Option<Vec<u8>>,
  /// File name for display.
  pub file_name: String,
  /// Error message if SVG failed to load.
  pub error: Option<String>,
  /// Generation counter for detecting changes.
  pub generation: u64,
  /// Zoom level (1.0 = 100%).
  pub zoom: f32,
}

impl SvgPreviewState {
  /// Opens an SVG file for preview.
  pub fn open(&mut self, path: &Path) {
    self.enabled = true;
    self.current_file = Some(path.to_path_buf());

    self.file_name = path
      .file_name()
      .and_then(|s| s.to_str())
      .unwrap_or("Unknown")
      .to_string();

    self.zoom = 1.0;

    match std::fs::read(path) {
      Ok(data) => {
        self.svg_data = Some(data);
        self.error = None;
      }
      Err(e) => {
        self.svg_data = None;
        self.error = Some(e.to_string());
      }
    }

    self.generation += 1;
  }

  /// Closes the SVG preview.
  pub fn close(&mut self) {
    self.enabled = false;
    self.current_file = None;
    self.svg_data = None;
    self.file_name.clear();
    self.error = None;
  }

  /// Zooms in by 25%.
  pub fn zoom_in(&mut self) {
    self.zoom = (self.zoom * 1.25).min(8.0);
    self.generation += 1;
  }

  /// Zooms out by 25%.
  pub fn zoom_out(&mut self) {
    self.zoom = (self.zoom / 1.25).max(0.125);
    self.generation += 1;
  }

  /// Resets zoom to 100%.
  pub fn zoom_reset(&mut self) {
    self.zoom = 1.0;
    self.generation += 1;
  }
}

/// Returns true if `path` has an SVG extension.
pub fn accepts(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| ext.eq_ignore_ascii_case("svg"))
    .unwrap_or(false)
}
