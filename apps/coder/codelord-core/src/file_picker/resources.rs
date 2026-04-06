use crate::animation::components::Animatable;
use crate::file_picker::components::{
  CachedPreview, DirEntry, FilePickerItem, PickerQuery, SelectAction,
};

use bevy_ecs::message::Message;
use bevy_ecs::resource::Resource;
use eazy::interpolation::Interpolation;
use eazy::{Curve, Easing};
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};
use rustc_hash::FxHashMap;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// File picker mode determines what items are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilePickerMode {
  /// Show all files in the workspace.
  #[default]
  Files,
  /// Show recently opened files.
  Recent,
  /// Show symbols in the current file.
  Symbols,
  /// Show symbols across the workspace.
  WorkspaceSymbols,
  /// Show buffers/open tabs.
  Buffers,
  /// Show commands.
  Commands,
}

/// Row padding animation state.
#[derive(Clone)]
pub struct RowPaddingAnim {
  /// Current padding value.
  pub current: f32,
  /// Target padding value.
  pub target: f32,
  /// Animation progress (0.0 to 1.0).
  pub progress: f32,
  /// Whether animating in (selected) or out (deselected).
  pub is_entering: bool,
}

impl RowPaddingAnim {
  pub const MIN_PADDING: f32 = 8.0;
  pub const MAX_PADDING: f32 = 16.0;
  pub const DURATION: f32 = 0.3;

  pub fn new(selected: bool) -> Self {
    let padding = if selected {
      Self::MAX_PADDING
    } else {
      Self::MIN_PADDING
    };

    Self {
      current: padding,
      target: padding,
      progress: 1.0,
      is_entering: selected,
    }
  }

  /// Set target and start animation.
  pub fn set_selected(&mut self, selected: bool) {
    let new_target = if selected {
      Self::MAX_PADDING
    } else {
      Self::MIN_PADDING
    };

    if (self.target - new_target).abs() > 0.01 {
      self.target = new_target;
      self.progress = 0.0;
      self.is_entering = selected;
    }
  }

  /// Update animation, returns true if still animating.
  pub fn update(&mut self, dt: f32) -> bool {
    if self.progress >= 1.0 {
      return false;
    }

    self.progress = (self.progress + dt / Self::DURATION).min(1.0);

    // Apply easing: InOutElastic for entering, OutElastic for exiting.
    let eased = if self.is_entering {
      Easing::InOutSine.y(self.progress)
    } else {
      Easing::OutSine.y(self.progress)
    };

    // Interpolate padding.
    let start = if self.is_entering {
      Self::MIN_PADDING
    } else {
      Self::MAX_PADDING
    };

    self.current = start + (self.target - start) * eased;

    self.progress < 1.0
  }

  /// Get current padding value.
  pub fn value(&self) -> f32 {
    self.current
  }
}

/// Main file picker state resource.
#[derive(Resource)]
pub struct FilePickerState {
  /// Whether the picker is visible.
  pub visible: bool,
  /// Current picker mode.
  pub mode: FilePickerMode,
  /// Search input string.
  pub search_input: String,
  /// Parsed query.
  pub query: PickerQuery,
  /// Current selection index.
  pub selection: usize,
  /// Scroll offset for virtualization.
  pub scroll_offset: usize,
  /// Toggle preview panel visibility.
  pub show_preview: bool,
  /// Preview cache.
  pub preview_cache: FxHashMap<PathBuf, CachedPreview>,
  /// Version counter for canceling stale async jobs.
  pub version: Arc<AtomicUsize>,
  /// Animation start time for open animation.
  pub animation_start: Option<f64>,
  /// Root paths to search.
  pub root_paths: Vec<PathBuf>,
  /// Column names for query parsing.
  pub column_names: Vec<String>,
  /// Whether files have been populated for current session.
  pub populated: bool,
  /// Animated line count for preview.
  pub line_count_anim: Animatable<f64>,
  /// Animated match count for search results.
  pub match_count_anim: Animatable<f64>,
  /// Per-row padding animations.
  pub row_padding_anims: FxHashMap<usize, RowPaddingAnim>,
  /// Previous selection for detecting changes.
  pub prev_selection: Option<usize>,
}

impl Default for FilePickerState {
  fn default() -> Self {
    Self {
      visible: false,
      mode: FilePickerMode::Files,
      search_input: String::new(),
      query: PickerQuery::default(),
      selection: 0,
      scroll_offset: 0,
      show_preview: true,
      preview_cache: FxHashMap::default(),
      version: Arc::new(AtomicUsize::new(0)),
      animation_start: None,
      root_paths: Vec::new(),
      column_names: vec!["path".to_string(), "name".to_string()],
      populated: false,
      line_count_anim: Animatable::new(
        0.0,
        0.0,
        0.3,
        Easing::Interpolation(Interpolation::InOutSmooth),
      ),
      match_count_anim: Animatable::new(
        0.0,
        0.0,
        0.3,
        Easing::Interpolation(Interpolation::InOutSmooth),
      ),
      row_padding_anims: FxHashMap::default(),
      prev_selection: None,
    }
  }
}

