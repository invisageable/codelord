//! Session persistence for IDE state.
//!
//! Handles saving and restoring workspace state between sessions:
//! - Open files with cursor positions
//! - Panel visibility
//! - Theme preference
//! - Explorer state (workspace roots, expanded folders)

use codelord_core::ecs::world::World;
use codelord_core::navigation::components::{Expanded, FileEntry};
use codelord_core::navigation::resources::ExplorerState;
use codelord_core::panel::resources::{
  BottomPanelResource, LeftPanelResource, RightPanelResource,
};
use codelord_core::tabbar::components::{EditorTab, Tab};
use codelord_core::text_editor::components::{Cursor, FileTab, TextBuffer};
use codelord_core::theme::components::ThemeKind;
use codelord_core::theme::resources::ThemeResource;
use codelord_core::ui::component::Active;

use bevy_ecs::query::With;

use serde::{Deserialize, Serialize};

use std::path::PathBuf;

/// Key used to store session state in eframe storage.
pub const SESSION_KEY: &str = "codelord_session_v1";

/// Clear the saved session from storage.
pub fn clear_session(storage: &mut dyn eframe::Storage) {
  storage.set_string(SESSION_KEY, String::new());

  log::info!("[Session] Cleared saved session");
}

/// Serializable session state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
  /// Open editor tabs.
  pub tabs: Vec<TabState>,
  /// Index of the active tab (if any).
  pub active_tab_index: Option<usize>,
  /// Panel visibility.
  pub panels: PanelState,
  /// Current theme.
  pub theme: ThemeState,
  /// Explorer state.
  pub explorer: ExplorerSessionState,
}

/// State of a single editor tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
  /// File path (None for untitled tabs).
  pub path: Option<PathBuf>,
  /// Tab title/name.
  pub title: String,
  /// Text content.
  pub content: String,
  /// Cursor position (byte offset).
  pub cursor_position: usize,
  /// Whether the tab has unsaved changes.
  pub is_dirty: bool,
  /// Tab order for sorting.
  pub order: u32,
}

/// Panel visibility state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelState {
  /// Left panel (explorer) visible.
  pub left_visible: bool,
  /// Right panel (copilord) visible.
  pub right_visible: bool,
  /// Bottom panel (terminal) visible.
  pub bottom_visible: bool,
}

impl Default for PanelState {
  fn default() -> Self {
    Self {
      left_visible: true,
      right_visible: false,
      bottom_visible: false,
    }
  }
}

/// Theme state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeState {
  /// Theme kind (dark/light).
  pub kind: String,
}

impl Default for ThemeState {
  fn default() -> Self {
    Self {
      kind: "dark".to_string(),
    }
  }
}

/// Explorer state for session persistence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExplorerSessionState {
  /// Workspace root directories.
  pub roots: Vec<PathBuf>,
  /// Expanded folder paths.
  pub expanded_folders: Vec<PathBuf>,
}

impl SessionState {
  /// Extract session state from the ECS World.
  pub fn from_world(world: &mut World) -> Self {
    let mut state = SessionState::default();

    state.extract_tabs(world);
    state.extract_panels(world);
    state.extract_theme(world);
    state.extract_explorer(world);

    state
  }

  /// Extract tab state from World.
  fn extract_tabs(&mut self, world: &mut World) {
    // Query all editor tabs with their components
    // EditorTab is a marker, FileTab holds the path
    let mut tabs: Vec<(bevy_ecs::entity::Entity, TabState)> = world
      .query_filtered::<(
        bevy_ecs::entity::Entity,
        &Tab,
        Option<&FileTab>,
        &TextBuffer,
        &Cursor,
      ), With<EditorTab>>()
      .iter(world)
      .map(|(entity, tab, file_tab, buffer, cursor)| {
        (
          entity,
          TabState {
            path: file_tab.map(|ft| ft.path.clone()),
            title: tab.label.clone(),
            content: buffer.to_string(),
            cursor_position: cursor.position,
            is_dirty: buffer.modified,
            order: tab.order,
          },
        )
      })
      .collect();

    // Sort by order
    tabs.sort_by_key(|(_, t)| t.order);

    // Find active tab index
    let active_entity = world
      .query_filtered::<bevy_ecs::entity::Entity, (With<EditorTab>, With<Active>)>()
      .iter(world)
      .next();

    self.active_tab_index = active_entity
      .and_then(|active| tabs.iter().position(|(e, _)| *e == active));

    self.tabs = tabs.into_iter().map(|(_, t)| t).collect();
  }

  /// Extract panel state from World.
  fn extract_panels(&mut self, world: &mut World) {
    if let Some(left) = world.get_resource::<LeftPanelResource>() {
      self.panels.left_visible = left.is_visible;
    }

    if let Some(right) = world.get_resource::<RightPanelResource>() {
      self.panels.right_visible = right.is_visible;
    }

    if let Some(bottom) = world.get_resource::<BottomPanelResource>() {
      self.panels.bottom_visible = bottom.is_visible;
    }
  }

  /// Extract theme state from World.
  fn extract_theme(&mut self, world: &mut World) {
    if let Some(theme) = world.get_resource::<ThemeResource>() {
      self.theme.kind = match theme.current {
        ThemeKind::Dark => "dark",
        ThemeKind::Light => "light",
        ThemeKind::Custom => "custom",
      }
      .to_string()
    }
  }

  /// Extract explorer state from World.
  fn extract_explorer(&mut self, world: &mut World) {
    if let Some(explorer) = world.get_resource::<ExplorerState>() {
      self.explorer.roots = explorer.roots.clone();
    }

    self.explorer.expanded_folders = world
      .query_filtered::<&FileEntry, With<Expanded>>()
      .iter(world)
      .filter(|entry| entry.is_dir)
      .map(|entry| entry.path.clone())
      .collect();
  }
}
