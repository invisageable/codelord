//! Tabbar ECS systems.

use super::components::{EditorTab, PlaygroundTab, Tab};
use super::resources::{
  PanelSnapshot, UnsavedChangesDialog, ZoomState, ZoomTransition,
};
use crate::animation::components::DeltaTime;
use crate::animation::resources::ActiveAnimations;
use crate::events::{
  CloseAllTabsRequest, CloseOtherTabsRequest, CloseTabRequest,
  CloseTabsToRightRequest, NavigateNextTabRequest, NavigatePrevTabRequest,
  OpenPdfPreviewRequest, ToggleZoomRequest,
};
use crate::panel::resources::{
  BottomPanelResource, LeftPanelResource, RightPanelResource,
};
use crate::previews::{PdfPreviewState, SqlitePreviewState, XlsPreviewState};
use crate::terminal::TerminalTab;
use crate::text_editor::components::FileTab;
use crate::ui::component::{Active, Modified};

use bevy_ecs::prelude::*;

/// System to handle close tab requests for editor tabs.
/// If tab has unsaved changes, shows dialog instead of closing.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn close_editor_tab_system(
  mut commands: Commands,
  requests: Query<(Entity, &CloseTabRequest)>,
  tabs: Query<(Entity, &Tab, Has<Active>, Has<Modified>), With<EditorTab>>,
  file_tabs: Query<&FileTab>,
  mut unsaved_dialog: ResMut<UnsavedChangesDialog>,
  mut sqlite_preview: ResMut<SqlitePreviewState>,
  mut pdf_preview: ResMut<PdfPreviewState>,
  mut xls_preview: ResMut<XlsPreviewState>,
) {
  for (request_entity, request) in requests.iter() {
    let tab_entity = request.entity;

    // Check if this is an editor tab
    let Some((_, tab, is_active, is_modified)) =
      tabs.iter().find(|(e, _, _, _)| *e == tab_entity)
    else {
      continue; // Not an editor tab, let another system handle it
    };

    // Check if closable
    if !tab.closable {
      commands.entity(request_entity).despawn();
      continue;
    }

    // Check if tab has unsaved changes
    if is_modified {
      // Show unsaved changes dialog instead of closing
      unsaved_dialog.show(tab_entity, &tab.label);
      commands.entity(request_entity).despawn();
      continue;
    }

    let tab_order = tab.order;

    // If closing active tab, activate another and handle preview state
    if is_active {
      let other_tabs = tabs
        .iter()
        .filter(|(e, _, _, _)| *e != tab_entity)
        .map(|(e, tab, _, _)| (e, tab.order));

      if let Some(next_entity) = find_next_tab(other_tabs, tab_order) {
        commands.entity(next_entity).insert(Active);

        // Handle preview state for the next tab
        if let Ok(next_file_tab) = file_tabs.get(next_entity) {
          if next_file_tab.is_sqlite() {
            sqlite_preview.enabled = true;
            sqlite_preview.current_file = Some(next_file_tab.path.clone());
            sqlite_preview.needs_reload = true;
            pdf_preview.disable();
            xls_preview.close();
          } else if next_file_tab.is_pdf() {
            sqlite_preview.close();
            // Spawn request so open_pdf_preview_system handles connection
            commands.spawn(OpenPdfPreviewRequest(next_file_tab.path.clone()));
            xls_preview.close();
          } else if next_file_tab.is_xls() {
            sqlite_preview.close();
            pdf_preview.disable();
            xls_preview.open(next_file_tab.path.clone());
          } else {
            sqlite_preview.close();
            pdf_preview.disable();
            xls_preview.close();
          }
        } else {
          // Next tab has no FileTab (untitled)
          sqlite_preview.close();
          pdf_preview.disable();
          xls_preview.close();
        }
      } else {
        // No more tabs, close all previews
        sqlite_preview.close();
        pdf_preview.close();
        xls_preview.close();
      }
    }

    // Despawn tab and request
    commands.entity(tab_entity).despawn();
    commands.entity(request_entity).despawn();
  }
}

