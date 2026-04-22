//! Session state — ECS snapshot for persistence.
//!
//! DTOs plus pure-ECS extract (`from_world`) and restore
//! (`apply_to_world`). No persistence transport here — the app shell
//! (codelord-coder) reads/writes these DTOs from eframe's storage.

use crate::ecs::query::With;
use crate::ecs::world::World;
use crate::events::OpenPdfPreviewRequest;
use crate::keyboard::{Focusable, KeyboardHandler};
use crate::navigation::components::{Expanded, FileEntry};
use crate::navigation::resources::{ActiveWorkspaceRoot, ExplorerState};
use crate::panel::resources::{
  BottomPanelResource, LeftPanelResource, RightPanelResource,
};
use crate::previews;
use crate::previews::font::FontPreviewState;
use crate::previews::svg::SvgPreviewState;
use crate::previews::{SqlitePreviewState, XlsPreviewState};
use crate::tabbar::components::{EditorTab, SonarAnimation, Tab};
use crate::tabbar::resources::TabOrderCounter;
use crate::text_editor::components::{Cursor, FileTab, TextBuffer};
use crate::theme::components::ThemeKind;
use crate::theme::resources::ThemeResource;
use crate::ui::component::Active;
use crate::{git, symbol};

use serde::{Deserialize, Serialize};

use std::path::PathBuf;

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

  /// Apply session state into the ECS world. Spawns editor tabs, flips
  /// panel visibility, restores theme + workspace roots, and triggers
  /// preview activation for binary files.
  ///
  /// Returns `true` if any tabs were restored (the caller uses this to
  /// decide whether to spawn a default playground tab).
  pub fn apply_to_world(&self, world: &mut World) -> bool {
    log::info!(
      "[Session] Restoring: {} tabs, theme: {}, roots: {}",
      self.tabs.len(),
      self.theme.kind,
      self.explorer.roots.len()
    );

    self.restore_theme(world);
    self.restore_panels(world);
    self.restore_explorer(world);

    if self.tabs.is_empty() {
      return false;
    }

    self.restore_tabs(world);
    self.activate_preview_for_active_tab(world);

    log::info!(
      "[Session] Restored {} tabs, active: {:?}",
      self.tabs.len(),
      self.active_tab_index
    );

    true
  }

  fn extract_tabs(&mut self, world: &mut World) {
    let mut tabs: Vec<(crate::ecs::entity::Entity, TabState)> = world
      .query_filtered::<(
        crate::ecs::entity::Entity,
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

    tabs.sort_by_key(|(_, t)| t.order);

    let active_entity = world
      .query_filtered::<crate::ecs::entity::Entity, (With<EditorTab>, With<Active>)>()
      .iter(world)
      .next();

    self.active_tab_index = active_entity
      .and_then(|active| tabs.iter().position(|(e, _)| *e == active));

    self.tabs = tabs.into_iter().map(|(_, t)| t).collect();
  }

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

  fn restore_theme(&self, world: &mut World) {
    if let Some(mut theme_res) = world.get_resource_mut::<ThemeResource>() {
      theme_res.current = match self.theme.kind.as_str() {
        "light" => ThemeKind::Light,
        "custom" => ThemeKind::Custom,
        _ => ThemeKind::Dark,
      }
    }
  }

  fn restore_panels(&self, world: &mut World) {
    if let Some(mut left) = world.get_resource_mut::<LeftPanelResource>() {
      left.is_visible = self.panels.left_visible;
    }

    if let Some(mut right) = world.get_resource_mut::<RightPanelResource>() {
      right.is_visible = self.panels.right_visible;
    }

    if let Some(mut bottom) = world.get_resource_mut::<BottomPanelResource>() {
      bottom.is_visible = self.panels.bottom_visible;
    }
  }

  fn restore_explorer(&self, world: &mut World) {
    if self.explorer.roots.is_empty() {
      return;
    }

    if let Some(mut explorer) = world.get_resource_mut::<ExplorerState>() {
      explorer.roots = self.explorer.roots.clone();
    }

    if let Some(first_root) = self.explorer.roots.first()
      && let Some(mut active_ws) =
        world.get_resource_mut::<ActiveWorkspaceRoot>()
    {
      active_ws.path = Some(first_root.clone());
      active_ws.name = first_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string());
    }
  }

  fn restore_tabs(&self, world: &mut World) {
    for (idx, tab_state) in self.tabs.iter().enumerate() {
      let order = world
        .get_resource_mut::<TabOrderCounter>()
        .map(|mut counter| counter.next())
        .unwrap_or(idx as u32);

      // Determine content: reload from disk if file exists and not dirty,
      // otherwise use saved content (for unsaved changes or new files).
      let content = if !tab_state.is_dirty {
        tab_state
          .path
          .as_ref()
          .and_then(|p| std::fs::read_to_string(p).ok())
          .unwrap_or_else(|| tab_state.content.clone())
      } else {
        tab_state.content.clone()
      };

      let mut buffer = TextBuffer::new(&content);
      buffer.modified = tab_state.is_dirty;

      let mut entity = world.spawn((
        Tab::new(&tab_state.title, order),
        EditorTab,
        SonarAnimation::default(),
        buffer,
        Cursor::new(tab_state.cursor_position.min(content.len())),
        symbol::TabSymbols::new(),
        git::components::TabBlame::new(),
        Focusable,
        KeyboardHandler::text_editor(),
      ));

      if let Some(path) = tab_state.path.as_ref() {
        entity.insert(FileTab::new(path.clone()));
      }

      if self.active_tab_index == Some(idx) {
        entity.insert(Active);
      }
    }
  }

  fn activate_preview_for_active_tab(&self, world: &mut World) {
    let Some(active_idx) = self.active_tab_index else {
      return;
    };
    let Some(tab_state) = self.tabs.get(active_idx) else {
      return;
    };
    let Some(path) = &tab_state.path else {
      return;
    };

    if previews::sqlite::accepts(path) {
      if let Some(mut sqlite_preview) =
        world.get_resource_mut::<SqlitePreviewState>()
      {
        sqlite_preview.enabled = true;
        sqlite_preview.current_file = Some(path.clone());
        sqlite_preview.needs_reload = true;
      }
    } else if previews::pdf::accepts(path) {
      world.spawn(OpenPdfPreviewRequest(path.clone()));
    } else if previews::xls::accepts(path) {
      if let Some(mut xls_preview) = world.get_resource_mut::<XlsPreviewState>()
      {
        xls_preview.open(path.clone());
      }
    } else if previews::font::accepts(path) {
      if let Some(mut font_preview) =
        world.get_resource_mut::<FontPreviewState>()
      {
        font_preview.open(path);
      }
    } else if previews::svg::accepts(path)
      && let Some(mut svg_preview) = world.get_resource_mut::<SvgPreviewState>()
    {
      svg_preview.open(path);
    }
  }
}

