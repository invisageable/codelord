use crate::dialog;
use crate::events::{
  ActivateTabRequest, ClosePdfPreviewRequest, CursorMovement, DeleteDirection,
  DeleteTextEvent, InsertTextEvent, MoveCursorEvent, NewEditorTabRequest,
  OpenFileRequest, OpenPdfPreviewRequest, SaveAsDialogRequest, SaveFileRequest,
  SetCursorEvent, ToggleFoldRequest,
};
use crate::git::components::TabBlame;
use crate::keyboard::{FocusRequest, Focusable, KeyboardHandler};
use crate::previews::font::FontPreviewState;
use crate::previews::sqlite::SqlitePreviewState;
use crate::previews::svg::SvgPreviewState;
use crate::previews::xls::XlsPreviewState;
use crate::runtime::RuntimeHandle;
use crate::symbol::TabSymbols;
use crate::tabbar::components::{EditorTab, SonarAnimation, Tab};
use crate::tabbar::resources::TabOrderCounter;
use crate::text_editor::components::{Cursor, FileTab, TextBuffer};
use crate::text_editor::resources::PendingSaveFileDialog;
use crate::ui::component::{Active, Modified};

use bevy_ecs::entity::Entity;
use bevy_ecs::query::{With, Without};
use bevy_ecs::system::{Commands, Query, Res, ResMut};

/// Checks if a file is a SQLite database by extension.
fn is_sqlite_file(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| {
      matches!(ext.to_lowercase().as_str(), "db" | "sqlite" | "sqlite3")
    })
    .unwrap_or(false)
}

/// Checks if a file is a PDF by extension.
fn is_pdf_file(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| ext.eq_ignore_ascii_case("pdf"))
    .unwrap_or(false)
}

/// Checks if a file is an Excel/spreadsheet file by extension.
fn is_xls_file(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| {
      matches!(
        ext.to_lowercase().as_str(),
        "xls" | "xlsx" | "xlsm" | "xlsb" | "ods"
      )
    })
    .unwrap_or(false)
}

/// Checks if a file is a font file by extension.
fn is_font_file(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| {
      matches!(
        ext.to_lowercase().as_str(),
        "ttf" | "otf" | "woff" | "woff2"
      )
    })
    .unwrap_or(false)
}

/// Checks if a file is an SVG by extension.
fn is_svg_file(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| ext.eq_ignore_ascii_case("svg"))
    .unwrap_or(false)
}

