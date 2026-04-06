use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

use std::path::PathBuf;

/// Event: request to open a file in the editor.
/// This is a "one-shot" entity - processed then despawned.
#[derive(Component, Debug, Clone)]
pub struct OpenFileRequest {
  pub path: PathBuf,
}

impl OpenFileRequest {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into() }
  }
}

/// Event: request to close a tab.
#[derive(Component, Debug, Clone)]
pub struct CloseTabRequest {
  pub entity: Entity,
}

impl CloseTabRequest {
  pub fn new(entity: Entity) -> Self {
    Self { entity }
  }
}

/// Event: request to activate a tab.
#[derive(Component, Debug, Clone)]
pub struct ActivateTabRequest {
  pub entity: Entity,
}

impl ActivateTabRequest {
  pub fn new(entity: Entity) -> Self {
    Self { entity }
  }
}

/// Event: request to save a file.
#[derive(Component, Debug, Clone)]
pub struct SaveFileRequest {
  pub entity: Entity,
}

impl SaveFileRequest {
  pub fn new(entity: Entity) -> Self {
    Self { entity }
  }
}

/// Event: request to open a folder dialog.
/// UI spawns this, system handles opening rfd dialog.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct OpenFolderDialogRequest;

/// Event: folder was selected from dialog.
/// Spawned by poll system when folder dialog returns a path.
#[derive(Component, Debug, Clone)]
pub struct FolderSelectedEvent {
  pub path: PathBuf,
}

impl FolderSelectedEvent {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into() }
  }
}

/// Event: request to expand a folder in the explorer.
/// UI spawns this when user clicks on a collapsed folder.
#[derive(Component, Debug, Clone)]
pub struct ExpandFolderRequest {
  pub entity: Entity,
  pub path: PathBuf,
  pub depth: u32,
}

impl ExpandFolderRequest {
  pub fn new(entity: Entity, path: impl Into<PathBuf>, depth: u32) -> Self {
    Self {
      entity,
      path: path.into(),
      depth,
    }
  }
}

/// Event: request to collapse a folder in the explorer.
/// UI spawns this when user clicks on an expanded folder.
#[derive(Component, Debug, Clone)]
pub struct CollapseFolderRequest {
  pub entity: Entity,
  pub path: PathBuf,
}

impl CollapseFolderRequest {
  pub fn new(entity: Entity, path: impl Into<PathBuf>) -> Self {
    Self {
      entity,
      path: path.into(),
    }
  }
}

// ============================================================================
// Text Editing Events
// ============================================================================

/// Event: insert text at cursor position.
#[derive(Component, Debug, Clone)]
pub struct InsertTextEvent {
  pub entity: Entity,
  pub text: String,
}

impl InsertTextEvent {
  pub fn new(entity: Entity, text: impl Into<String>) -> Self {
    Self {
      entity,
      text: text.into(),
    }
  }

  pub fn char(entity: Entity, ch: char) -> Self {
    Self {
      entity,
      text: ch.to_string(),
    }
  }
}

/// Event: delete text (backspace or delete key).
#[derive(Component, Debug, Clone, Copy)]
pub struct DeleteTextEvent {
  pub entity: Entity,
  pub direction: DeleteDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteDirection {
  /// Backspace - delete char before cursor.
  Before,
  /// Delete key - delete char after cursor.
  After,
}

impl DeleteTextEvent {
  pub fn backspace(entity: Entity) -> Self {
    Self {
      entity,
      direction: DeleteDirection::Before,
    }
  }

