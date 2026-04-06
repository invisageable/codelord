use crate::symbol::SymbolKind;

use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use eazy::interpolation::linear::lerp::lerp;
use eazy::{Curve, Easing};
use flume::Receiver;

use std::ops::Range;
use std::path::{Path, PathBuf};

/// Segment type discriminator for breadcrumbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
  /// File path component (folder or filename).
  Path { is_filename: bool },
  /// Symbol in code (function, struct, impl, etc.).
  Symbol { kind: SymbolKind },
}

/// A breadcrumb segment for display.
#[derive(Debug, Clone)]
pub struct BreadcrumbSegment {
  /// Display text.
  pub text: String,
  /// Segment classification.
  pub kind: SegmentKind,
  /// Optional: byte range for click-to-jump (symbols only).
  pub byte_range: Option<Range<usize>>,
  /// Optional: syntax highlight spans for rich rendering.
  /// Format: (byte_range_in_text, token_type_as_u8).
  pub highlights: Vec<(Range<usize>, u8)>,
}

impl BreadcrumbSegment {
  /// Create a path segment.
  pub fn path(text: impl Into<String>, is_filename: bool) -> Self {
    Self {
      text: text.into(),
      kind: SegmentKind::Path { is_filename },
      byte_range: None,
      highlights: Vec::new(),
    }
  }

  /// Create a symbol segment.
  pub fn symbol(
    text: impl Into<String>,
    kind: SymbolKind,
    byte_range: Range<usize>,
    highlights: Vec<(Range<usize>, u8)>,
  ) -> Self {
    Self {
      text: text.into(),
      kind: SegmentKind::Symbol { kind },
      byte_range: Some(byte_range),
      highlights,
    }
  }

  /// Check if this is a filename segment.
  pub fn is_filename(&self) -> bool {
    matches!(self.kind, SegmentKind::Path { is_filename: true })
  }

  /// Check if this is a symbol segment.
  pub fn is_symbol(&self) -> bool {
    matches!(self.kind, SegmentKind::Symbol { .. })
  }
}

/// Resource holding pre-computed breadcrumb segments.
#[derive(Resource, Debug, Default)]
pub struct BreadcrumbData {
  pub segments: Vec<BreadcrumbSegment>,
}

/// Resource for animated items count display in explorer header.
///
/// Follows the same pattern as codelord's CounterAnimation - proper start value
/// tracking for smooth interpolation using eazy::Easing.
#[derive(Resource, Debug, Clone)]
pub struct ExplorerItemsCounter {
  /// Current displayed count (animated)
  pub count: usize,
  /// Target count value
  target: f32,
  /// Current interpolated count
  current: f32,
  /// Animation elapsed time
  elapsed: f32,
  /// Animation duration
  duration: f32,
  /// Start value for interpolation
  start: f32,
  /// Whether animation is currently active
  pub is_active: bool,
}

impl Default for ExplorerItemsCounter {
  fn default() -> Self {
    Self {
      count: 0,
      target: 0.0,
      current: 0.0,
      elapsed: 0.0,
      duration: 0.3,
      start: 0.0,
      is_active: false,
    }
  }
}

impl ExplorerItemsCounter {
  pub fn new() -> Self {
    Self::default()
  }

  /// Set new target value. Returns true if animation was started.
  pub fn set_target(&mut self, count: usize) -> bool {
    let new_target = count as f32;

    if (self.target - new_target).abs() > 0.001 {
      self.start = self.current;
      self.target = new_target;
      self.elapsed = 0.0;

      // Only return true if we weren't already animating
      let was_inactive = !self.is_active;
      self.is_active = true;

      return was_inactive;
    }

    false
  }