/// System to handle close tab requests for terminal tabs.
pub fn close_terminal_tab_system(
  mut commands: Commands,
  requests: Query<(Entity, &CloseTabRequest)>,
  tabs: Query<(Entity, &Tab, Has<Active>), With<TerminalTab>>,
) {
  for (request_entity, request) in requests.iter() {
    let tab_entity = request.entity;

    // Check if this is a terminal tab
    let Some((_, tab, is_active)) =
      tabs.iter().find(|(e, _, _)| *e == tab_entity)
    else {
      continue; // Not a terminal tab, let another system handle it
    };

    // Check if closable
    if !tab.closable {
      commands.entity(request_entity).despawn();
      continue;
    }

    let tab_order = tab.order;

    // If closing active tab, activate another
    if is_active {
      let other_tabs = tabs
        .iter()
        .filter(|(e, _, _)| *e != tab_entity)
        .map(|(e, tab, _)| (e, tab.order));
      if let Some(next_entity) = find_next_tab(other_tabs, tab_order) {
        commands.entity(next_entity).insert(Active);
      }
    }

    // Despawn tab and request
    commands.entity(tab_entity).despawn();
    commands.entity(request_entity).despawn();
  }
}

/// System to handle close tab requests for playground tabs.
/// Prevents closing the last tab - playground always needs at least one tab.
pub fn close_playground_tab_system(
  mut commands: Commands,
  requests: Query<(Entity, &CloseTabRequest)>,
  tabs: Query<(Entity, &Tab, Has<Active>), With<PlaygroundTab>>,
) {
  let tab_count = tabs.iter().count();

  for (request_entity, request) in requests.iter() {
    let tab_entity = request.entity;

    let Some((_, tab, is_active)) =
      tabs.iter().find(|(e, _, _)| *e == tab_entity)
    else {
      continue;
    };

    // Don't close the last playground tab.
    if tab_count <= 1 {
      commands.entity(request_entity).despawn();
      continue;
    }

    if !tab.closable {
      commands.entity(request_entity).despawn();
      continue;
    }

    let tab_order = tab.order;

    if is_active {
      let other_tabs = tabs
        .iter()
        .filter(|(e, _, _)| *e != tab_entity)
        .map(|(e, tab, _)| (e, tab.order));
      if let Some(next_entity) = find_next_tab(other_tabs, tab_order) {
        commands.entity(next_entity).insert(Active);
      }
    }

    commands.entity(tab_entity).despawn();
    commands.entity(request_entity).despawn();
  }
}

/// Find the next tab to activate when closing the active tab.
/// Prefers the previous tab (lower order), falls back to next tab.
fn find_next_tab(
  other_tabs_data: impl Iterator<Item = (Entity, u32)>,
  closing_order: u32,
) -> Option<Entity> {
  let mut other_tabs: Vec<(Entity, u32)> = other_tabs_data.collect();

  if other_tabs.is_empty() {
    return None;
  }

  other_tabs.sort_by_key(|(_, order)| *order);

  // Prefer previous tab, then next tab, then first available
  let prev = other_tabs
    .iter()
    .rfind(|(_, order)| *order < closing_order)
    .map(|(e, _)| *e);

  let next = other_tabs
    .iter()
    .find(|(_, order)| *order > closing_order)
    .map(|(e, _)| *e);

  prev.or(next).or(other_tabs.first().map(|(e, _)| *e))
}

/// System to navigate to previous editor tab.
pub fn navigate_prev_editor_tab_system(
  mut commands: Commands,
  requests: Query<Entity, With<NavigatePrevTabRequest>>,
  tabs: Query<(Entity, &Tab, Has<Active>), With<EditorTab>>,
) {
  for request_entity in requests.iter() {
    // Find active tab
    if let Some((_, active_tab, _)) =
      tabs.iter().find(|(_, _, is_active)| *is_active)
    {
      let active_order = active_tab.order;

      // Find previous tab (highest order less than current)
      let prev_tab = tabs
        .iter()
        .filter(|(_, tab, _)| tab.order < active_order)
        .max_by_key(|(_, tab, _)| tab.order)
        .map(|(e, _, _)| e);

      if let Some(prev_entity) = prev_tab {
        // Deactivate all, activate previous
        for (e, _, is_active) in tabs.iter() {
          if is_active {
            commands.entity(e).remove::<Active>();
          }
        }
        commands.entity(prev_entity).insert(Active);
      }
    }

    commands.entity(request_entity).despawn();
  }
}

