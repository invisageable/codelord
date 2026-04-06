use crate::events::ToggleBlameRequest;
use crate::git::components::TabBlame;
use crate::git::resources::{
  BlameResult, BranchResult, GitBlameSettings, GitBranchState,
  PendingBlameRequests, PendingBranchRequests, StatusResult,
};
use crate::navigation::resources::ActiveWorkspaceRoot;
use crate::settings::resources::{SettingItem, SettingsResource};
use crate::tabbar::components::EditorTab;
use crate::text_editor::components::FileTab;
use crate::ui::component::Active;

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{Changed, With};
use bevy_ecs::system::{Commands, Query, Res, ResMut};

use codelord_git::blame::{blame_file, find_repo_root};

/// System: spawns background threads to fetch blame data.
///
/// Runs when TabBlame is marked as enabled and not yet loaded.
pub fn fetch_blame_system(
  pending: Option<Res<PendingBlameRequests>>,
  mut tabs: Query<(Entity, &FileTab, &mut TabBlame), Changed<TabBlame>>,
) {
  let Some(pending) = pending else { return };

  for (entity, file_tab, mut blame) in tabs.iter_mut() {
    // Skip if not enabled, already loading, or already loaded
    if !blame.enabled || blame.loading || blame.blame.is_some() {
      continue;
    }

    // Find repo root if not cached
    if blame.repo_root.is_none() {
      blame.repo_root = find_repo_root(&file_tab.path);
    }

    let Some(repo_root) = blame.repo_root.clone() else {
      // Not a git repo
      continue;
    };

    blame.start_loading();

    // Spawn background thread to fetch blame
    let sender = pending.sender.clone();
    let file_path = file_tab.path.clone();

    std::thread::spawn(move || {
      let result = blame_file(&repo_root, &file_path);

      let _ = sender.send(BlameResult {
        entity,
        blame: result,
      });
    });
  }
}

/// System: polls for completed blame results and updates TabBlame.
pub fn poll_blame_results_system(
  pending: Option<Res<PendingBlameRequests>>,
  mut tabs: Query<&mut TabBlame>,
) {
  let Some(pending) = pending else { return };

  // Process all available results
  while let Ok(result) = pending.receiver.try_recv() {
    if let Ok(mut blame) = tabs.get_mut(result.entity) {
      if let Some(data) = result.blame {
        blame.set_blame(data);
      } else {
        blame.loading = false;
      }
    }
  }
}

/// System: clears blame when tab content changes.
pub fn invalidate_blame_on_edit_system(
  mut tabs: Query<
    &mut TabBlame,
    Changed<crate::text_editor::components::TextBuffer>,
  >,
) {
  for mut blame in tabs.iter_mut() {
    if blame.blame.is_some() {
      blame.clear();
    }
  }
}

/// System: toggles blame for the active editor tab.
pub fn toggle_blame_system(
  mut commands: Commands,
  requests: Query<Entity, With<ToggleBlameRequest>>,
  mut tabs: Query<&mut TabBlame, (With<EditorTab>, With<Active>)>,
) {
  for request_entity in requests.iter() {
    // Toggle blame on active tab
    for mut blame in tabs.iter_mut() {
      blame.toggle();
    }

    commands.entity(request_entity).despawn();
  }
}

/// System: syncs git blame settings from SettingsResource to GitBlameSettings.
///
/// Also updates all existing tabs when the global setting changes.
pub fn sync_blame_settings_system(
  settings: Option<Res<SettingsResource>>,
  mut blame_settings: ResMut<GitBlameSettings>,
  mut tabs: Query<&mut TabBlame>,
) {
  let Some(settings) = settings else { return };

  // EDITOR category is index 2 (after APP, APPEARANCE)
  let Some(editor_category) = settings.categories.get(2) else {
    return;
  };

  // Git Blame toggle is first item in EDITOR category
  let Some(SettingItem::Toggle { value: enabled, .. }) =
    editor_category.items.first()
  else {
    return;
  };

  // Min column selector is second item
  let min_column = editor_category
    .items
    .get(1)
    .and_then(|item| {
      if let SettingItem::Selector {
        options, selected, ..
      } = item
      {
        options.get(*selected).and_then(|s| s.parse().ok())
      } else {
        None
      }
    })
    .unwrap_or(60);

  // Update global settings if changed
  if blame_settings.enabled != *enabled
    || blame_settings.min_column != min_column
  {
    let was_enabled = blame_settings.enabled;
    blame_settings.enabled = *enabled;
    blame_settings.min_column = min_column;

    // If global enabled state changed, update ALL tabs
    if was_enabled != *enabled {
      for mut tab_blame in tabs.iter_mut() {
        tab_blame.enabled = *enabled;
      }
    }
  }
}

