use crate::events::components::OpenFileRequest;
use crate::filescope::components::FilescopeItem;
use crate::filescope::resources::{
  FilescopeAction, FilescopeCommand, FilescopeMatcher, FilescopeResponse,
  FilescopeState,
};
use crate::navigation::resources::ExplorerState;

use bevy_ecs::message::MessageReader;
use bevy_ecs::system::{Res, ResMut};
use bevy_ecs::world::World;

use walkdir::WalkDir;

use std::path::PathBuf;

/// System to process filescope commands.
pub fn filescope_command_system(
  mut commands: MessageReader<FilescopeCommand>,
  mut state: ResMut<FilescopeState>,
  matcher: ResMut<FilescopeMatcher>,
) {
  for command in commands.read() {
    match &command.action {
      FilescopeAction::Show(mode) => {
        state.show(*mode);
      }
      FilescopeAction::Hide => {
        state.hide();
      }
      FilescopeAction::Toggle(mode) => {
        state.toggle(*mode);
      }
      FilescopeAction::UpdateQuery(query) => {
        state.set_query(query.clone());
        // Pattern will be updated in tick system.
      }
      FilescopeAction::SelectPrevious => {
        if let Some(m) = matcher.matcher.as_ref() {
          state.move_selection(-1, m.matched_count());
        }
      }
      FilescopeAction::SelectNext => {
        if let Some(m) = matcher.matcher.as_ref() {
          state.move_selection(1, m.matched_count());
        }
      }
      FilescopeAction::PageUp => {
        if let Some(m) = matcher.matcher.as_ref() {
          state.page_up(m.matched_count());
        }
      }
      FilescopeAction::PageDown => {
        if let Some(m) = matcher.matcher.as_ref() {
          state.page_down(m.matched_count());
        }
      }
      FilescopeAction::SelectFirst => {
        state.selection = 0;
      }
      FilescopeAction::SelectLast => {
        if let Some(m) = matcher.matcher.as_ref() {
          state.selection = m.matched_count().saturating_sub(1);
        }
      }
      FilescopeAction::TogglePreview => {
        state.show_preview = !state.show_preview;
      }
      FilescopeAction::Refresh => {
        // Will be handled by populate system.
      }
      FilescopeAction::Confirm(_) => {
        // Handled by UI layer.
      }
    }
  }
}

/// System to populate filescope with items when shown.
/// This spawns a background thread to avoid blocking the UI.
pub fn filescope_populate_system(
  mut state: ResMut<FilescopeState>,
  explorer: Res<ExplorerState>,
  mut matcher: ResMut<FilescopeMatcher>,
) {
  // Only populate when visible and not already populated.
  if !state.visible || state.populated {
    return;
  }

  // Mark as populated immediately to prevent re-entry.
  state.populated = true;

  // Get root paths from explorer.
  let root_paths: Vec<PathBuf> = explorer.roots.to_vec();

  if root_paths.is_empty() {
    return;
  }

  // Reset matcher.
  matcher.reset();

  let Some(m) = matcher.matcher.as_ref() else {
    return;
  };

  // Get injector for background thread.
  let injector = m.injector();
  let version = state.version.load(std::sync::atomic::Ordering::Acquire);
  let state_version = state.version.clone();

  // Spawn background thread to walk directories.
  std::thread::spawn(move || {
    const MAX_FILES: usize = 50_000;
    let mut file_count = 0;

    'outer: for root in &root_paths {
      let walker = WalkDir::new(root)
        .follow_links(false)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| {
          let name = e.file_name().to_str().unwrap_or("");

          !name.starts_with('.')
            && name != "node_modules"
            && name != "target"
            && name != "build"
            && name != "dist"
            && name != "__pycache__"
            && name != ".git"
        });

      for entry in walker.filter_map(|e| e.ok()) {
        // Check if picker was closed/reopened.
        if state_version.load(std::sync::atomic::Ordering::Acquire) != version {
          return;
        }

        let path = entry.path().to_path_buf();

        if path.is_dir() {
          continue;
        }

        let item = FilescopeItem::new_with_root(path, Some(root));
        let text = item.display_text();

        injector.push(item, |_, cols| {
          cols[0] = text.into();
        });

        file_count += 1;

        if file_count >= MAX_FILES {
          break 'outer;
        }
      }
    }
  });
}

/// System to tick the fuzzy matcher each frame.
pub fn filescope_tick_system(
  state: Res<FilescopeState>,
  mut matcher: ResMut<FilescopeMatcher>,
) {
  if !state.visible {
    return;
  }

  let Some(m) = matcher.get_mut() else {
    return;
  };

  // Update pattern if needed.
  m.set_pattern(&state.query.primary);

  // Tick matcher.
  m.tick();
}

/// Get selected file path from the picker.
pub fn get_selected_path(
  state: &FilescopeState,
  matcher: &FilescopeMatcher,
) -> Option<PathBuf> {
  matcher
    .get()
    .and_then(|m| m.get(state.selection))
    .map(|item| item.path.clone())
}

/// Handle filescope response and spawn appropriate events.
pub fn handle_filescope_response(
  world: &mut World,
  response: FilescopeResponse,
) {
  match response {
    FilescopeResponse::Select(path, _action) => {
      world.spawn(OpenFileRequest::new(path));
      world.resource_mut::<FilescopeState>().hide();
    }
    FilescopeResponse::Close => {
      world.resource_mut::<FilescopeState>().hide();
    }
    FilescopeResponse::None => {}
  }
}
