use crate::button::components::{Button, ButtonContent};
use crate::dialog;
use crate::events::{
  AddRootRequest, CollapseFolderRequest, CreateFileRequest,
  CreateFolderRequest, DeleteRequest, ExpandFolderRequest, FolderSelectedEvent,
  PasteRequest, RemoveRootRequest, RenameRequest,
};

use crate::events::AddFolderToWorkspaceDialogRequest;
use crate::icon::components::{Icon, Structure};
use crate::navigation::components::{
  Expanded, FileEntry, FileEntryBundle, Selected,
};
use crate::navigation::resources::{
  ActiveWorkspaceRoot, BreadcrumbData, BreadcrumbSegment, ExplorerState,
  FileClipboard, PendingFolderDialog, PendingWorkspaceFolderDialog,
};
use crate::runtime::RuntimeHandle;
use crate::symbol::TabSymbols;
use crate::tabbar::components::EditorTab;
use crate::text_editor::components::{Cursor, FileTab, TextBuffer};
use crate::ui::component::Active;

use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query, Res, ResMut};

use std::fs;
use std::path::{Path, PathBuf};

/// System to scan directory and spawn file entries.
/// Handles multiple workspace roots.
pub fn scan_directory_system(
  mut commands: Commands,
  explorer_state: Res<ExplorerState>,
  existing_entries: Query<&FileEntry>,
) {
  // Check if we already have entries (avoid re-scanning every frame)
  if existing_entries.iter().next().is_some() {
    return;
  }

  // No roots to scan
  if explorer_state.roots.is_empty() {
    return;
  }

  // Spawn entries for each root
  for root in &explorer_state.roots {
    spawn_root_entry(&mut commands, root);
  }
}

/// Spawn a root folder entry and scan its contents.
fn spawn_root_entry(commands: &mut Commands, root: &Path) {
  let root_name = root
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_default();
  let root_label: &'static str = Box::leak(root_name.into_boxed_str());

  commands.spawn((
    FileEntry::new(root.to_path_buf(), None, 0),
    Button {
      content: ButtonContent::IconLabel(
        Icon::Structure(Structure::FolderOpen),
        root_label,
      ),
      variant: crate::button::components::ButtonVariant::Ghost,
    },
    crate::ui::component::Clickable::default(),
    Expanded,
  ));

  // Scan root directory contents at depth 1
  scan_directory(commands, root, Some(root.to_path_buf()), 1);
}

/// Scan a directory and spawn file entries.
pub fn scan_directory(
  commands: &mut Commands,
  path: &Path,
  parent: Option<PathBuf>,
  depth: u32,
) {
  let Ok(entries) = fs::read_dir(path) else {
    return;
  };

  let mut paths: Vec<_> = entries
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| {
      // Skip hidden files
      p.file_name()
        .map(|n| !n.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
    })
    .collect();

  // Sort: directories first, then alphabetically by name
  paths.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
    (true, false) => std::cmp::Ordering::Less,
    (false, true) => std::cmp::Ordering::Greater,
    _ => {
      let a_name = a.file_name().map(|n| n.to_ascii_lowercase());
      let b_name = b.file_name().map(|n| n.to_ascii_lowercase());
      a_name.cmp(&b_name)
    }
  });

  for entry_path in paths {
    commands.spawn(FileEntryBundle::new(entry_path, parent.clone(), depth));
  }
}

/// System to poll pending folder dialog for results.
pub fn poll_folder_dialog_system(
  mut commands: Commands,
  pending: Option<Res<PendingFolderDialog>>,
) {
  let Some(pending) = pending else {
    return;
  };

  match pending.0.try_recv() {
    Ok(Some(path)) => {
      commands.spawn(FolderSelectedEvent::new(path));
      commands.remove_resource::<PendingFolderDialog>();
    }
    Ok(None) => {
      commands.remove_resource::<PendingFolderDialog>();
    }
    Err(flume::TryRecvError::Empty) => {}
    Err(flume::TryRecvError::Disconnected) => {
      commands.remove_resource::<PendingFolderDialog>();
    }
  }
}