impl FilePickerState {
  pub fn new() -> Self {
    Self::default()
  }

  /// Show the file picker with the given mode.
  pub fn show(&mut self, mode: FilePickerMode) {
    self.visible = true;
    self.mode = mode;
    self.search_input.clear();
    self.query = PickerQuery::default();
    self.selection = 0;
    self.scroll_offset = 0;
    self.animation_start = None;
    self.populated = false; // Reset so files get repopulated.
    self.version.fetch_add(1, Ordering::Relaxed);
  }

  /// Hide the file picker.
  pub fn hide(&mut self) {
    self.visible = false;
    self.animation_start = None;
  }

  /// Toggle the file picker visibility.
  pub fn toggle(&mut self, mode: FilePickerMode) {
    if self.visible && self.mode == mode {
      self.hide();
    } else {
      self.show(mode);
    }
  }

  /// Update the search query.
  pub fn set_query(&mut self, input: String) {
    self.search_input = input.clone();
    self.query = PickerQuery::parse(&input, &self.column_names);
    self.selection = 0;
    self.scroll_offset = 0;
  }

  /// Move selection up or down.
  pub fn move_selection(&mut self, delta: i32, total_count: usize) {
    if total_count == 0 {
      return;
    }

    let new_selection = if delta > 0 {
      (self.selection + delta as usize) % total_count
    } else {
      let abs_delta = (-delta) as usize;
      (self.selection + total_count - (abs_delta % total_count)) % total_count
    };

    self.selection = new_selection;
  }

  /// Page up (move selection by ~20 items).
  pub fn page_up(&mut self, total_count: usize) {
    self.move_selection(-20, total_count);
  }

  /// Page down (move selection by ~20 items).
  pub fn page_down(&mut self, total_count: usize) {
    self.move_selection(20, total_count);
  }

  /// Get or load preview for a path.
  pub fn get_preview(&self, path: &PathBuf) -> Option<&CachedPreview> {
    self.preview_cache.get(path)
  }

  /// Cache a preview result.
  pub fn cache_preview(&mut self, path: PathBuf, preview: CachedPreview) {
    self.preview_cache.insert(path, preview);
  }

  /// Clear preview cache.
  pub fn clear_preview_cache(&mut self) {
    self.preview_cache.clear();
  }
}

/// Fuzzy matcher wrapper using nucleo.
pub struct FuzzyMatcher {
  /// The nucleo matcher instance.
  matcher: Nucleo<FilePickerItem>,
}

impl FuzzyMatcher {
  pub fn new() -> Self {
    let matcher = Nucleo::new(
      Config::DEFAULT,
      Arc::new(|| {}),
      None,
      1, // number of columns
    );

    Self { matcher }
  }

  /// Restart the matcher (clear all items).
  pub fn restart(&mut self) {
    self.matcher.restart(false);
  }

  /// Push an item into the matcher.
  pub fn push(&self, item: FilePickerItem) {
    let text = item.display_text();

    self
      .matcher
      .injector()
      .push(item, |_, cols| cols[0] = text.into());
  }

  /// Get injector for background population.
  pub fn injector(&self) -> nucleo::Injector<FilePickerItem> {
    self.matcher.injector()
  }

  /// Update the search pattern.
  pub fn set_pattern(&mut self, pattern: &str) {
    self.matcher.pattern.reparse(
      0, // column index
      pattern,
      CaseMatching::Smart,
      Normalization::Smart,
      false,
    );
  }

  /// Tick the matcher (process ~10ms of work).
  pub fn tick(&mut self) -> bool {
    self.matcher.tick(10).changed
  }

  /// Get count of matched items.
  pub fn matched_count(&self) -> usize {
    self.matcher.snapshot().matched_item_count() as usize
  }

  /// Get total item count.
  pub fn total_count(&self) -> usize {
    self.matcher.snapshot().item_count() as usize
  }

  /// Get item at index in matched results.
  pub fn get(&self, index: usize) -> Option<&FilePickerItem> {
    self
      .matcher
      .snapshot()
      .get_matched_item(index as u32)
      .map(|item| item.data)
  }