  /// Update animation with delta time. Returns true if animation completed.
  pub fn update(&mut self, dt: f32) -> bool {
    if !self.is_active {
      return false;
    }

    self.elapsed += dt;

    if self.elapsed >= self.duration {
      // Animation complete
      self.current = self.target;
      self.count = self.target.round() as usize;
      self.is_active = false;

      return true;
    }

    // Interpolate from start to target
    let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
    let eased = Easing::InOutSine.y(t);

    self.current = lerp(eased, self.start, self.target);
    self.count = self.current.round() as usize;

    false
  }
}

/// Resource for explorer state with multi-root workspace support.
#[derive(Resource, Debug, Default)]
pub struct ExplorerState {
  /// All root paths in the workspace.
  pub roots: Vec<PathBuf>,
  /// Whether to show hidden files (dotfiles).
  pub show_hidden: bool,
}

impl ExplorerState {
  /// Create with a single root.
  pub fn new(root_path: PathBuf) -> Self {
    Self {
      roots: vec![root_path],
      show_hidden: false,
    }
  }

  /// Get the primary (first) root path.
  pub fn root_path(&self) -> Option<&PathBuf> {
    self.roots.first()
  }

  /// Check if workspace has multiple roots.
  pub fn is_multi_root(&self) -> bool {
    self.roots.len() > 1
  }

  /// Add a root to the workspace.
  pub fn add_root(&mut self, path: PathBuf) {
    // Avoid duplicates
    if !self.roots.iter().any(|r| r == &path) {
      self.roots.push(path);
    }
  }

  /// Remove a root from the workspace (only if multiple roots exist).
  pub fn remove_root(&mut self, path: &Path) -> bool {
    // Don't remove the last root
    if self.roots.len() <= 1 {
      return false;
    }
    self.roots.retain(|r| r != path);
    true
  }

  /// Check if a path is a workspace root.
  pub fn is_root(&self, path: &Path) -> bool {
    self.roots.iter().any(|r| r == path)
  }

  /// Find which workspace root contains the given path.
  /// Returns the root path if found.
  pub fn find_root_for_path(&self, path: &Path) -> Option<&PathBuf> {
    self.roots.iter().find(|root| path.starts_with(root))
  }
}

/// Resource tracking the currently active workspace root.
///
/// Updated when the user selects an item in the explorer or switches tabs.
/// Used by the titlebar to display the current workspace name.
#[derive(Resource, Debug, Default, Clone)]
pub struct ActiveWorkspaceRoot {
  /// The root path of the active workspace.
  pub path: Option<PathBuf>,
  /// The display name (folder name) of the active workspace.
  pub name: Option<String>,
}

impl ActiveWorkspaceRoot {
  /// Update from a file path by finding its workspace root.
  pub fn update_from_path(&mut self, path: &Path, explorer: &ExplorerState) {
    if let Some(root) = explorer.find_root_for_path(path) {
      let name = root.file_name().map(|n| n.to_string_lossy().to_string());
      self.path = Some(root.clone());
      self.name = name;
    }
  }

  /// Clear the active workspace (no selection).
  pub fn clear(&mut self) {
    self.path = None;
    self.name = None;
  }
}

/// Resource for indentation lines animation state.
#[derive(Resource, Debug, Default)]
pub struct IndentationLinesState {
  /// Whether the file tree area is currently hovered.
  pub is_hovered: bool,
  /// Time when hover state changed (for animation).
  pub hover_start_time: Option<f64>,
  /// Whether we're fading in (true) or fading out (false).
  pub fading_in: bool,
  /// Whether we need to decrement ActiveAnimations when animation completes.
  pub needs_decrement: bool,
}

impl IndentationLinesState {
  /// Update hover state and return whether animation should be active.
  pub fn set_hovered(&mut self, hovered: bool, current_time: f64) -> bool {
    if hovered != self.is_hovered {
      self.is_hovered = hovered;
      self.hover_start_time = Some(current_time);
      self.fading_in = hovered;
      self.needs_decrement = true;
      true // Animation started
    } else {
      false
    }
  }