/// System to navigate to next editor tab.
pub fn navigate_next_editor_tab_system(
  mut commands: Commands,
  requests: Query<Entity, With<NavigateNextTabRequest>>,
  tabs: Query<(Entity, &Tab, Has<Active>), With<EditorTab>>,
) {
  for request_entity in requests.iter() {
    // Find active tab
    if let Some((_, active_tab, _)) =
      tabs.iter().find(|(_, _, is_active)| *is_active)
    {
      let active_order = active_tab.order;

      // Find next tab (lowest order greater than current)
      let next_tab = tabs
        .iter()
        .filter(|(_, tab, _)| tab.order > active_order)
        .min_by_key(|(_, tab, _)| tab.order)
        .map(|(e, _, _)| e);

      if let Some(next_entity) = next_tab {
        // Deactivate all, activate next
        for (e, _, is_active) in tabs.iter() {
          if is_active {
            commands.entity(e).remove::<Active>();
          }
        }
        commands.entity(next_entity).insert(Active);
      }
    }

    commands.entity(request_entity).despawn();
  }
}

/// System to navigate to previous terminal tab.
pub fn navigate_prev_terminal_tab_system(
  mut commands: Commands,
  requests: Query<Entity, With<NavigatePrevTabRequest>>,
  tabs: Query<(Entity, &Tab, Has<Active>), With<TerminalTab>>,
) {
  // Only process if there are terminal tabs and no editor tabs handled this
  if tabs.is_empty() {
    return;
  }

  for _request_entity in requests.iter() {
    if let Some((_, active_tab, _)) =
      tabs.iter().find(|(_, _, is_active)| *is_active)
    {
      let active_order = active_tab.order;

      let prev_tab = tabs
        .iter()
        .filter(|(_, tab, _)| tab.order < active_order)
        .max_by_key(|(_, tab, _)| tab.order)
        .map(|(e, _, _)| e);

      if let Some(prev_entity) = prev_tab {
        for (e, _, is_active) in tabs.iter() {
          if is_active {
            commands.entity(e).remove::<Active>();
          }
        }
        commands.entity(prev_entity).insert(Active);
      }
    }
    // Don't despawn here - editor system already did
  }
}

/// System to navigate to next terminal tab.
pub fn navigate_next_terminal_tab_system(
  mut commands: Commands,
  requests: Query<Entity, With<NavigateNextTabRequest>>,
  tabs: Query<(Entity, &Tab, Has<Active>), With<TerminalTab>>,
) {
  if tabs.is_empty() {
    return;
  }

  for _request_entity in requests.iter() {
    if let Some((_, active_tab, _)) =
      tabs.iter().find(|(_, _, is_active)| *is_active)
    {
      let active_order = active_tab.order;

      let next_tab = tabs
        .iter()
        .filter(|(_, tab, _)| tab.order > active_order)
        .min_by_key(|(_, tab, _)| tab.order)
        .map(|(e, _, _)| e);

      if let Some(next_entity) = next_tab {
        for (e, _, is_active) in tabs.iter() {
          if is_active {
            commands.entity(e).remove::<Active>();
          }
        }
        commands.entity(next_entity).insert(Active);
      }
    }
    // Don't despawn here - editor system already did
  }
}