/// System to handle AddFolderToWorkspaceDialogRequest events.
/// Opens a folder picker dialog to add a new root to the workspace.
pub fn add_folder_to_workspace_dialog_system(
  mut commands: Commands,
  requests: Query<(Entity, &AddFolderToWorkspaceDialogRequest)>,
  pending: Option<Res<PendingWorkspaceFolderDialog>>,
  runtime: Option<Res<RuntimeHandle>>,
) {
  // Only process if no dialog is pending and runtime is available
  if pending.is_some() {
    return;
  }

  let Some(runtime) = runtime else {
    return;
  };

  for (event_entity, _) in requests.iter() {
    let rx = dialog::pick_folder(&runtime);
    commands.insert_resource(PendingWorkspaceFolderDialog(rx));
    commands.entity(event_entity).despawn();
  }
}

/// System to poll pending workspace folder dialog for results.
pub fn poll_workspace_folder_dialog_system(
  mut commands: Commands,
  pending: Option<Res<PendingWorkspaceFolderDialog>>,
) {
  let Some(pending) = pending else {
    return;
  };

  match pending.0.try_recv() {
    Ok(Some(path)) => {
      commands.spawn(AddRootRequest::new(path));
      commands.remove_resource::<PendingWorkspaceFolderDialog>();
    }
    Ok(None) => {
      commands.remove_resource::<PendingWorkspaceFolderDialog>();
    }
    Err(flume::TryRecvError::Empty) => {}
    Err(flume::TryRecvError::Disconnected) => {
      commands.remove_resource::<PendingWorkspaceFolderDialog>();
    }
  }
}

/// System to handle FolderSelectedEvent.
/// Sets explorer root - scan_directory_system will spawn entries.
pub fn folder_selected_system(
  mut commands: Commands,
  events: Query<(Entity, &FolderSelectedEvent)>,
  mut explorer_state: ResMut<ExplorerState>,
  mut active_workspace: ResMut<ActiveWorkspaceRoot>,
  existing_entries: Query<Entity, With<FileEntry>>,
) {
  for (entity, event) in events.iter() {
    // Clear existing entries when selecting a new primary folder
    for entry_entity in existing_entries.iter() {
      commands.entity(entry_entity).despawn();
    }

    // Set as the only root (replaces existing roots)
    explorer_state.roots = vec![event.path.clone()];

    // Update active workspace to the selected folder
    let name = event
      .path
      .file_name()
      .map(|n| n.to_string_lossy().to_string());
    active_workspace.path = Some(event.path.clone());
    active_workspace.name = name;

    // Despawn event
    commands.entity(entity).despawn();
  }
}

/// System to handle ExpandFolderRequest events.
/// Adds Expanded marker, updates icon, and spawns children.
pub fn expand_folder_system(
  mut commands: Commands,
  requests: Query<(Entity, &ExpandFolderRequest)>,
  mut buttons: Query<&mut Button>,
) {
  for (event_entity, request) in requests.iter() {
    // Add Expanded marker to folder entity
    commands.entity(request.entity).insert(Expanded);

    // Update icon to FolderOpen
    if let Ok(mut btn) = buttons.get_mut(request.entity)
      && let ButtonContent::IconLabel(_, label) = btn.content
    {
      btn.content =
        ButtonContent::IconLabel(Icon::Structure(Structure::FolderOpen), label);
    }

    // Spawn children entries
    scan_directory(
      &mut commands,
      &request.path,
      Some(request.path.clone()),
      request.depth + 1,
    );

    // Despawn event
    commands.entity(event_entity).despawn();
  }
}

