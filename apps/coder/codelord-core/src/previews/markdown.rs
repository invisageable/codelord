//! Markdown preview state resource.
//!
//! Unlike HTML preview (which uses WebView in the right panel), markdown
//! preview renders inline, replacing the code editor with rendered content.

use bevy_ecs::prelude::Resource;

use std::path::PathBuf;

/// Resource for tracking markdown preview state.
///
/// Markdown preview replaces the code editor content with rendered markdown,
/// rather than displaying in a separate panel like HTML preview.
#[derive(Resource, Default)]
pub struct MarkdownPreviewState {
  /// Whether markdown preview is currently enabled.
  pub enabled: bool,
  /// The file path currently being previewed.
  pub current_file: Option<PathBuf>,
  /// Cached content for rendering (avoids re-reading file every frame).
  pub cached_content: Option<String>,
  /// Generation counter for detecting content changes.
  pub generation: u64,
}

impl MarkdownPreviewState {
  /// Toggles markdown preview for a file.
  ///
  /// Returns true if preview was enabled, false if disabled.
  pub fn toggle(&mut self, file: PathBuf, content: String) -> bool {
    if self.enabled && self.current_file.as_ref() == Some(&file) {
      // Disable preview
      self.enabled = false;
      self.current_file = None;
      self.cached_content = None;
      false
    } else {
      // Enable preview
      self.enabled = true;
      self.current_file = Some(file);
      self.cached_content = Some(content);
      self.generation += 1;
      true
    }
  }

  /// Updates the cached content when file changes.
  pub fn update_content(&mut self, file: &PathBuf, content: String) {
    if self.enabled && self.current_file.as_ref() == Some(file) {
      self.cached_content = Some(content);
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
    self.cached_content = None;
  }

  /// Gets the cached content if available.
  pub fn get_content(&self) -> Option<&str> {
    self.cached_content.as_deref()
  }
}