  pub fn delete(entity: Entity) -> Self {
    Self {
      entity,
      direction: DeleteDirection::After,
    }
  }
}

/// Event: move cursor.
#[derive(Component, Debug, Clone, Copy)]
pub struct MoveCursorEvent {
  pub entity: Entity,
  pub movement: CursorMovement,
  pub extend_selection: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorMovement {
  Left,
  Right,
  Up,
  Down,
  LineStart,
  LineEnd,
  BufferStart,
  BufferEnd,
  WordLeft,
  WordRight,
}

impl MoveCursorEvent {
  pub fn new(
    entity: Entity,
    movement: CursorMovement,
    extend_selection: bool,
  ) -> Self {
    Self {
      entity,
      movement,
      extend_selection,
    }
  }
}

/// Event: set cursor position directly (e.g., from mouse click).
#[derive(Component, Debug, Clone, Copy)]
pub struct SetCursorEvent {
  pub entity: Entity,
  pub position: usize,
  pub extend_selection: bool,
}

impl SetCursorEvent {
  pub fn new(entity: Entity, position: usize, extend_selection: bool) -> Self {
    Self {
      entity,
      position,
      extend_selection,
    }
  }
}

// ============================================================================
// Tabbar Events
// ============================================================================

/// Event: request to create a new editor tab.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NewEditorTabRequest;

/// Event: request to create a new terminal tab.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NewTerminalTabRequest;

/// Event: request to create a new playground tab.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NewPlaygroundTabRequest;

/// Event: request to toggle zoom mode.
#[derive(Component, Debug, Clone, Copy)]
pub struct ToggleZoomRequest {
  pub source: crate::tabbar::ZoomSource,
}

impl ToggleZoomRequest {
  pub fn editor() -> Self {
    Self {
      source: crate::tabbar::ZoomSource::Editor,
    }
  }

  pub fn terminal() -> Self {
    Self {
      source: crate::tabbar::ZoomSource::Terminal,
    }
  }

  pub fn playground() -> Self {
    Self {
      source: crate::tabbar::ZoomSource::Playground,
    }
  }
}

/// Event: request to navigate to previous tab.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NavigatePrevTabRequest;

/// Event: request to navigate to next tab.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NavigateNextTabRequest;

// ============================================================================
// Terminal Events
// ============================================================================

/// Event: request to create a new terminal.
#[derive(Component, Debug, Clone, Default)]
pub struct NewTerminalRequest {
  pub working_directory: Option<PathBuf>,
}

impl NewTerminalRequest {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn with_cwd(cwd: impl Into<PathBuf>) -> Self {
    Self {
      working_directory: Some(cwd.into()),
    }
  }
}

/// Event: request to close a terminal.
#[derive(Component, Debug, Clone, Copy)]
pub struct CloseTerminalRequest {
  pub entity: Entity,
}

impl CloseTerminalRequest {
  pub fn new(entity: Entity) -> Self {
    Self { entity }
  }
}

/// Event: send input to a terminal (keystrokes, paste).
#[derive(Component, Debug, Clone)]
pub struct TerminalInputEvent {
  pub entity: Entity,
  pub input: String,
}

impl TerminalInputEvent {
  pub fn new(entity: Entity, input: impl Into<String>) -> Self {
    Self {
      entity,
      input: input.into(),
    }
  }
}

/// Event: resize a terminal.
#[derive(Component, Debug, Clone, Copy)]
pub struct TerminalResizeEvent {
  pub entity: Entity,
  pub rows: u16,
  pub cols: u16,
}

impl TerminalResizeEvent {
  pub fn new(entity: Entity, rows: u16, cols: u16) -> Self {
    Self { entity, rows, cols }
  }
}

/// Event: scroll a terminal.
#[derive(Component, Debug, Clone, Copy)]
pub struct TerminalScrollEvent {
  pub entity: Entity,
  pub delta: i32,
}

impl TerminalScrollEvent {
  pub fn new(entity: Entity, delta: i32) -> Self {
    Self { entity, delta }
  }
}

// ============================================================================
// Explorer File Operation Events
// ============================================================================

/// Event: request to create a new file.
#[derive(Component, Debug, Clone)]
pub struct CreateFileRequest {
  pub parent_path: PathBuf,
  pub name: String,
}

impl CreateFileRequest {
  pub fn new(parent_path: impl Into<PathBuf>, name: impl Into<String>) -> Self {
    Self {
      parent_path: parent_path.into(),
      name: name.into(),
    }
  }
}

/// Event: request to create a new folder.
#[derive(Component, Debug, Clone)]
pub struct CreateFolderRequest {
  pub parent_path: PathBuf,
  pub name: String,
}

impl CreateFolderRequest {
  pub fn new(parent_path: impl Into<PathBuf>, name: impl Into<String>) -> Self {
    Self {
      parent_path: parent_path.into(),
      name: name.into(),
    }
  }
}

/// Event: request to rename a file or folder.
#[derive(Component, Debug, Clone)]
pub struct RenameRequest {
  pub entity: Entity,
  pub old_path: PathBuf,
  pub new_name: String,
}

impl RenameRequest {
  pub fn new(
    entity: Entity,
    old_path: impl Into<PathBuf>,
    new_name: impl Into<String>,
  ) -> Self {
    Self {
      entity,
      old_path: old_path.into(),
      new_name: new_name.into(),
    }
  }
}

/// Event: request to delete a file or folder.
#[derive(Component, Debug, Clone)]
pub struct DeleteRequest {
  pub entity: Entity,
  pub path: PathBuf,
  pub is_dir: bool,
}

impl DeleteRequest {
  pub fn new(entity: Entity, path: impl Into<PathBuf>, is_dir: bool) -> Self {
    Self {
      entity,
      path: path.into(),
      is_dir,
    }
  }
}

/// Event: request to copy path to clipboard.
#[derive(Component, Debug, Clone)]
pub struct CopyPathRequest {
  pub path: PathBuf,
}

/// Event: request to paste file/folder from clipboard.
#[derive(Component, Debug, Clone)]
pub struct PasteRequest {
  pub source: PathBuf,
  pub destination: PathBuf,
  pub is_cut: bool,
}

impl PasteRequest {
  pub fn new(
    source: impl Into<PathBuf>,
    destination: impl Into<PathBuf>,
    is_cut: bool,
  ) -> Self {
    Self {
      source: source.into(),
      destination: destination.into(),
      is_cut,
    }
  }
}

// ============================================================================
// Workspace Events
// ============================================================================

/// Event: request to add a folder to the workspace (multi-root).
#[derive(Component, Debug, Clone)]
pub struct AddRootRequest {
  pub path: PathBuf,
}

impl AddRootRequest {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into() }
  }
}