  /// Check if animation is currently active.
  pub fn is_animating(&self, current_time: f64, duration: f64) -> bool {
    self
      .hover_start_time
      .is_some_and(|start| current_time - start < duration)
  }
}

/// Resource for pending folder dialog receiver.
#[derive(Resource)]
pub struct PendingFolderDialog(pub Receiver<Option<PathBuf>>);

/// Resource for pending "add folder to workspace" dialog receiver.
#[derive(Resource)]
pub struct PendingWorkspaceFolderDialog(pub Receiver<Option<PathBuf>>);

/// Resource tracking the target of a context menu action in the explorer.
#[derive(Resource, Debug, Default)]
pub struct ExplorerContextTarget {
  /// The entity that was right-clicked (if any).
  pub entity: Option<Entity>,
  /// The path of the right-clicked item.
  pub path: Option<PathBuf>,
  /// Whether the target is a directory.
  pub is_dir: bool,
}

/// Editing mode for inline file/folder operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerEditingMode {
  /// Creating a new file.
  NewFile,
  /// Creating a new folder.
  NewFolder,
  /// Renaming an existing item.
  Rename,
}

/// Resource for inline editing state in the explorer.
#[derive(Resource, Debug, Default)]
pub struct ExplorerEditingState {
  /// Current editing mode (None if not editing).
  pub mode: Option<ExplorerEditingMode>,
  /// The text being edited.
  pub text: String,
  /// Entity being renamed (for Rename mode).
  pub target_entity: Option<Entity>,
  /// Parent path for new file/folder creation.
  pub parent_path: Option<PathBuf>,
  /// Depth for rendering the input at correct indentation.
  pub depth: u32,
}

/// A stage in the stagebar.
#[derive(Debug, Clone)]
pub struct Stage {
  /// Display label.
  pub label: &'static str,
  /// Unique identifier.
  pub id: usize,
}

impl Stage {
  pub const fn new(label: &'static str, id: usize) -> Self {
    Self { label, id }
  }
}

/// Output mode for the playground (determines last stage).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaygroundMode {
  /// Programming mode: Tokens → Tree → SIR → Asm
  #[default]
  Programming,
  /// Templating mode: Tokens → Tree → SIR → UI
  Templating,
}

/// Resource for stagebar state with animated highlight.
///
/// The stagebar is a segmented tab navigation component. The highlight
/// pill slides smoothly between selected stages using InOutSine easing.
/// Animation is index-based - positions are computed from layout each frame.
#[derive(Resource, Debug, Clone)]
pub struct StagebarResource {
  /// Available stages.
  pub stages: Vec<Stage>,
  /// Currently selected stage index.
  pub selected: usize,
  /// Start index for animation interpolation.
  pub start_index: usize,
  /// Target index for animation interpolation.
  pub target_index: usize,
  /// Animation progress (0.0 to 1.0).
  pub progress: f32,
  /// Whether animation is active.
  pub is_animating: bool,
  /// Animation duration in seconds.
  pub duration: f32,
  /// Track if animation was active last frame (for ActiveAnimations).
  was_animating: bool,
  /// Current playground mode (Programming vs Templating).
  pub mode: PlaygroundMode,
  /// Label morph animation progress (0.0 to 1.0).
  pub morph_progress: f32,
  /// Whether label morph animation is active.
  pub is_morphing: bool,
  /// Previous label for morph animation (fading out).
  pub morph_from_label: &'static str,
  /// Track if morph was active last frame.
  was_morphing: bool,
}

impl Default for StagebarResource {
  fn default() -> Self {
    Self {
      stages: Vec::new(),
      selected: 0,
      start_index: 0,
      target_index: 0,
      progress: 1.0,
      is_animating: false,
      duration: 0.3,
      was_animating: false,
      mode: PlaygroundMode::Programming,
      morph_progress: 1.0,
      is_morphing: false,
      morph_from_label: "Asm",
      was_morphing: false,
    }
  }
}