/// Reset the world to a fresh post-install state: despawn every editor
/// tab and explorer entry, clear the workspace roots, reset panel
/// visibility and the tab-order counter, then spawn a fresh empty
/// playground tab.
///
/// Called from the app shell when the user requests "Clear Session".
pub fn reset_to_fresh_state(world: &mut World) {
  use crate::ecs::entity::Entity;
  use crate::keyboard::{Focusable, KeyboardHandler};
  use crate::navigation::components::FileEntry;
  use crate::panel::resources::{
    BottomPanelResource, LeftPanelResource, RightPanelResource,
  };
  use crate::tabbar::components::{PlaygroundTab, SonarAnimation, Tab};
  use crate::tabbar::resources::TabOrderCounter;
  use crate::text_editor::components::{Cursor, TextBuffer};
  use crate::ui::component::Active;

  let editor_tabs: Vec<Entity> = world
    .query_filtered::<Entity, With<EditorTab>>()
    .iter(world)
    .collect();

  let file_entries: Vec<Entity> = world
    .query_filtered::<Entity, With<FileEntry>>()
    .iter(world)
    .collect();

  for entity in editor_tabs {
    world.despawn(entity);
  }

  for entity in file_entries {
    world.despawn(entity);
  }

  if let Some(mut explorer) = world.get_resource_mut::<ExplorerState>() {
    explorer.roots.clear();
  }

  if let Some(mut left) = world.get_resource_mut::<LeftPanelResource>() {
    left.is_visible = true;
  }

  if let Some(mut right) = world.get_resource_mut::<RightPanelResource>() {
    right.is_visible = false;
  }

  if let Some(mut bottom) = world.get_resource_mut::<BottomPanelResource>() {
    bottom.is_visible = false;
  }

  if let Some(mut counter) = world.get_resource_mut::<TabOrderCounter>() {
    counter.reset();
  }

  let order = world
    .get_resource_mut::<TabOrderCounter>()
    .map(|mut counter| counter.next())
    .unwrap_or(0);

  world.spawn((
    Tab::new("playground-1", order),
    PlaygroundTab,
    SonarAnimation::default(),
    TextBuffer::empty(),
    Cursor::new(0),
    Active,
    Focusable,
    KeyboardHandler::text_editor(),
  ));
}
