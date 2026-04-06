use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;

/// Resource to track the context menu target tab.
/// Pure data - similar to ExplorerContextTarget.
#[derive(Resource, Debug, Default)]
pub struct TabContextTarget {
  /// The tab entity that was right-clicked.
  pub entity: Option<Entity>,
  /// The tab's order (for "close to right" logic).
  pub order: u32,
}

/// Identifies the source of a zoom request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoomSource {
  /// Zoom triggered from editor tabbar.
  #[default]
  Editor,
  /// Zoom triggered from terminal tabbar.
  Terminal,
  /// Zoom triggered from playground tabbar.
  Playground,
}

/// Resource to track the next tab order number.
#[derive(Resource, Debug, Default)]
pub struct TabOrderCounter(pub u32);

impl TabOrderCounter {
  #[allow(clippy::should_implement_trait)]
  pub fn next(&mut self) -> u32 {
    let order = self.0;
    self.0 += 1;
    order
  }

  pub fn reset(&mut self) {
    self.0 = 0;
  }
}

/// Snapshot of panel visibility states before zoom mode.
/// Used to restore the exact panel configuration when exiting zoom.
#[derive(Debug, Clone, Copy, Default)]
pub struct PanelSnapshot {
  pub left_panel: bool,
  pub right_panel: bool,
  pub bottom_panel: bool,
}

/// Animation state for zoom transitions (pure data).
#[derive(Debug, Clone, Copy)]
pub struct ZoomTransition {
  /// Raw animation progress from 0.0 (start) to 1.0 (end).
  pub progress: f32,
  /// Eased progress (computed by system).
  pub eased_progress: f32,
  /// Animated margin value (computed by system).
  pub animated_margin: f32,
  /// Duration of the animation in seconds.
  pub duration: f32,
  /// Elapsed time.
  pub elapsed: f32,
  /// Target zoomed state (true = zooming in, false = zooming out).
  pub target_zoomed: bool,
}

impl ZoomTransition {
  /// Create a new zoom transition with initial values.
  pub fn new(target_zoomed: bool) -> Self {
    Self {
      progress: 0.0,
      eased_progress: 0.0,
      animated_margin: if target_zoomed { 0.0 } else { 4.0 },
      duration: 0.2,
      elapsed: 0.0,
      target_zoomed,
    }
  }
}

/// Resource to track zoom state for the editor.
#[derive(Resource, Debug, Default)]
pub struct ZoomState {
  pub is_zoomed: bool,
  /// Source of the zoom (editor or terminal).
  pub source: ZoomSource,
  /// Panel state before zoom (None if not zoomed).
  pub pre_zoom_snapshot: Option<PanelSnapshot>,
  /// Active zoom animation (None if not animating).
  pub transition: Option<ZoomTransition>,
}

/// Response from the unsaved changes dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnsavedChangesResponse {
  /// Dialog is still open, no action taken.
  #[default]
  None,
  /// User clicked Save button.
  Save,
  /// User clicked Don't Save button.
  DontSave,
  /// User clicked Cancel button or closed dialog.
  Cancel,
}

/// Resource to track unsaved changes dialog state.
/// Shown when trying to close a tab with unsaved changes.
#[derive(Resource, Debug, Default)]
pub struct UnsavedChangesDialog {
  /// The tab entity being closed.
  pub entity: Option<Entity>,
  /// The filename for display.
  pub filename: String,
  /// Animation start time (seconds from ui.input(|i| i.time)).
  pub animation_start: Option<f64>,
}

impl UnsavedChangesDialog {
  /// Show the dialog for a specific tab.
  pub fn show(&mut self, entity: Entity, filename: impl Into<String>) {
    self.entity = Some(entity);
    self.filename = filename.into();
    self.animation_start = None; // Will be set on first render
  }

  /// Close the dialog.
  pub fn close(&mut self) {
    self.entity = None;
    self.filename.clear();
    self.animation_start = None;
  }

  /// Check if dialog is visible.
  pub fn is_visible(&self) -> bool {
    self.entity.is_some()
  }
}
