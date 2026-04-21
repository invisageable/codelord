//! XLS preview state resource.
//!
//! XLS preview renders Excel files inline replacing the code editor
//! with a formatted table view with sheet navigation.

use bevy_ecs::prelude::{Component, Resource};

use std::path::PathBuf;

use swisskit::renderer::xls::XlsData;

/// Resource for tracking XLS preview state.
///
/// XLS preview replaces the code editor content with a table view.
#[derive(Resource)]
pub struct XlsPreviewState {
  /// Whether XLS preview is currently enabled.
  pub enabled: bool,
  /// The file path currently being previewed.
  pub current_file: Option<PathBuf>,
  /// Cached parsed XLS data for rendering.
  pub cached_data: Option<XlsData>,
  /// Current page for pagination (0-indexed).
  pub current_page: usize,
  /// Rows per page for pagination.
  pub rows_per_page: usize,
}

impl Default for XlsPreviewState {
  fn default() -> Self {
    Self {
      enabled: false,
      current_file: None,
      cached_data: None,
      current_page: 0,
      rows_per_page: Self::DEFAULT_ROWS_PER_PAGE,
    }
  }
}

impl XlsPreviewState {
  /// Default rows per page.
  pub const DEFAULT_ROWS_PER_PAGE: usize = 100;

  /// Opens XLS preview for a file.
  pub fn open(&mut self, file: PathBuf) {
    let data = swisskit::renderer::xls::parse_xls_file(&file);

    if data.has_error() {
      self.enabled = false;
      self.current_file = None;
      self.cached_data = Some(data);
      return;
    }

    self.enabled = true;
    self.current_file = Some(file);
    self.cached_data = Some(data);
    self.current_page = 0;
  }

  /// Switches to a different sheet.
  pub fn select_sheet(&mut self, sheet_index: usize) {
    let Some(file) = self.current_file.clone() else {
      return;
    };

    let data = swisskit::renderer::xls::parse_sheet_at(&file, sheet_index);
    self.cached_data = Some(data);
    self.current_page = 0;
  }

  /// Closes the preview.
  pub fn close(&mut self) {
    self.enabled = false;
    self.current_file = None;
    self.cached_data = None;
    self.current_page = 0;
  }

  /// Returns total number of pages based on row count.
  pub fn total_pages(&self) -> usize {
    self
      .cached_data
      .as_ref()
      .map(|data| {
        let total = data.rows.len();
        if total == 0 {
          1
        } else {
          total.div_ceil(self.rows_per_page)
        }
      })
      .unwrap_or(1)
  }

  /// Changes to a specific page.
  pub fn go_to_page(&mut self, page: usize) {
    let max_page = self.total_pages().saturating_sub(1);
    self.current_page = page.min(max_page);
  }
}

/// Request to change the selected sheet.
#[derive(Component)]
pub struct SelectSheetRequest(pub usize);

/// Request to change the current page.
#[derive(Component)]
pub struct ChangeXlsPageRequest(pub usize);

/// Returns true if `path` has a spreadsheet extension handled by the XLS
/// preview (xls/xlsx/xlsm/xlsb/ods).
pub fn accepts(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| {
      matches!(
        ext.to_lowercase().as_str(),
        "xls" | "xlsx" | "xlsm" | "xlsb" | "ods"
      )
    })
    .unwrap_or(false)
}