/// System to handle CollapseFolderRequest events.
/// Removes Expanded marker, updates icon, and despawns descendants.
pub fn collapse_folder_system(
  mut commands: Commands,
  requests: Query<(Entity, &CollapseFolderRequest)>,
  mut buttons: Query<&mut Button>,
  file_entries: Query<(Entity, &FileEntry)>,
) {
  for (event_entity, request) in requests.iter() {
    // Remove Expanded marker from folder entity
    commands.entity(request.entity).remove::<Expanded>();

    // Update icon to FolderClose
    if let Ok(mut btn) = buttons.get_mut(request.entity)
      && let ButtonContent::IconLabel(_, label) = btn.content
    {
      btn.content = ButtonContent::IconLabel(
        Icon::Structure(Structure::FolderClose),
        label,
      );
    }

    // Despawn all descendants recursively
    despawn_descendants(&mut commands, &file_entries, &request.path);

    // Despawn event
    commands.entity(event_entity).despawn();
  }
}

/// Recursively despawn all descendants of a folder.
fn despawn_descendants(
  commands: &mut Commands,
  file_entries: &Query<(Entity, &FileEntry)>,
  parent_path: &PathBuf,
) {
  // Collect descendants first to avoid borrow issues
  let descendants: Vec<(Entity, PathBuf, bool)> = file_entries
    .iter()
    .filter(|(_, e)| e.parent.as_ref() == Some(parent_path))
    .map(|(e, entry)| (e, entry.path.clone(), entry.is_dir))
    .collect();

  for (entity, path, is_dir) in descendants {
    if is_dir {
      despawn_descendants(commands, file_entries, &path);
    }
    commands.entity(entity).despawn();
  }
}

/// System to update breadcrumb data when active tab changes.
/// Includes both file path segments and symbol segments based on cursor.
#[allow(clippy::type_complexity)]
pub fn update_breadcrumbs_system(
  active_tab: Query<
    (&FileTab, &TextBuffer, &Cursor, Option<&TabSymbols>),
    (With<EditorTab>, With<Active>),
  >,
  explorer: Res<ExplorerState>,
  mut breadcrumbs: ResMut<BreadcrumbData>,
) {
  // Get active tab data
  let tab_data = active_tab.iter().next();

  // Clear and rebuild segments
  breadcrumbs.segments.clear();

  let Some((file_tab, buffer, cursor, symbols)) = tab_data else {
    return;
  };

  let path = &file_tab.path;

  // Try to make path relative to any workspace root
  let mut path_added = false;
  for root in &explorer.roots {
    if let Ok(relative) = path.strip_prefix(root) {
      // Add root folder name first
      if let Some(root_name) = root.file_name() {
        breadcrumbs
          .segments
          .push(BreadcrumbSegment::path(root_name.to_string_lossy(), false));
      }

      // Add relative path components
      let components: Vec<_> = relative.components().collect();
      let len = components.len();

      for (i, component) in components.iter().enumerate() {
        if let Some(text) = component.as_os_str().to_str() {
          breadcrumbs
            .segments
            .push(BreadcrumbSegment::path(text, i == len - 1));
        }
      }

      path_added = true;
      break;
    }
  }

  // No workspace root or path not relative - show full path
  if !path_added {
    let components: Vec<_> = path.components().collect();
    let len = components.len();

    for (i, component) in components.iter().enumerate() {
      if let Some(text) = component.as_os_str().to_str() {
        breadcrumbs
          .segments
          .push(BreadcrumbSegment::path(text, i == len - 1));
      }
    }
  }

  // Add symbol segments based on cursor position
  if let Some(tab_symbols) = symbols {
    // Get cursor line from position
    let (cursor_line, _) = buffer.char_to_line_col(cursor.position);

    // Find symbols containing the cursor
    let containing_symbols = tab_symbols.map.find_containing(cursor_line);

    // Add symbol segments (outermost to innermost)
    for symbol in containing_symbols {
      breadcrumbs.segments.push(BreadcrumbSegment::symbol(
        symbol.display_text.clone(),
        symbol.kind,
        symbol.byte_range.clone(),
        symbol.highlight_ranges.clone(),
      ));
    }
  }
}