/// System to handle ToggleZoomRequest.
/// Starts the zoom animation and captures/restores panel state.
pub fn zoom_toggle_system(
  mut commands: Commands,
  requests: Query<(Entity, &ToggleZoomRequest)>,
  mut zoom_state: ResMut<ZoomState>,
  mut left_panel: ResMut<LeftPanelResource>,
  mut right_panel: ResMut<RightPanelResource>,
  mut bottom_panel: ResMut<BottomPanelResource>,
  mut active_animations: ResMut<ActiveAnimations>,
) {
  for (event_entity, request) in requests.iter() {
    // Ignore if already animating
    if zoom_state.transition.is_some() {
      commands.entity(event_entity).despawn();
      continue;
    }

    let target_zoomed = !zoom_state.is_zoomed;

    // Start the animation and capture source
    zoom_state.transition = Some(ZoomTransition::new(target_zoomed));
    zoom_state.source = request.source;
    active_animations.increment();

    if target_zoomed {
      // Entering zoom mode: capture current panel state and close all
      zoom_state.pre_zoom_snapshot = Some(PanelSnapshot {
        left_panel: left_panel.is_visible,
        right_panel: right_panel.is_visible,
        bottom_panel: bottom_panel.is_visible,
      });

      // Close all visible panels immediately
      left_panel.is_visible = false;
      right_panel.is_visible = false;
      bottom_panel.is_visible = false;
    } else {
      // Exiting zoom mode: restore panels from snapshot immediately
      if let Some(snapshot) = zoom_state.pre_zoom_snapshot.take() {
        if snapshot.left_panel {
          left_panel.is_visible = true;
        }
        if snapshot.right_panel {
          right_panel.is_visible = true;
        }
        if snapshot.bottom_panel {
          bottom_panel.is_visible = true;
        }
      }
    }

    commands.entity(event_entity).despawn();
  }
}

/// System to update the zoom animation.
/// Computes progress, eased_progress, and animated_margin each frame.
pub fn zoom_animation_system(
  time: Res<DeltaTime>,
  mut zoom_state: ResMut<ZoomState>,
  mut active_animations: ResMut<ActiveAnimations>,
) {
  if let Some(ref mut transition) = zoom_state.transition {
    // Update elapsed time and raw progress
    transition.elapsed += time.delta();
    transition.progress = (transition.elapsed / transition.duration).min(1.0);

    // Compute eased progress (InOutCubic)
    let t = transition.progress;
    transition.eased_progress = if t < 0.5 {
      4.0 * t * t * t
    } else {
      1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    };

    // Compute animated margin
    transition.animated_margin = if transition.target_zoomed {
      // Zooming in: 0.0 -> 4.0
      transition.eased_progress * 4.0
    } else {
      // Zooming out: 4.0 -> 0.0
      (1.0 - transition.eased_progress) * 4.0
    };

    // Check if animation completed
    if transition.progress >= 1.0 {
      zoom_state.is_zoomed = transition.target_zoomed;
      zoom_state.transition = None;
      active_animations.decrement();
    }
  }
}

/// System to handle close all tabs request for editor tabs.
/// Closes all editor tabs except unclosable ones.
pub fn close_all_editor_tabs_system(
  mut commands: Commands,
  requests: Query<Entity, With<CloseAllTabsRequest>>,
  tabs: Query<(Entity, &Tab, Has<Modified>), With<EditorTab>>,
  mut unsaved_dialog: ResMut<UnsavedChangesDialog>,
  mut sqlite_preview: ResMut<SqlitePreviewState>,
  mut pdf_preview: ResMut<PdfPreviewState>,
  mut xls_preview: ResMut<XlsPreviewState>,
) {
  for request_entity in requests.iter() {
    // Find tabs that can be closed (closable and not modified)
    let mut closable_tabs: Vec<Entity> = Vec::new();
    let mut first_modified: Option<(Entity, String)> = None;

    for (entity, tab, is_modified) in tabs.iter() {
      if !tab.closable {
        continue;
      }

      if is_modified && first_modified.is_none() {
        first_modified = Some((entity, tab.label.clone()));
      } else if !is_modified {
        closable_tabs.push(entity);
      }
    }

    // Close all closable tabs
    for entity in closable_tabs {
      commands.entity(entity).despawn();
    }

    // Close all previews when closing all tabs
    sqlite_preview.close();
    pdf_preview.close();
    xls_preview.close();

    // Show unsaved dialog for first modified tab if any
    if let Some((entity, label)) = first_modified {
      unsaved_dialog.show(entity, label);
    }

    commands.entity(request_entity).despawn();
  }
}