impl StagebarResource {
  /// Create a new stagebar with the given stages.
  pub fn new(stages: Vec<Stage>) -> Self {
    Self {
      stages,
      ..Default::default()
    }
  }

  /// Create with default compiler stages (Tokens, Tree, SIR, Asm).
  pub fn compiler_stages() -> Self {
    Self::new(vec![
      Stage::new("Tokens", 0),
      Stage::new("Tree", 1),
      Stage::new("SIR", 2),
      Stage::new("Asm", 3),
    ])
  }

  /// Set the playground mode and animate the last stage label change.
  /// Returns true if mode changed and animation started.
  pub fn set_mode(&mut self, mode: PlaygroundMode) -> bool {
    if self.mode == mode {
      return false;
    }

    // Store the old label for morph animation
    if let Some(last) = self.stages.last() {
      self.morph_from_label = last.label;
    }

    // Update the last stage label based on mode
    let new_label = match mode {
      PlaygroundMode::Programming => "Asm",
      PlaygroundMode::Templating => "Ui",
    };

    if let Some(last) = self.stages.last_mut() {
      last.label = new_label;
    }

    self.mode = mode;
    self.morph_progress = 0.0;
    self.is_morphing = true;

    true
  }

  /// Select a stage and start animation. Returns true if selection changed.
  pub fn select(&mut self, index: usize) -> bool {
    if index >= self.stages.len() || index == self.selected {
      return false;
    }

    self.start_index = self.selected;
    self.target_index = index;
    self.selected = index;
    self.progress = 0.0;
    self.is_animating = true;

    true
  }

  /// Update animation with delta time.
  pub fn update(&mut self, dt: f32) {
    // Update selection animation
    if self.is_animating {
      self.progress += dt / self.duration;

      if self.progress >= 1.0 {
        self.progress = 1.0;
        self.is_animating = false;
      }
    }

    // Update morph animation
    if self.is_morphing {
      self.morph_progress += dt / self.duration;

      if self.morph_progress >= 1.0 {
        self.morph_progress = 1.0;
        self.is_morphing = false;
      }
    }
  }

  /// Get eased animation progress using OutElastic.
  pub fn eased_progress(&self) -> f32 {
    Easing::OutElastic.y(self.progress)
  }

  /// Get eased morph progress using InOutSine.
  pub fn eased_morph_progress(&self) -> f32 {
    Easing::InOutSine.y(self.morph_progress)
  }

  /// Check animation state transition for ActiveAnimations tracking.
  /// Returns: (should_increment, should_decrement)
  pub fn check_animation_transition(&mut self) -> (bool, bool) {
    let currently_animating = self.is_animating || self.is_morphing;
    let was = self.was_animating || self.was_morphing;
    self.was_animating = self.is_animating;
    self.was_morphing = self.is_morphing;

    match (was, currently_animating) {
      (false, true) => (true, false),
      (true, false) => (false, true),
      _ => (false, false),
    }
  }
}

/// Resource for file clipboard (cut/copy/paste).
#[derive(Resource, Debug, Default)]
pub struct FileClipboard {
  /// Path of the file/folder in clipboard.
  pub path: Option<PathBuf>,
  /// Whether this is a cut (move) or copy operation.
  pub is_cut: bool,
}

impl FileClipboard {
  /// Set clipboard to cut mode.
  pub fn set_cut(&mut self, path: PathBuf) {
    self.path = Some(path);
    self.is_cut = true;
  }

  /// Set clipboard to copy mode.
  pub fn set_copy(&mut self, path: PathBuf) {
    self.path = Some(path);
    self.is_cut = false;
  }

  /// Clear the clipboard.
  pub fn clear(&mut self) {
    self.path = None;
    self.is_cut = false;
  }

  /// Check if clipboard is empty.
  pub fn is_empty(&self) -> bool {
    self.path.is_none()
  }
}