// ============================================================================
// Workspace Root Systems
// ============================================================================

/// System to handle AddRootRequest events.
/// Adds a new root to the workspace without clearing existing roots.
pub fn add_root_system(
  mut commands: Commands,
  requests: Query<(Entity, &AddRootRequest)>,
  mut explorer_state: ResMut<ExplorerState>,
) {
  for (event_entity, request) in requests.iter() {
    // Add to workspace roots
    let path = request.path.clone();
    if !explorer_state.is_root(&path) {
      explorer_state.add_root(path.clone());

      // Spawn the new root entry
      spawn_root_entry(&mut commands, &path);

      log::info!("[Explorer] Added root to workspace: {}", path.display());
    }

    commands.entity(event_entity).despawn();
  }
}

/// System to handle RemoveRootRequest events.
/// Removes a root from the workspace (only if multiple roots exist).
pub fn remove_root_system(
  mut commands: Commands,
  requests: Query<(Entity, &RemoveRootRequest)>,
  mut explorer_state: ResMut<ExplorerState>,
  file_entries: Query<(Entity, &FileEntry)>,
) {
  for (event_entity, request) in requests.iter() {
    let path = &request.path;

    // Only remove if we have multiple roots
    if explorer_state.remove_root(path) {
      // Despawn the root entry and all its descendants
      file_entries
        .iter()
        .filter(|(_, e)| e.path == *path)
        .for_each(|(entity, _)| {
          despawn_descendants(&mut commands, &file_entries, path);
          commands.entity(entity).despawn();
        });

      log::info!("[Explorer] Removed root from workspace: {}", path.display());
    } else {
      log::warn!(
        "[Explorer] Cannot remove last root from workspace: {}",
        path.display()
      );
    }

    commands.entity(event_entity).despawn();
  }
}

// ============================================================================
// File Operation Systems
// ============================================================================

/// System to handle CreateFileRequest events.
pub fn create_file_system(
  mut commands: Commands,
  requests: Query<(Entity, &CreateFileRequest)>,
  file_entries: Query<(Entity, &FileEntry)>,
) {
  for (event_entity, request) in requests.iter() {
    let file_path = request.parent_path.join(&request.name);

    // Create the file
    match fs::File::create(&file_path) {
      Ok(_) => {
        log::info!("[Explorer] Created file: {}", file_path.display());

        // Find parent entity's depth
        let parent_depth = file_entries
          .iter()
          .find(|(_, e)| e.path == request.parent_path)
          .map(|(_, e)| e.depth)
          .unwrap_or(0);

        // Spawn file entry
        commands.spawn(FileEntryBundle::new(
          file_path,
          Some(request.parent_path.clone()),
          parent_depth + 1,
        ));
      }
      Err(e) => {
        log::error!("[Explorer] Failed to create file: {e}");
      }
    }

    commands.entity(event_entity).despawn();
  }
}

/// System to handle CreateFolderRequest events.
pub fn create_folder_system(
  mut commands: Commands,
  requests: Query<(Entity, &CreateFolderRequest)>,
  file_entries: Query<(Entity, &FileEntry)>,
) {
  for (event_entity, request) in requests.iter() {
    let folder_path = request.parent_path.join(&request.name);

    // Create the folder
    match fs::create_dir(&folder_path) {
      Ok(_) => {
        log::info!("[Explorer] Created folder: {}", folder_path.display());

        // Find parent entity's depth
        let parent_depth = file_entries
          .iter()
          .find(|(_, e)| e.path == request.parent_path)
          .map(|(_, e)| e.depth)
          .unwrap_or(0);

        // Spawn folder entry
        commands.spawn(FileEntryBundle::new(
          folder_path,
          Some(request.parent_path.clone()),
          parent_depth + 1,
        ));
      }
      Err(e) => {
        log::error!("[Explorer] Failed to create folder: {e}");
      }
    }

    commands.entity(event_entity).despawn();
  }
}