/// System to handle close other tabs request for editor tabs.
/// Closes all editor tabs except the specified one.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn close_other_editor_tabs_system(
  mut commands: Commands,
  requests: Query<(Entity, &CloseOtherTabsRequest)>,
  tabs: Query<(Entity, &Tab, Has<Active>, Has<Modified>), With<EditorTab>>,
  file_tabs: Query<&FileTab>,
  mut unsaved_dialog: ResMut<UnsavedChangesDialog>,
  mut sqlite_preview: ResMut<SqlitePreviewState>,
  mut pdf_preview: ResMut<PdfPreviewState>,
  mut xls_preview: ResMut<XlsPreviewState>,
) {
  for (request_entity, request) in requests.iter() {
    let keep_entity = request.entity;

    // Find tabs that can be closed
    let mut closable_tabs: Vec<Entity> = Vec::new();
    let mut first_modified: Option<(Entity, String)> = None;

    for (entity, tab, _, is_modified) in tabs.iter() {
      if entity == keep_entity || !tab.closable {
        continue;
      }

      if is_modified && first_modified.is_none() {
        first_modified = Some((entity, tab.label.clone()));
      } else if !is_modified {
        closable_tabs.push(entity);
      }
    }

    // Close all closable tabs
    for entity in closable_tabs {
      commands.entity(entity).despawn();
    }

    // Activate the kept tab if it exists and isn't already active
    if let Some((_, _, is_active, _)) =
      tabs.iter().find(|(e, _, _, _)| *e == keep_entity)
      && !is_active
    {
      // Remove Active from all tabs first
      for (e, _, was_active, _) in tabs.iter() {
        if was_active {
          commands.entity(e).remove::<Active>();
        }
      }
      commands.entity(keep_entity).insert(Active);
    }

    // Handle preview state for the kept tab
    if let Ok(keep_file_tab) = file_tabs.get(keep_entity) {
      if keep_file_tab.is_sqlite() {
        sqlite_preview.enabled = true;
        sqlite_preview.current_file = Some(keep_file_tab.path.clone());
        sqlite_preview.needs_reload = true;
        pdf_preview.disable();
        xls_preview.close();
      } else if keep_file_tab.is_pdf() {
        sqlite_preview.close();
        // Spawn request so open_pdf_preview_system handles connection
        commands.spawn(OpenPdfPreviewRequest(keep_file_tab.path.clone()));
        xls_preview.close();
      } else if keep_file_tab.is_xls() {
        sqlite_preview.close();
        pdf_preview.disable();
        xls_preview.open(keep_file_tab.path.clone());
      } else {
        sqlite_preview.close();
        pdf_preview.disable();
        xls_preview.close();
      }
    } else {
      sqlite_preview.close();
      pdf_preview.disable();
      xls_preview.close();
    }

    // Show unsaved dialog for first modified tab if any
    if let Some((entity, label)) = first_modified {
      unsaved_dialog.show(entity, label);
    }

    commands.entity(request_entity).despawn();
  }
}

/// System to handle close tabs to right request for editor tabs.
/// Closes all editor tabs to the right of the specified one.
pub fn close_tabs_to_right_editor_system(
  mut commands: Commands,
  requests: Query<(Entity, &CloseTabsToRightRequest)>,
  tabs: Query<(Entity, &Tab, Has<Modified>), With<EditorTab>>,
  mut unsaved_dialog: ResMut<UnsavedChangesDialog>,
) {
  for (request_entity, request) in requests.iter() {
    let keep_entity = request.entity;

    // Get the order of the kept tab
    let Some((_, keep_tab, _)) =
      tabs.iter().find(|(e, _, _)| *e == keep_entity)
    else {
      commands.entity(request_entity).despawn();
      continue;
    };
    let keep_order = keep_tab.order;

    // Find tabs to the right that can be closed
    let mut closable_tabs = Vec::new();
    let mut first_modified: Option<(Entity, String)> = None;

    for (entity, tab, is_modified) in tabs.iter() {
      if tab.order <= keep_order || !tab.closable {
        continue;
      }

      if is_modified && first_modified.is_none() {
        first_modified = Some((entity, tab.label.clone()));
      } else if !is_modified {
        closable_tabs.push(entity);
      }
    }

    // Close all closable tabs
    for entity in closable_tabs {
      commands.entity(entity).despawn();
    }

    // Show unsaved dialog for first modified tab if any
    if let Some((entity, label)) = first_modified {
      unsaved_dialog.show(entity, label);
    }

    commands.entity(request_entity).despawn();
  }
}
