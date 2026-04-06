//! CSV preview state resource.
//!
//! Like markdown preview, CSV preview renders inline replacing the code editor
//! with a formatted table view.

use super::csv::CsvData;

use bevy_ecs::prelude::Resource;

use std::path::PathBuf;

/// Resource for tracking CSV preview state.
///
/// CSV preview replaces the code editor content with a table view.
#[derive(Resource, Default)]
pub struct CsvPreviewState {
  /// Whether CSV preview is currently enabled.
  pub enabled: bool,
  /// The file path currently being previewed.
  pub current_file: Option<PathBuf>,
  /// Cached parsed CSV data for rendering.
  pub cached_data: Option<CsvData>,
  /// Generation counter for detecting content changes.
  pub generation: u64,
}

impl CsvPreviewState {
  /// Toggles CSV preview for a file.
  ///
  /// Returns true if preview was enabled, false if disabled.
  pub fn toggle(&mut self, file: PathBuf, content: &str) -> bool {
    if self.enabled && self.current_file.as_ref() == Some(&file) {
      // Disable preview
      self.enabled = false;
      self.current_file = None;
      self.cached_data = None;
      false
    } else {
      // Enable preview - parse CSV content
      self.enabled = true;
      self.current_file = Some(file);
      self.cached_data = Some(super::csv::parse_csv(content));
      self.generation += 1;
      true
    }
  }

  /// Updates the cached data when file changes.
  pub fn update_content(&mut self, file: &PathBuf, content: &str) {
    if self.enabled && self.current_file.as_ref() == Some(file) {
      self.cached_data = Some(super::csv::parse_csv(content));
      self.generation += 1;
    }
  }

  /// Checks if preview is active for a specific file.
  pub fn is_active_for(&self, file: &PathBuf) -> bool {
    self.enabled && self.current_file.as_ref() == Some(file)
  }

  /// Closes the preview.
  pub fn close(&mut self) {
    self.enabled = false;
    self.current_file = None;
    self.cached_data = None;
  }

  /// Gets the cached CSV data if available.
  pub fn get_data(&self) -> Option<&CsvData> {
    self.cached_data.as_ref()
  }
}