/// System to handle RenameRequest events.
pub fn rename_system(
  mut commands: Commands,
  requests: Query<(Entity, &RenameRequest)>,
  mut file_entries: Query<&mut FileEntry>,
  mut buttons: Query<&mut Button>,
) {
  for (event_entity, request) in requests.iter() {
    let new_path = request
      .old_path
      .parent()
      .map(|p| p.join(&request.new_name))
      .unwrap_or_else(|| PathBuf::from(&request.new_name));

    // Rename the file/folder on disk
    match fs::rename(&request.old_path, &new_path) {
      Ok(_) => {
        log::info!(
          "[Explorer] Renamed: {} -> {}",
          request.old_path.display(),
          new_path.display()
        );

        // Update the FileEntry component
        if let Ok(mut entry) = file_entries.get_mut(request.entity) {
          entry.path = new_path.clone();
        }

        // Update the button label
        if let Ok(mut btn) = buttons.get_mut(request.entity) {
          let label: &'static str =
            Box::leak(request.new_name.clone().into_boxed_str());
          if let ButtonContent::IconLabel(icon, _) = btn.content {
            btn.content = ButtonContent::IconLabel(icon, label);
          }
        }
      }
      Err(e) => {
        log::error!("[Explorer] Failed to rename: {e}");
      }
    }

    commands.entity(event_entity).despawn();
  }
}

/// System to handle DeleteRequest events.
pub fn delete_system(
  mut commands: Commands,
  requests: Query<(Entity, &DeleteRequest)>,
  file_entries: Query<(Entity, &FileEntry)>,
) {
  for (event_entity, request) in requests.iter() {
    let result = if request.is_dir {
      fs::remove_dir_all(&request.path)
    } else {
      fs::remove_file(&request.path)
    };

    match result {
      Ok(_) => {
        log::info!("[Explorer] Deleted: {}", request.path.display());

        // Despawn the entity and all descendants if it's a directory
        if request.is_dir {
          despawn_descendants(&mut commands, &file_entries, &request.path);
        }
        commands.entity(request.entity).despawn();
      }
      Err(e) => {
        log::error!("[Explorer] Failed to delete: {e}");
      }
    }

    commands.entity(event_entity).despawn();
  }
}

/// System to handle PasteRequest events.
pub fn paste_system(
  mut commands: Commands,
  requests: Query<(Entity, &PasteRequest)>,
  file_entries: Query<(Entity, &FileEntry)>,
  mut clipboard: ResMut<FileClipboard>,
) {
  for (event_entity, request) in requests.iter() {
    let source = &request.source;
    let dest_dir = &request.destination;

    // Get filename from source
    let file_name = source
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_default();

    let dest_path = dest_dir.join(&file_name);

    // Check if destination already exists
    if dest_path.exists() {
      log::error!(
        "[Explorer] Paste failed: {} already exists",
        dest_path.display()
      );
      commands.entity(event_entity).despawn();
      continue;
    }

    let result = if request.is_cut {
      // Move operation
      fs::rename(source, &dest_path)
    } else {
      // Copy operation
      if source.is_dir() {
        copy_dir_recursive(source, &dest_path)
      } else {
        fs::copy(source, &dest_path).map(|_| ())
      }
    };

    match result {
      Ok(_) => {
        let action = if request.is_cut { "Moved" } else { "Copied" };
        log::info!(
          "[Explorer] {}: {} -> {}",
          action,
          source.display(),
          dest_path.display()
        );

        // Find destination parent's depth
        let parent_depth = file_entries
          .iter()
          .find(|(_, e)| e.path == *dest_dir)
          .map(|(_, e)| e.depth)
          .unwrap_or(0);

        // Spawn new file entry
        commands.spawn(FileEntryBundle::new(
          dest_path,
          Some(dest_dir.clone()),
          parent_depth + 1,
        ));

        // If cut, despawn the source entity
        if request.is_cut
          && let Some((entity, _)) =
            file_entries.iter().find(|(_, e)| e.path == *source)
        {
          commands.entity(entity).despawn();
        }

        // Clear clipboard after cut operation
        if request.is_cut {
          clipboard.clear();
        }
      }
      Err(e) => {
        log::error!("[Explorer] Paste failed: {e}");
      }
    }

    commands.entity(event_entity).despawn();
  }
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
  fs::create_dir_all(dst)?;

  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let file_type = entry.file_type()?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());

    if file_type.is_dir() {
      copy_dir_recursive(&src_path, &dst_path)?;
    } else {
      fs::copy(&src_path, &dst_path)?;
    }
  }

  Ok(())
}