/// Event: request to remove a folder from the workspace.
#[derive(Component, Debug, Clone)]
pub struct RemoveRootRequest {
  pub path: PathBuf,
}

impl RemoveRootRequest {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into() }
  }
}

/// Event: request to open folder dialog for adding to workspace.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AddFolderToWorkspaceDialogRequest;

/// Event: request to open "Save As" dialog for a tab without a file path.
/// Used when saving a new/untitled tab.
#[derive(Component, Debug, Clone, Copy)]
pub struct SaveAsDialogRequest {
  pub entity: Entity,
}

impl SaveAsDialogRequest {
  pub fn new(entity: Entity) -> Self {
    Self { entity }
  }
}

// ============================================================================
// Explorer Header Events
// ============================================================================

/// Event: request to refresh the file explorer.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct RefreshExplorerRequest;

/// Event: request to collapse all folders in the explorer.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CollapseAllFoldersRequest;

/// Event: request to toggle visibility of hidden files.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ToggleHiddenFilesRequest;

// ============================================================================
// Preview Events
// ============================================================================

/// Event: request to toggle HTML preview panel.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ToggleHtmlPreviewRequest;

/// Event: request to toggle markdown preview for the active tab.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ToggleMarkdownPreviewRequest;

/// Event: request to toggle CSV preview for the active tab.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ToggleCsvPreviewRequest;

/// Event: request to open PDF preview for a file.
#[derive(Component, Debug, Clone)]
pub struct OpenPdfPreviewRequest(pub std::path::PathBuf);

/// Event: request to close PDF preview.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ClosePdfPreviewRequest;

/// Event: request to zoom in SVG preview.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SvgZoomInRequest;

/// Event: request to zoom out SVG preview.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SvgZoomOutRequest;

/// Event: request to reset SVG preview zoom.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SvgZoomResetRequest;

/// Event: request to toggle SQLite preview for the active tab.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ToggleSqlitePreviewRequest;

/// Event: request to toggle git blame for the active tab.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ToggleBlameRequest;

// ============================================================================
// Search Events
// ============================================================================

/// Event: request to toggle search panel visibility.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ToggleSearchRequest;

/// Event: request to hide search panel.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HideSearchRequest;

/// Event: request to find next match.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FindNextRequest;

/// Event: request to find previous match.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FindPreviousRequest;

/// Event: request to update search query.
#[derive(Component, Debug, Clone)]
pub struct UpdateSearchQueryRequest {
  pub query: String,
}

