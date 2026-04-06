use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use flume::Receiver;

use std::path::PathBuf;

/// Resource for pending "Save As" file dialog.
/// Contains the receiver for the dialog result and the tab entity to save.
#[derive(Resource)]
pub struct PendingSaveFileDialog {
  pub receiver: Receiver<Option<PathBuf>>,
  pub entity: Entity,
}

impl PendingSaveFileDialog {
  pub fn new(receiver: Receiver<Option<PathBuf>>, entity: Entity) -> Self {
    Self { receiver, entity }
  }
}

/// Settings for indent guides (vertical lines showing indentation).
#[derive(Resource, Debug, Clone)]
pub struct IndentGuidesSettings {
  /// Whether indent guides are enabled.
  pub enabled: bool,
  /// Highlight the guide at cursor's indent scope.
  pub highlight_active_scope: bool,
  /// Line width in pixels.
  pub line_width: f32,
  /// Indent size in spaces (for calculating guide columns).
  pub indent_size: usize,
}

impl Default for IndentGuidesSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      highlight_active_scope: true,
      line_width: 1.0,
      indent_size: 2,
    }
  }
}

/// Active indent scope information for a cursor position.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActiveIndentScope {
  /// First line of the active scope.
  pub start_line: usize,
  /// Last line of the active scope.
  pub end_line: usize,
  /// Indent level of the active scope (in indent_size units).
  pub indent_level: usize,
}