// ============================================================================
// Explorer Header Action Systems
// ============================================================================

/// System to handle RefreshExplorerRequest events.
/// Clears all file entries and re-scans the workspace roots.
pub fn refresh_explorer_system(
  mut commands: Commands,
  requests: Query<(Entity, &crate::events::RefreshExplorerRequest)>,
  existing_entries: Query<Entity, With<FileEntry>>,
  explorer_state: Res<ExplorerState>,
) {
  for (event_entity, _) in requests.iter() {
    // Despawn all existing file entries
    for entry_entity in existing_entries.iter() {
      commands.entity(entry_entity).despawn();
    }

    // Re-scan all roots
    for root in &explorer_state.roots {
      spawn_root_entry(&mut commands, root);
    }

    log::info!("[Explorer] Refreshed file tree");
    commands.entity(event_entity).despawn();
  }
}

/// System to handle CollapseAllFoldersRequest events.
/// Collapses all expanded folders in the explorer.
pub fn collapse_all_folders_system(
  mut commands: Commands,
  requests: Query<(Entity, &crate::events::CollapseAllFoldersRequest)>,
  expanded_folders: Query<(Entity, &FileEntry), With<Expanded>>,
  file_entries: Query<(Entity, &FileEntry)>,
  mut buttons: Query<&mut Button>,
  explorer_state: Res<ExplorerState>,
) {
  for (event_entity, _) in requests.iter() {
    // Collect all expanded folder entities (excluding workspace roots)
    let folders_to_collapse: Vec<(Entity, PathBuf)> = expanded_folders
      .iter()
      .filter(|(_, entry)| entry.is_dir && !explorer_state.is_root(&entry.path))
      .map(|(entity, entry)| (entity, entry.path.clone()))
      .collect();

    for (entity, path) in folders_to_collapse {
      // Remove Expanded marker
      commands.entity(entity).remove::<Expanded>();

      // Update icon to FolderClose
      if let Ok(mut btn) = buttons.get_mut(entity)
        && let ButtonContent::IconLabel(_, label) = btn.content
      {
        btn.content = ButtonContent::IconLabel(
          Icon::Structure(Structure::FolderClose),
          label,
        );
      }

      // Despawn all descendants
      despawn_descendants(&mut commands, &file_entries, &path);
    }

    log::info!("[Explorer] Collapsed all folders");
    commands.entity(event_entity).despawn();
  }
}

/// System to handle ToggleHiddenFilesRequest events.
/// Toggles visibility of hidden files (dotfiles) and refreshes the tree.
pub fn toggle_hidden_files_system(
  mut commands: Commands,
  requests: Query<(Entity, &crate::events::ToggleHiddenFilesRequest)>,
  existing_entries: Query<Entity, With<FileEntry>>,
  mut explorer_state: ResMut<ExplorerState>,
) {
  for (event_entity, _) in requests.iter() {
    // Toggle the show_hidden flag
    explorer_state.show_hidden = !explorer_state.show_hidden;

    // Despawn all existing file entries
    for entry_entity in existing_entries.iter() {
      commands.entity(entry_entity).despawn();
    }

    // Re-scan all roots (scan_directory will check show_hidden)
    let roots = explorer_state.roots.clone();
    let show_hidden = explorer_state.show_hidden;
    for root in &roots {
      spawn_root_entry_with_hidden(&mut commands, root, show_hidden);
    }

    log::info!(
      "[Explorer] Hidden files: {}",
      if explorer_state.show_hidden {
        "visible"
      } else {
        "hidden"
      }
    );
    commands.entity(event_entity).despawn();
  }
}