impl UpdateSearchQueryRequest {
  pub fn new(query: impl Into<String>) -> Self {
    Self {
      query: query.into(),
    }
  }
}

/// Event: request to toggle a search option.
#[derive(Component, Debug, Clone, Copy)]
pub struct ToggleSearchOptionRequest {
  pub option: crate::search::SearchOption,
}

impl ToggleSearchOptionRequest {
  pub fn new(option: crate::search::SearchOption) -> Self {
    Self { option }
  }
}

// ============================================================================
// Session Events
// ============================================================================

/// Event: request to clear saved session data.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ClearSessionRequest;

// ============================================================================
// Tab Context Menu Events
// ============================================================================

/// Event: request to close all tabs.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CloseAllTabsRequest;

/// Event: request to close all tabs except the specified one.
#[derive(Component, Debug, Clone, Copy)]
pub struct CloseOtherTabsRequest {
  pub entity: Entity,
}

impl CloseOtherTabsRequest {
  pub fn new(entity: Entity) -> Self {
    Self { entity }
  }
}

/// Event: request to close tabs to the right of the specified one.
#[derive(Component, Debug, Clone, Copy)]
pub struct CloseTabsToRightRequest {
  pub entity: Entity,
}

impl CloseTabsToRightRequest {
  pub fn new(entity: Entity) -> Self {
    Self { entity }
  }
}

// ============================================================================
// File Picker Events
// ============================================================================

/// Event: request to show file picker.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ShowFilePickerRequest {
  pub mode: crate::file_picker::FilePickerMode,
}

impl ShowFilePickerRequest {
  pub fn files() -> Self {
    Self {
      mode: crate::file_picker::FilePickerMode::Files,
    }
  }

  pub fn recent() -> Self {
    Self {
      mode: crate::file_picker::FilePickerMode::Recent,
    }
  }

  pub fn symbols() -> Self {
    Self {
      mode: crate::file_picker::FilePickerMode::Symbols,
    }
  }

  pub fn buffers() -> Self {
    Self {
      mode: crate::file_picker::FilePickerMode::Buffers,
    }
  }

  pub fn commands() -> Self {
    Self {
      mode: crate::file_picker::FilePickerMode::Commands,
    }
  }
}

/// Event: request to hide file picker.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HideFilePickerRequest;

/// Event: request to toggle file picker.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ToggleFilePickerRequest {
  pub mode: crate::file_picker::FilePickerMode,
}

impl ToggleFilePickerRequest {
  pub fn files() -> Self {
    Self {
      mode: crate::file_picker::FilePickerMode::Files,
    }
  }
}

// ============================================================================
// Code Folding Events
// ============================================================================

/// Event: request to toggle fold state for a symbol.
#[derive(Component, Debug, Clone, Copy)]
pub struct ToggleFoldRequest {
  /// The tab entity containing the symbol.
  pub entity: Entity,
  /// Index of the symbol in TabSymbols.map.anchors.
  pub symbol_index: usize,
}

impl ToggleFoldRequest {
  pub fn new(entity: Entity, symbol_index: usize) -> Self {
    Self {
      entity,
      symbol_index,
    }
  }
}

// ============================================================================
// Window Events
// ============================================================================

/// Event: request to center the window on screen.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CenterWindowRequest;

/// Event: request to shake the window (error feedback).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ShakeWindowRequest;

/// Event: request to position window on left half of screen.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PositionWindowLeftHalfRequest;

/// Event: request to position window on right half of screen.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PositionWindowRightHalfRequest;

// ============================================================================
// Playground Compilation Events
// ============================================================================

/// Event: request to compile playground source code.
#[derive(Component, Debug, Clone)]
pub struct CompileRequest {
  /// Source code to compile.
  pub source: String,
  /// Target for compilation (e.g., "native", "wasm").
  pub target: String,
  /// Stage to compile up to (inclusive): 0=Tokens, 1=Tree, 2=SIR, 3=Asm.
  pub stage: u8,
}

impl CompileRequest {
  pub fn new(
    source: impl Into<String>,
    target: impl Into<String>,
    stage: u8,
  ) -> Self {
    Self {
      source: source.into(),
      target: target.into(),
      stage,
    }
  }
}