  /// Get match indices for highlighting.
  pub fn match_indices(&self, index: usize) -> Vec<u32> {
    let snapshot = self.matcher.snapshot();

    let Some(item) = snapshot.get_matched_item(index as u32) else {
      return Vec::new();
    };

    let mut indices = Vec::new();
    let pattern = snapshot.pattern();

    // Get the column pattern and find match positions.
    let col_pattern = pattern.column_pattern(0);
    let mut matcher = nucleo::Matcher::new(Config::DEFAULT);

    col_pattern.indices(
      item.matcher_columns[0].slice(..),
      &mut matcher,
      &mut indices,
    );

    indices
  }

  /// Iterate over all matched items (for rendering).
  pub fn iter_matches(&self) -> impl Iterator<Item = &FilePickerItem> {
    let snapshot = self.matcher.snapshot();

    (0..snapshot.matched_item_count())
      .filter_map(move |idx| snapshot.get_matched_item(idx).map(|i| i.data))
  }
}

impl Default for FuzzyMatcher {
  fn default() -> Self {
    Self::new()
  }
}

/// Resource holding the fuzzy matcher instance.
#[derive(Resource, Default)]
pub struct FilePickerMatcher {
  pub matcher: Option<FuzzyMatcher>,
}

impl FilePickerMatcher {
  pub fn new() -> Self {
    Self {
      matcher: Some(FuzzyMatcher::new()),
    }
  }

  pub fn get(&self) -> Option<&FuzzyMatcher> {
    self.matcher.as_ref()
  }

  pub fn get_mut(&mut self) -> Option<&mut FuzzyMatcher> {
    self.matcher.as_mut()
  }

  pub fn reset(&mut self) {
    self.matcher = Some(FuzzyMatcher::new());
  }
}

/// Command to control file picker.
#[derive(Message, Debug, Clone)]
pub struct FilePickerCommand {
  pub action: FilePickerAction,
}

/// Actions that can be performed on the file picker.
#[derive(Debug, Clone)]
pub enum FilePickerAction {
  /// Show the file picker with the given mode.
  Show(FilePickerMode),
  /// Hide the file picker.
  Hide,
  /// Toggle the file picker with the given mode.
  Toggle(FilePickerMode),
  /// Update the search query.
  UpdateQuery(String),
  /// Move selection up.
  SelectPrevious,
  /// Move selection down.
  SelectNext,
  /// Page up.
  PageUp,
  /// Page down.
  PageDown,
  /// Go to first item.
  SelectFirst,
  /// Go to last item.
  SelectLast,
  /// Confirm selection with action.
  Confirm(SelectAction),
  /// Toggle preview visibility.
  TogglePreview,
  /// Refresh file list.
  Refresh,
}

/// Response from the file picker.
#[derive(Debug, Clone)]
pub enum FilePickerResponse {
  /// User selected an item.
  Select(PathBuf, SelectAction),
  /// User closed the picker.
  Close,
  /// No action.
  None,
}

/// Load preview for a file synchronously.
pub fn load_preview(path: &PathBuf) -> CachedPreview {
  // Check metadata.
  let metadata = match std::fs::metadata(path) {
    Ok(m) => m,
    Err(_) => return CachedPreview::NotFound,
  };

  // Directory.
  if metadata.is_dir() {
    let entries = std::fs::read_dir(path)
      .ok()
      .map(|rd| {
        let mut entries = rd
          .filter_map(|e| e.ok())
          .map(|e| DirEntry {
            name: e.file_name().to_string_lossy().to_string(),
            is_dir: e.file_type().map(|t| t.is_dir()).unwrap_or(false),
          })
          .collect::<Vec<_>>();

        entries.sort_by(|a, b| {
          // Directories first, then by name.
          match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
          }
        });

        entries
      })
      .unwrap_or_default();

    return CachedPreview::Directory(entries);
  }

  // Size check (10MB limit).
  const MAX_SIZE: u64 = 10 * 1024 * 1024;

  if metadata.len() > MAX_SIZE {
    return CachedPreview::LargeFile;
  }

  // Read content.
  let content = match std::fs::read(path) {
    Ok(bytes) => bytes,
    Err(_) => return CachedPreview::NotFound,
  };

  // Binary check (look for null bytes in first 1KB).
  let check_len = content.len().min(1024);

  if content[..check_len].contains(&0) {
    return CachedPreview::Binary;
  }

  // Convert to string.
  let content = String::from_utf8_lossy(&content).to_string();

  // Store extension for language detection (Language::from expects extension).
  // Handle dotfiles like .env by stripping the leading dot.
  let language = path
    .extension()
    .and_then(|e| e.to_str())
    .map(|s| s.to_string())
    .or_else(|| {
      path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|name| name.strip_prefix('.'))
        .map(|s| s.to_string())
    });

  CachedPreview::Document { content, language }
}