/// Spawn a root folder entry with hidden files visibility setting.
fn spawn_root_entry_with_hidden(
  commands: &mut Commands,
  root: &Path,
  show_hidden: bool,
) {
  let root_name = root
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_default();
  let root_label: &'static str = Box::leak(root_name.into_boxed_str());

  commands.spawn((
    FileEntry::new(root.to_path_buf(), None, 0),
    Button {
      content: ButtonContent::IconLabel(
        Icon::Structure(Structure::FolderOpen),
        root_label,
      ),
      variant: crate::button::components::ButtonVariant::Ghost,
    },
    crate::ui::component::Clickable::default(),
    Expanded,
  ));

  // Scan root directory contents at depth 1
  scan_directory_with_hidden(
    commands,
    root,
    Some(root.to_path_buf()),
    1,
    show_hidden,
  );
}

/// Scan a directory and spawn file entries, respecting hidden files visibility.
pub fn scan_directory_with_hidden(
  commands: &mut Commands,
  path: &Path,
  parent: Option<PathBuf>,
  depth: u32,
  show_hidden: bool,
) {
  let Ok(entries) = fs::read_dir(path) else {
    return;
  };

  let mut paths: Vec<_> = entries
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| {
      // Filter hidden files based on setting
      if show_hidden {
        true
      } else {
        p.file_name()
          .map(|n| !n.to_string_lossy().starts_with('.'))
          .unwrap_or(false)
      }
    })
    .collect();

  // Sort: directories first, then alphabetically by name
  paths.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
    (true, false) => std::cmp::Ordering::Less,
    (false, true) => std::cmp::Ordering::Greater,
    _ => {
      let a_name = a.file_name().map(|n| n.to_ascii_lowercase());
      let b_name = b.file_name().map(|n| n.to_ascii_lowercase());

      a_name.cmp(&b_name)
    }
  });

  for entry_path in paths {
    commands.spawn(FileEntryBundle::new(entry_path, parent.clone(), depth));
  }
}

// ============================================================================
// Explorer Selection Sync System
// ============================================================================

/// System to sync explorer selection with the active editor tab.
/// When the active tab changes, highlights the corresponding file in the
/// explorer and updates the active workspace root.
pub fn sync_explorer_selection_system(
  mut commands: Commands,
  active_tab: Query<&FileTab, (With<EditorTab>, With<Active>)>,
  file_entries: Query<(Entity, &FileEntry)>,
  selected_entries: Query<Entity, With<Selected>>,
  explorer: Res<ExplorerState>,
  mut active_workspace: ResMut<ActiveWorkspaceRoot>,
) {
  // Get active file path
  let active_path = active_tab.iter().next().map(|ft| &ft.path);

  // Update active workspace root based on the active file
  if let Some(path) = active_path {
    active_workspace.update_from_path(path, &explorer);
  }

  // Find if any entry currently matches
  let matching_entry = active_path.and_then(|path| {
    file_entries
      .iter()
      .find(|(_, entry)| &entry.path == path)
      .map(|(entity, _)| entity)
  });

  // Check if already selected
  let already_selected = matching_entry
    .map(|entity| selected_entries.iter().any(|e| e == entity))
    .unwrap_or(false);

  // Only update if selection needs to change
  if !already_selected {
    // Remove Selected from all entries
    selected_entries.iter().for_each(|entity| {
      commands.entity(entity).remove::<Selected>();
    });

    // Add Selected to matching entry
    if let Some(entity) = matching_entry {
      commands.entity(entity).insert(Selected);
    }
  }
}