// ============================================================================
// Git Branch Detection Systems
// ============================================================================

/// System: detects git branch when active workspace changes.
///
/// Watches `ActiveWorkspaceRoot` for changes and spawns background
/// thread to read `.git/HEAD`.
///
/// Note: We don't clear the old branch value until the new one arrives
/// to avoid visual flickering in the UI.
pub fn detect_branch_system(
  active_workspace: Option<Res<ActiveWorkspaceRoot>>,
  mut branch_state: ResMut<GitBranchState>,
  pending: Option<Res<PendingBranchRequests>>,
) {
  let Some(active_workspace) = active_workspace else {
    return;
  };
  let Some(pending) = pending else { return };

  // Skip if already loading
  if branch_state.loading {
    return;
  }

  // Check if workspace changed
  if active_workspace.path != branch_state.workspace_path {
    if let Some(path) = &active_workspace.path {
      // Mark as loading to prevent duplicate spawns
      // Keep old branch value visible until new one arrives
      branch_state.loading = true;
      branch_state.workspace_path = Some(path.clone());

      // Spawn background thread to detect branch
      let sender = pending.branch_sender.clone();
      let workspace_path = path.clone();

      std::thread::spawn(move || {
        let branch = codelord_git::detect_branch(&workspace_path);
        let is_detached = codelord_git::is_detached_head(&workspace_path);

        let _ = sender.send(BranchResult {
          workspace_path,
          branch,
          is_detached,
        });
      });
    } else {
      // No workspace selected - clear everything
      branch_state.branch = None;
      branch_state.workspace_path = None;
      branch_state.is_detached = false;
      branch_state.loading = false;
    }
  }
}

/// System: polls for completed branch detection results.
pub fn poll_branch_results_system(
  pending: Option<Res<PendingBranchRequests>>,
  mut branch_state: ResMut<GitBranchState>,
) {
  let Some(pending) = pending else { return };

  while let Ok(result) = pending.branch_receiver.try_recv() {
    // Only update if this result is for the current workspace
    if branch_state.workspace_path.as_ref() == Some(&result.workspace_path) {
      branch_state.branch = result.branch;
      branch_state.is_detached = result.is_detached;
    }
    // Clear loading flag regardless (result arrived)
    branch_state.loading = false;
  }
}

/// System: checks dirty status when branch is detected or workspace changes.
///
/// Spawns background thread to run `git status --porcelain`.
/// This runs alongside branch detection to keep dirty status in sync.
pub fn check_dirty_status_system(
  branch_state: Res<GitBranchState>,
  pending: Option<Res<PendingBranchRequests>>,
) {
  let Some(pending) = pending else { return };

  // Only check when we have a workspace and branch detection just completed
  // (loading was true, now false means result just arrived)
  if branch_state.is_changed()
    && !branch_state.loading
    && let Some(path) = &branch_state.workspace_path
  {
    let sender = pending.status_sender.clone();
    let workspace_path = path.clone();

    std::thread::spawn(move || {
      let is_dirty = codelord_git::check_git_dirty(&workspace_path);

      let _ = sender.send(StatusResult {
        workspace_path,
        is_dirty,
      });
    });
  }
}

/// System: polls for completed dirty status results.
pub fn poll_status_results_system(
  pending: Option<Res<PendingBranchRequests>>,
  mut branch_state: ResMut<GitBranchState>,
) {
  let Some(pending) = pending else { return };

  while let Ok(result) = pending.status_receiver.try_recv() {
    // Only update if this result is for the current workspace
    if branch_state.workspace_path.as_ref() == Some(&result.workspace_path) {
      branch_state.is_dirty = result.is_dirty;
    }
  }
}