/// System: processes OpenFileRequest events and spawns editor tabs.
#[allow(clippy::too_many_arguments)]
pub fn open_file_system(
  mut commands: Commands,
  requests: Query<(Entity, &OpenFileRequest)>,
  existing_tabs: Query<(Entity, &FileTab), With<EditorTab>>,
  active_tabs: Query<Entity, (With<EditorTab>, With<Active>)>,
  focusables: Query<&Focusable>,
  mut tab_order: ResMut<TabOrderCounter>,
  mut font_preview: ResMut<FontPreviewState>,
  mut sqlite_preview: ResMut<SqlitePreviewState>,
  mut svg_preview: ResMut<SvgPreviewState>,
  mut xls_preview: ResMut<XlsPreviewState>,
) {
  for (event_entity, request) in requests.iter() {
    let path = &request.path;
    let is_font = is_font_file(path);
    let is_sqlite = is_sqlite_file(path);
    let is_pdf = is_pdf_file(path);
    let is_svg = is_svg_file(path);
    let is_xls = is_xls_file(path);

    // Check if tab already exists for this path
    let existing_tab = existing_tabs
      .iter()
      .find(|(_, ft)| &ft.path == path)
      .map(|(e, _)| e);

    if let Some(tab_entity) = existing_tab {
      // Deactivate all active tabs
      for active_entity in active_tabs.iter() {
        commands.entity(active_entity).remove::<Active>();
      }
      // Activate existing tab
      commands.entity(tab_entity).insert(Active);

      // Auto-focus if the tab is focusable (text editor tabs).
      if focusables.get(tab_entity).is_ok() {
        commands.spawn(FocusRequest::new(tab_entity));
      }

      // Handle preview state based on file type
      if is_font {
        font_preview.open(path);
        sqlite_preview.close();
        svg_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_svg {
        svg_preview.open(path);
        font_preview.close();
        sqlite_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_sqlite {
        font_preview.close();
        sqlite_preview.enabled = true;
        sqlite_preview.current_file = Some(path.clone());
        sqlite_preview.needs_reload = true;
        svg_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_xls {
        font_preview.close();
        xls_preview.open(path.clone());
        sqlite_preview.close();
        svg_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_pdf {
        font_preview.close();
        sqlite_preview.close();
        svg_preview.close();
        xls_preview.close();
        commands.spawn(OpenPdfPreviewRequest(path.clone()));
      } else {
        // Non-binary file: close all binary previews
        font_preview.close();
        sqlite_preview.close();
        svg_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      }
    } else {
      // Deactivate all active tabs
      for active_entity in active_tabs.iter() {
        commands.entity(active_entity).remove::<Active>();
      }

      let label = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".into());

      let order = tab_order.next();

      if is_font {
        // Font files: spawn tab without text content, enable preview
        commands.spawn((
          Tab::new(label, order),
          EditorTab,
          SonarAnimation::default(),
          FileTab::new(path.clone()),
          TextBuffer::empty(),
          Cursor::new(0),
          Active,
        ));

        font_preview.open(path);
        sqlite_preview.close();
        svg_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_svg {
        // SVG files: spawn tab without text content, enable preview
        commands.spawn((
          Tab::new(label, order),
          EditorTab,
          SonarAnimation::default(),
          FileTab::new(path.clone()),
          TextBuffer::empty(),
          Cursor::new(0),
          Active,
        ));

        svg_preview.open(path);
        font_preview.close();
        sqlite_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_sqlite {
        // SQLite files: spawn tab without text content, enable preview
        commands.spawn((
          Tab::new(label, order),
          EditorTab,
          SonarAnimation::default(),
          FileTab::new(path.clone()),
          TextBuffer::empty(),
          Cursor::new(0),
          Active,
        ));

        font_preview.close();
        sqlite_preview.enabled = true;
        sqlite_preview.current_file = Some(path.clone());
        sqlite_preview.needs_reload = true;
        svg_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_xls {
        // XLS files: spawn tab without text content, enable preview
        commands.spawn((
          Tab::new(label, order),
          EditorTab,
          SonarAnimation::default(),
          FileTab::new(path.clone()),
          TextBuffer::empty(),
          Cursor::new(0),
          Active,
        ));

        font_preview.close();
        xls_preview.open(path.clone());
        sqlite_preview.close();
        svg_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_pdf {
        // PDF files: spawn tab without text content, enable preview via ECS
        commands.spawn((
          Tab::new(label, order),
          EditorTab,
          SonarAnimation::default(),
          FileTab::new(path.clone()),
          TextBuffer::empty(),
          Cursor::new(0),
          Active,
        ));

        font_preview.close();
        sqlite_preview.close();
        svg_preview.close();
        xls_preview.close();
        commands.spawn(OpenPdfPreviewRequest(path.clone()));
      } else {
        // Text files: read content and spawn full editor tab
        let content = std::fs::read_to_string(path).unwrap_or_default();

        let tab_entity = commands
          .spawn((
            Tab::new(label, order),
            EditorTab,
            SonarAnimation::default(),
            FileTab::new(path.clone()),
            TextBuffer::new(&content),
            Cursor::new(0),
            TabSymbols::new(),
            TabBlame::new(),
            Active,
            Focusable,
            KeyboardHandler::text_editor(),
          ))
          .id();

        // Auto-focus the new editor tab.
        commands.spawn(FocusRequest::new(tab_entity));

        // Close any active binary previews
        font_preview.close();
        sqlite_preview.close();
        svg_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      }
    }

    // Despawn the event (one-shot)
    commands.entity(event_entity).despawn();
  }
}

/// System: processes ActivateTabRequest events and switches active tab.
#[allow(clippy::too_many_arguments)]
pub fn activate_tab_system(
  mut commands: Commands,
  requests: Query<(Entity, &ActivateTabRequest)>,
  active_tabs: Query<Entity, (With<EditorTab>, With<Active>)>,
  file_tabs: Query<&FileTab>,
  focusables: Query<&Focusable>,
  mut font_preview: ResMut<FontPreviewState>,
  mut sqlite_preview: ResMut<SqlitePreviewState>,
  mut svg_preview: ResMut<SvgPreviewState>,
  mut xls_preview: ResMut<XlsPreviewState>,
) {
  for (event_entity, request) in requests.iter() {
    let target_tab = request.entity;

    // Deactivate all active tabs
    for active_entity in active_tabs.iter() {
      commands.entity(active_entity).remove::<Active>();
    }

    // Activate target tab
    commands.entity(target_tab).insert(Active);

    // Auto-focus if the tab is focusable (text editor tabs).
    if focusables.get(target_tab).is_ok() {
      commands.spawn(FocusRequest::new(target_tab));
    }

    // Handle preview state based on target tab type via ECS requests
    if let Ok(file_tab) = file_tabs.get(target_tab) {
      if is_font_file(&file_tab.path) {
        font_preview.open(&file_tab.path);
        sqlite_preview.close();
        svg_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_svg_file(&file_tab.path) {
        svg_preview.open(&file_tab.path);
        font_preview.close();
        sqlite_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_sqlite_file(&file_tab.path) {
        font_preview.close();
        sqlite_preview.enabled = true;
        sqlite_preview.current_file = Some(file_tab.path.clone());
        sqlite_preview.needs_reload = true;
        svg_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_xls_file(&file_tab.path) {
        font_preview.close();
        xls_preview.open(file_tab.path.clone());
        sqlite_preview.close();
        svg_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      } else if is_pdf_file(&file_tab.path) {
        font_preview.close();
        commands.spawn(OpenPdfPreviewRequest(file_tab.path.clone()));
        sqlite_preview.close();
        svg_preview.close();
        xls_preview.close();
      } else {
        font_preview.close();
        sqlite_preview.close();
        svg_preview.close();
        xls_preview.close();
        commands.spawn(ClosePdfPreviewRequest);
      }
    } else {
      font_preview.close();
      sqlite_preview.close();
      svg_preview.close();
      xls_preview.close();
      commands.spawn(ClosePdfPreviewRequest);
    }

    // Despawn the event (one-shot)
    commands.entity(event_entity).despawn();
  }
}

/// System: processes NewEditorTabRequest and creates an untitled editor tab.
#[allow(clippy::too_many_arguments)]
pub fn new_editor_tab_system(
  mut commands: Commands,
  requests: Query<Entity, With<NewEditorTabRequest>>,
  active_tabs: Query<Entity, (With<EditorTab>, With<Active>)>,
  mut tab_order: ResMut<TabOrderCounter>,
  mut font_preview: ResMut<FontPreviewState>,
  mut svg_preview: ResMut<SvgPreviewState>,
  mut sqlite_preview: ResMut<SqlitePreviewState>,
  mut xls_preview: ResMut<XlsPreviewState>,
) {
  for event_entity in requests.iter() {
    // Deactivate all active editor tabs
    for active_entity in active_tabs.iter() {
      commands.entity(active_entity).remove::<Active>();
    }

    // Create new untitled tab
    let order = tab_order.next();
    let label = format!("untitled-{}", order + 1);

    let tab_entity = commands
      .spawn((
        Tab::new(label, order),
        EditorTab,
        SonarAnimation::default(),
        TextBuffer::empty(),
        Cursor::new(0),
        TabSymbols::new(),
        Active,
        Focusable,
        KeyboardHandler::text_editor(),
      ))
      .id();

    // Auto-focus the new editor tab.
    commands.spawn(FocusRequest::new(tab_entity));

    // Close binary previews for untitled tabs
    font_preview.close();
    svg_preview.close();
    sqlite_preview.close();
    xls_preview.close();
    commands.spawn(ClosePdfPreviewRequest);

    // Despawn the event (one-shot)
    commands.entity(event_entity).despawn();
  }
}

/// System: processes InsertTextEvent and inserts text at cursor.
pub fn insert_text_system(
  mut commands: Commands,
  events: Query<(Entity, &InsertTextEvent)>,
  mut editors: Query<(&mut TextBuffer, &mut Cursor, Option<&mut TabSymbols>)>,
) {
  for (event_entity, event) in events.iter() {
    if let Ok((mut buffer, mut cursor, symbols)) = editors.get_mut(event.entity)
    {
      // If there's a selection, delete it first
      if let Some((start, end)) = cursor.selection() {
        buffer.delete_range(start, end);
        cursor.position = start;
        cursor.clear_selection();
      }

      // Insert text at cursor position
      buffer.insert(cursor.position, &event.text);

      // Move cursor past inserted text
      cursor.position += event.text.chars().count();

      // Mark symbols as needing re-extraction
      if let Some(mut sym) = symbols {
        sym.mark_dirty();
      }

      // Mark as modified (dirty)
      commands.entity(event.entity).insert(Modified);
    }

    commands.entity(event_entity).despawn();
  }
}

/// System: processes DeleteTextEvent and deletes text.
pub fn delete_text_system(
  mut commands: Commands,
  events: Query<(Entity, &DeleteTextEvent)>,
  mut editors: Query<(&mut TextBuffer, &mut Cursor, Option<&mut TabSymbols>)>,
) {
  for (event_entity, event) in events.iter() {
    let mut did_modify = false;

    if let Ok((mut buffer, mut cursor, symbols)) = editors.get_mut(event.entity)
    {
      // If there's a selection, delete it
      if let Some((start, end)) = cursor.selection() {
        buffer.delete_range(start, end);
        cursor.position = start;
        cursor.clear_selection();
        did_modify = true;
      } else {
        // No selection - delete single char
        match event.direction {
          DeleteDirection::Before => {
            if cursor.position > 0 {
              buffer.delete_char_before(cursor.position);
              cursor.position -= 1;
              did_modify = true;
            }
          }
          DeleteDirection::After => {
            if cursor.position < buffer.len_chars() {
              buffer.delete_char_after(cursor.position);
              did_modify = true;
            }
          }
        }
      }

      // Mark symbols as needing re-extraction
      if did_modify && let Some(mut sym) = symbols {
        sym.mark_dirty();
      }
    }

    // Mark as modified (dirty) if content changed
    if did_modify {
      commands.entity(event.entity).insert(Modified);
    }

    commands.entity(event_entity).despawn();
  }
}

/// System: processes MoveCursorEvent and moves cursor.
pub fn move_cursor_system(
  mut commands: Commands,
  events: Query<(Entity, &MoveCursorEvent)>,
  mut editors: Query<(&TextBuffer, &mut Cursor)>,
) {
  for (event_entity, event) in events.iter() {
    if let Ok((buffer, mut cursor)) = editors.get_mut(event.entity) {
      let new_pos =
        calculate_new_position(buffer, cursor.position, event.movement);

      cursor.move_to(new_pos, event.extend_selection);
    }

    commands.entity(event_entity).despawn();
  }
}

/// System: processes SetCursorEvent and sets cursor position.
pub fn set_cursor_system(
  mut commands: Commands,
  events: Query<(Entity, &SetCursorEvent)>,
  mut editors: Query<(&TextBuffer, &mut Cursor)>,
) {
  for (event_entity, event) in events.iter() {
    if let Ok((buffer, mut cursor)) = editors.get_mut(event.entity) {
      let pos = event.position.min(buffer.len_chars());
      cursor.move_to(pos, event.extend_selection);
    }

    commands.entity(event_entity).despawn();
  }
}

/// System: processes SaveFileRequest and saves file to disk.
/// For tabs WITH FileTab, saves directly. For tabs WITHOUT FileTab,
/// spawns SaveAsDialogRequest to open a save dialog.
pub fn save_file_system(
  mut commands: Commands,
  events: Query<(Entity, &SaveFileRequest)>,
  editors_with_file: Query<(&TextBuffer, &FileTab)>,
  editors_without_file: Query<&TextBuffer, Without<FileTab>>,
) {
  for (event_entity, event) in events.iter() {
    // Check if tab has a file path
    if let Ok((buffer, file_tab)) = editors_with_file.get(event.entity) {
      // Tab has FileTab - save directly
      let content = buffer.to_string();
      let path = &file_tab.path;

      if std::fs::write(path, &content).is_ok() {
        // Remove Modified component to clear dirty state.
        commands.entity(event.entity).remove::<Modified>();
      }
    } else if editors_without_file.get(event.entity).is_ok() {
      // Tab has no FileTab (new/untitled) - spawn SaveAsDialogRequest
      commands.spawn(SaveAsDialogRequest::new(event.entity));
    }

    commands.entity(event_entity).despawn();
  }
}

/// System: handles SaveAsDialogRequest by opening a native "Save As" dialog.
pub fn save_as_dialog_system(
  mut commands: Commands,
  requests: Query<(Entity, &SaveAsDialogRequest)>,
  tabs: Query<&Tab>,
  pending: Option<Res<PendingSaveFileDialog>>,
  runtime: Option<Res<RuntimeHandle>>,
) {
  // Only process if no dialog is pending and runtime is available
  if pending.is_some() {
    return;
  }

  let Some(runtime) = runtime else {
    return;
  };

  for (event_entity, request) in requests.iter() {
    let tab_entity = request.entity;

    // Get default filename from tab label
    let default_name = tabs
      .get(tab_entity)
      .map(|t| format!("{}.txt", t.label))
      .unwrap_or_else(|_| "untitled.txt".to_string());

    let rx = dialog::save_file(
      &runtime,
      &default_name,
      &[
        ("All Files", &["*"]),
        ("Zo", &["zo"]),
        ("Rust", &["rs"]),
        ("Text", &["txt", "md"]),
      ],
    );

    // Store pending dialog resource
    commands.insert_resource(PendingSaveFileDialog::new(rx, tab_entity));
    commands.entity(event_entity).despawn();
  }
}

/// System: polls pending save file dialog for results.
/// When user selects a path, adds FileTab to tab and saves the file.
pub fn poll_save_file_dialog_system(
  mut commands: Commands,
  pending: Option<Res<PendingSaveFileDialog>>,
  editors: Query<&TextBuffer>,
  mut tabs: Query<&mut Tab>,
) {
  let Some(pending) = pending else {
    return;
  };

  match pending.receiver.try_recv() {
    Ok(Some(path)) => {
      let entity = pending.entity;

      // Add FileTab and TabBlame components with the chosen path
      commands
        .entity(entity)
        .insert((FileTab::new(path.clone()), TabBlame::new()));

      // Save the file
      if let Ok(buffer) = editors.get(entity) {
        let content = buffer.to_string();

        if std::fs::write(&path, &content).is_ok() {
          // Remove Modified component
          commands.entity(entity).remove::<Modified>();

          // Update tab label to filename
          if let Ok(mut tab) = tabs.get_mut(entity) {
            tab.label = path
              .file_name()
              .map(|n| n.to_string_lossy().to_string())
              .unwrap_or_else(|| "untitled".into());
          }
        }
      }

      commands.remove_resource::<PendingSaveFileDialog>();
    }
    Ok(None) => {
      // User cancelled the dialog
      commands.remove_resource::<PendingSaveFileDialog>();
    }
    Err(flume::TryRecvError::Empty) => {
      // Dialog still open, keep waiting
    }
    Err(flume::TryRecvError::Disconnected) => {
      // Dialog closed unexpectedly
      commands.remove_resource::<PendingSaveFileDialog>();
    }
  }
}

/// Calculate new cursor position based on movement.
fn calculate_new_position(
  buffer: &TextBuffer,
  current: usize,
  movement: CursorMovement,
) -> usize {
  let (line, col) = buffer.char_to_line_col(current);
  let total_chars = buffer.len_chars();
  let total_lines = buffer.len_lines();

  match movement {
    CursorMovement::Left => current.saturating_sub(1),

    CursorMovement::Right => (current + 1).min(total_chars),

    CursorMovement::Up => {
      if line == 0 {
        0
      } else {
        buffer.line_col_to_char(line - 1, col)
      }
    }

    CursorMovement::Down => {
      if line + 1 >= total_lines {
        total_chars
      } else {
        buffer.line_col_to_char(line + 1, col)
      }
    }

    CursorMovement::LineStart => buffer.line_col_to_char(line, 0),

    CursorMovement::LineEnd => {
      if let Some(line_slice) = buffer.line(line) {
        let line_start = buffer.line_col_to_char(line, 0);
        let line_len = line_slice.len_chars();
        // Don't include newline
        let end_col = if line < total_lines - 1 {
          line_len.saturating_sub(1)
        } else {
          line_len
        };
        line_start + end_col
      } else {
        current
      }
    }

    CursorMovement::BufferStart => 0,

    CursorMovement::BufferEnd => total_chars,

    CursorMovement::WordLeft => find_word_boundary_left(buffer, current),

    CursorMovement::WordRight => find_word_boundary_right(buffer, current),
  }
}

/// Find word boundary to the left of cursor.
fn find_word_boundary_left(buffer: &TextBuffer, pos: usize) -> usize {
  if pos == 0 {
    return 0;
  }

  let text = buffer.to_string();
  let chars: Vec<char> = text.chars().collect();
  let mut idx = pos - 1;

  // Skip whitespace
  while idx > 0 && chars[idx].is_whitespace() {
    idx -= 1;
  }

  // Skip word characters
  while idx > 0 && !chars[idx - 1].is_whitespace() {
    idx -= 1;
  }

  idx
}

/// Find word boundary to the right of cursor.
fn find_word_boundary_right(buffer: &TextBuffer, pos: usize) -> usize {
  let text = buffer.to_string();
  let chars: Vec<char> = text.chars().collect();
  let len = chars.len();

  if pos >= len {
    return len;
  }

  let mut idx = pos;

  // Skip current word
  while idx < len && !chars[idx].is_whitespace() {
    idx += 1;
  }

  // Skip whitespace
  while idx < len && chars[idx].is_whitespace() {
    idx += 1;
  }

  idx
}

/// System: processes ToggleFoldRequest events and toggles fold state.
pub fn toggle_fold_system(
  mut commands: Commands,
  events: Query<(Entity, &ToggleFoldRequest)>,
  mut editors: Query<&mut TabSymbols>,
) {
  for (event_entity, event) in events.iter() {
    if let Ok(mut symbols) = editors.get_mut(event.entity) {
      symbols.folds.toggle(event.symbol_index);
    }

    commands.entity(event_entity).despawn();
  }
}
