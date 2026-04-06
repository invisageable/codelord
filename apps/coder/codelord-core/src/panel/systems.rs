use super::components::RightPanelView;
use super::resources::{
  BottomPanelResource, LeftPanelResource, PanelAction, PanelCommand,
  RightPanelResource,
};
use crate::animation::resources::ActiveAnimations;
use crate::events::{
  ClosePdfPreviewRequest, FindNextRequest, FindPreviousRequest,
  HideSearchRequest, OpenPdfPreviewRequest, SvgZoomInRequest,
  SvgZoomOutRequest, SvgZoomResetRequest, ToggleCsvPreviewRequest,
  ToggleHtmlPreviewRequest, ToggleMarkdownPreviewRequest,
  ToggleSearchOptionRequest, ToggleSearchRequest, ToggleSqlitePreviewRequest,
  UpdateSearchQueryRequest,
};
use crate::loading::{GlobalLoading, LoadingTask};
use crate::previews::sqlite::{
  ChangePageRequest, ExecuteSqlRequest, ExportRequest, QueryResult,
  SelectTableRequest,
};
use crate::previews::xls::{ChangeXlsPageRequest, SelectSheetRequest};
use crate::previews::{
  CsvPreviewState, HtmlPreviewState, MarkdownPreviewState, PdfConnection,
  PdfPreviewState, SqlitePreviewState, SvgPreviewState, XlsPreviewState,
};
use crate::runtime::RuntimeHandle;
use crate::search::SearchState;
use crate::tabbar::components::EditorTab;
use crate::text_editor::components::{FileTab, TextBuffer};
use crate::ui::component::Active;

use bevy_ecs::entity::Entity;
use bevy_ecs::message::MessageReader;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query, Res, ResMut};

use std::sync::Arc;

/// System to handle panel visibility commands.
///
/// Listens for PanelCommand messages and updates panel resources.
pub fn panel_command_system(
  mut commands: MessageReader<PanelCommand>,
  mut left: ResMut<LeftPanelResource>,
  mut right: ResMut<RightPanelResource>,
  mut bottom: ResMut<BottomPanelResource>,
) {
  for command in commands.read() {
    match command.action {
      PanelAction::ToggleLeft => left.toggle(),
      PanelAction::ToggleRight => right.toggle(),
      PanelAction::ToggleBottom => bottom.toggle(),
    }
  }
}

/// System to handle HTML preview toggle requests.
///
/// Toggles the HTML preview state. The actual WebView show/hide
/// is handled in the app's update method (outside ECS) because
/// wry::WebView is !Send+!Sync.
pub fn toggle_html_preview_system(
  mut commands: Commands,
  requests: Query<Entity, With<ToggleHtmlPreviewRequest>>,
  mut preview_state: ResMut<HtmlPreviewState>,
  mut right_panel: ResMut<RightPanelResource>,
  active_tabs: Query<&FileTab, (With<EditorTab>, With<Active>)>,
) {
  for entity in requests.iter() {
    // Check if HTML preview is currently active (visible AND in WebView mode)
    let is_preview_active = right_panel.is_visible
      && right_panel.active_view == RightPanelView::WebView;

    if is_preview_active {
      // HTML preview is active - close the right panel entirely
      preview_state.enabled = false;
      preview_state.current_file = None;
      right_panel.is_visible = false;

      log::debug!("[HtmlPreview] Closed right panel");
    } else {
      // HTML preview is not active - show it
      preview_state.enabled = true;
      right_panel.active_view = RightPanelView::WebView;
      right_panel.is_visible = true;

      // Set the current file from the active tab
      if let Some(file_tab) = active_tabs.iter().next() {
        preview_state.current_file = Some(file_tab.path.clone());
        preview_state.needs_reload = true;

        log::debug!("[HtmlPreview] Opened with file: {:?}", file_tab.path);
      }
    }

    // Despawn the event (one-shot)
    commands.entity(entity).despawn();
  }
}

/// System to update HTML preview when the active tab changes.
///
/// If HTML preview is active and the newly activated tab is an HTML file,
/// update the preview to show that file.
pub fn update_html_preview_on_tab_change(
  mut preview_state: ResMut<HtmlPreviewState>,
  right_panel: Res<RightPanelResource>,
  active_tabs: Query<&FileTab, (With<EditorTab>, With<Active>)>,
) {
  // Only update if HTML preview is active
  if !preview_state.enabled
    || !right_panel.is_visible
    || right_panel.active_view != RightPanelView::WebView
  {
    return;
  }

  // Get the active tab's file
  let Some(file_tab) = active_tabs.iter().next() else {
    return;
  };

  // Only update for HTML files
  if !file_tab.is_html() {
    return;
  }

  // Check if the file changed
  let current = preview_state.current_file.as_ref();
  if current.map(|p| p != &file_tab.path).unwrap_or(true) {
    preview_state.current_file = Some(file_tab.path.clone());
    preview_state.needs_reload = true;

    log::debug!(
      "[HtmlPreview] Tab changed, updating to: {:?}",
      file_tab.path
    );
  }
}

/// System to handle markdown preview toggle requests.
///
/// Toggles markdown preview for the active tab. Unlike HTML preview,
/// markdown preview replaces the editor content inline.
#[allow(clippy::type_complexity)]
pub fn toggle_markdown_preview_system(
  mut commands: Commands,
  requests: Query<Entity, With<ToggleMarkdownPreviewRequest>>,
  mut preview_state: ResMut<MarkdownPreviewState>,
  active_tabs: Query<(&FileTab, &TextBuffer), (With<EditorTab>, With<Active>)>,
) {
  for entity in requests.iter() {
    // Get the active tab's file and content
    if let Some((file_tab, text_buffer)) = active_tabs.iter().next() {
      let content = text_buffer.to_string();
      preview_state.toggle(file_tab.path.clone(), content);
    }

    // Despawn the event (one-shot)
    commands.entity(entity).despawn();
  }
}

/// System to update markdown preview content when text buffer changes.
///
/// If markdown preview is active and the file content changed,
/// update the cached content.
#[allow(clippy::type_complexity)]
pub fn update_markdown_preview_on_change(
  mut preview_state: ResMut<MarkdownPreviewState>,
  active_tabs: Query<(&FileTab, &TextBuffer), (With<EditorTab>, With<Active>)>,
) {
  // Only update if markdown preview is active
  if !preview_state.enabled {
    return;
  }

  // Get the active tab's file and content
  let Some((file_tab, text_buffer)) = active_tabs.iter().next() else {
    return;
  };

  // Only update for markdown files
  if !file_tab.is_markdown() {
    return;
  }

  // Update content if this is the file being previewed
  if preview_state.is_active_for(&file_tab.path) {
    let current_content = text_buffer.to_string();

    // Check if content actually changed (avoid unnecessary re-renders)
    if preview_state.cached_content.as_ref() != Some(&current_content) {
      preview_state.update_content(&file_tab.path, current_content);
    }
  }
}

/// System to update markdown preview when switching tabs.
///
/// - If switching to a non-markdown file → close preview
/// - If switching to a different markdown file → update preview content
#[allow(clippy::type_complexity)]
pub fn update_markdown_preview_on_tab_change(
  mut preview_state: ResMut<MarkdownPreviewState>,
  active_tabs: Query<(&FileTab, &TextBuffer), (With<EditorTab>, With<Active>)>,
) {
  // Only check if markdown preview is active
  if !preview_state.enabled {
    return;
  }

  // Get the active tab's file and content
  let Some((file_tab, text_buffer)) = active_tabs.iter().next() else {
    return;
  };

  // If we switched to a non-markdown file, close preview
  if !file_tab.is_markdown() {
    preview_state.close();
    return;
  }

  // If we switched to a different markdown file, update preview content
  if preview_state.current_file.as_ref() != Some(&file_tab.path) {
    preview_state.current_file = Some(file_tab.path.clone());
    preview_state.cached_content = Some(text_buffer.to_string());
    preview_state.generation += 1;
  }
}

// ============================================================================
// CSV Preview Systems
// ============================================================================

/// System to handle CSV preview toggle requests.
#[allow(clippy::type_complexity)]
pub fn toggle_csv_preview_system(
  mut commands: Commands,
  requests: Query<Entity, With<ToggleCsvPreviewRequest>>,
  mut preview_state: ResMut<CsvPreviewState>,
  active_tabs: Query<(&FileTab, &TextBuffer), (With<EditorTab>, With<Active>)>,
) {
  for entity in requests.iter() {
    if let Some((file_tab, text_buffer)) = active_tabs.iter().next() {
      let content = text_buffer.to_string();
      preview_state.toggle(file_tab.path.clone(), &content);
    }

    commands.entity(entity).despawn();
  }
}

/// System to update CSV preview content when text buffer changes.
#[allow(clippy::type_complexity)]
pub fn update_csv_preview_on_change(
  mut preview_state: ResMut<CsvPreviewState>,
  active_tabs: Query<(&FileTab, &TextBuffer), (With<EditorTab>, With<Active>)>,
) {
  if !preview_state.enabled {
    return;
  }

  let Some((file_tab, text_buffer)) = active_tabs.iter().next() else {
    return;
  };

  if !file_tab.is_csv() {
    return;
  }

  if preview_state.is_active_for(&file_tab.path) {
    let current_content = text_buffer.to_string();
    // Re-parse CSV if content changed (check by generation or content hash)
    preview_state.update_content(&file_tab.path, &current_content);
  }
}

/// System to update CSV preview when switching tabs.
#[allow(clippy::type_complexity)]
pub fn update_csv_preview_on_tab_change(
  mut preview_state: ResMut<CsvPreviewState>,
  active_tabs: Query<(&FileTab, &TextBuffer), (With<EditorTab>, With<Active>)>,
) {
  if !preview_state.enabled {
    return;
  }

  let Some((file_tab, text_buffer)) = active_tabs.iter().next() else {
    return;
  };

  // If we switched to a non-CSV file, close preview
  if !file_tab.is_csv() {
    preview_state.close();
    return;
  }

  // If we switched to a different CSV file, update preview data
  if preview_state.current_file.as_ref() != Some(&file_tab.path) {
    let content = text_buffer.to_string();
    preview_state.current_file = Some(file_tab.path.clone());
    preview_state.cached_data = Some(crate::previews::csv::parse_csv(&content));
    preview_state.generation += 1;
  }
}

// ============================================================================
// PDF Preview Systems
// ============================================================================

/// System to handle PDF preview open requests.
pub fn open_pdf_preview_system(
  mut commands: Commands,
  requests: Query<(Entity, &OpenPdfPreviewRequest)>,
  mut preview_state: ResMut<PdfPreviewState>,
  mut pdf_connection: ResMut<PdfConnection>,
  mut global_loading: ResMut<GlobalLoading>,
  mut active_animations: ResMut<ActiveAnimations>,
) {
  for (entity, request) in requests.iter() {
    let new_file = &request.0;
    let is_different_file =
      preview_state.current_file.as_ref() != Some(new_file);

    // Close connection when switching files (need new pdfium worker for new
    // file) But keep the cache - it's keyed by file path so old pages are
    // preserved
    if is_different_file && pdf_connection.is_connected() {
      log::info!(
        "[PDF] Switching file, closing connection. Old: {:?}, New: {new_file:?}",
        preview_state.current_file,
      );
      pdf_connection.close();
    }

    // Start loading indicator BEFORE open() sets is_loading
    if is_different_file {
      global_loading.start(LoadingTask::PdfRender);
      active_animations.increment();
    }

    preview_state.open(new_file.clone());
    commands.entity(entity).despawn();
  }
}

/// System to handle PDF preview close requests.
/// Uses disable() to hide preview while keeping cache for quick restore.
pub fn close_pdf_preview_system(
  mut commands: Commands,
  requests: Query<Entity, With<ClosePdfPreviewRequest>>,
  mut preview_state: ResMut<PdfPreviewState>,
) {
  for entity in requests.iter() {
    preview_state.disable();
    commands.entity(entity).despawn();
  }
}

/// System to update PDF preview when switching between PDF files.
///
/// Note: This system does NOT open or close the preview. It only updates
/// when switching between different PDF files. Opening/closing is handled by:
/// - `activate_tab_system` when clicking tabs
/// - `open_file_system` when opening files
/// - `new_editor_tab_system` when creating untitled tabs
/// - `close_editor_tab_system` when closing tabs
///
/// This prevents race conditions with deferred commands.
pub fn update_pdf_preview_on_tab_change(
  mut preview_state: ResMut<PdfPreviewState>,
  mut pdf_connection: ResMut<PdfConnection>,
  mut global_loading: ResMut<GlobalLoading>,
  mut active_animations: ResMut<ActiveAnimations>,
  active_tabs: Query<&FileTab, (With<EditorTab>, With<Active>)>,
) {
  if !preview_state.enabled {
    return;
  }

  let Some(file_tab) = active_tabs.iter().next() else {
    return;
  };

  // Only update when switching between different PDF files
  if file_tab.is_pdf()
    && preview_state.current_file.as_ref() != Some(&file_tab.path)
  {
    // Close existing connection - need new worker for different file
    if pdf_connection.is_connected() {
      log::info!(
        "[PDF] Tab switch, closing connection. Old: {:?}, New: {:?}",
        preview_state.current_file,
        file_tab.path,
      );

      pdf_connection.close();
    }

    // Start loading indicator
    global_loading.start(LoadingTask::PdfRender);
    active_animations.increment();

    // Use open() to properly reset state and trigger loading
    preview_state.open(file_tab.path.clone());
  }
}

// ============================================================================
// Search Systems
// ============================================================================

/// System to handle search panel toggle requests.
pub fn toggle_search_system(
  mut commands: Commands,
  requests: Query<Entity, With<ToggleSearchRequest>>,
  mut search_state: ResMut<SearchState>,
) {
  for entity in requests.iter() {
    search_state.toggle();
    commands.entity(entity).despawn();
  }
}

/// System to handle search panel hide requests.
pub fn hcodelord_search_system(
  mut commands: Commands,
  requests: Query<Entity, With<HideSearchRequest>>,
  mut search_state: ResMut<SearchState>,
) {
  for entity in requests.iter() {
    search_state.hide();
    commands.entity(entity).despawn();
  }
}

/// System to handle search query update requests.
pub fn update_search_query_system(
  mut commands: Commands,
  requests: Query<(Entity, &UpdateSearchQueryRequest)>,
  mut search_state: ResMut<SearchState>,
) {
  for (entity, request) in requests.iter() {
    search_state.update_query(request.query.clone());
    commands.entity(entity).despawn();
  }
}

/// System to handle search option toggle requests.
pub fn toggle_search_option_system(
  mut commands: Commands,
  requests: Query<(Entity, &ToggleSearchOptionRequest)>,
  mut search_state: ResMut<SearchState>,
) {
  for (entity, request) in requests.iter() {
    search_state.toggle_option(request.option);
    commands.entity(entity).despawn();
  }
}

/// System to handle find next requests.
pub fn find_next_system(
  mut commands: Commands,
  requests: Query<Entity, With<FindNextRequest>>,
  mut search_state: ResMut<SearchState>,
) {
  for entity in requests.iter() {
    search_state.next_match();
    commands.entity(entity).despawn();
  }
}

/// System to handle find previous requests.
pub fn find_previous_system(
  mut commands: Commands,
  requests: Query<Entity, With<FindPreviousRequest>>,
  mut search_state: ResMut<SearchState>,
) {
  for entity in requests.iter() {
    search_state.previous_match();
    commands.entity(entity).despawn();
  }
}

/// System to execute search when query changes.
///
/// This runs the search on the active tab's text buffer and updates
/// the search state with the results using Aho-Corasick for efficiency.
pub fn execute_search_system(
  mut search_state: ResMut<SearchState>,
  active_tabs: Query<&TextBuffer, (With<EditorTab>, With<Active>)>,
) {
  // Only search if query is not empty and debounce time passed
  if search_state.query.is_empty() {
    if search_state.total_matches > 0 {
      search_state.matches = Arc::new(Vec::new());
      search_state.total_matches = 0;
      search_state.current_match_index = 0;
    }

    return;
  }

  // Check debounce (5ms for small files)
  if !search_state.should_search(5) {
    return;
  }

  // Get active tab's text
  let Some(text_buffer) = active_tabs.iter().next() else {
    return;
  };

  let text = text_buffer.to_string();

  // Perform search using Aho-Corasick engine
  let matches = crate::search::perform_search_str(&text, &search_state);

  // Update search state with results
  let generation = search_state.generation;
  search_state.update_results(generation, Arc::new(matches));
}

// ============================================================================
// SQLite Preview Systems
// ============================================================================

/// System to handle SQLite preview toggle requests.
pub fn toggle_sqlite_preview_system(
  mut commands: Commands,
  requests: Query<Entity, With<ToggleSqlitePreviewRequest>>,
  mut preview_state: ResMut<SqlitePreviewState>,
  active_tabs: Query<&FileTab, (With<EditorTab>, With<Active>)>,
) {
  for entity in requests.iter() {
    if let Some(file_tab) = active_tabs.iter().next() {
      preview_state.toggle(file_tab.path.clone());
    }

    commands.entity(entity).despawn();
  }
}

/// System to update SQLite preview when switching between SQLite files.
///
/// Note: This system does NOT close the preview. Closing is handled by:
/// - `activate_tab_system` when clicking non-SQLite tabs
/// - `open_file_system` when opening non-SQLite files
/// - `new_editor_tab_system` when creating untitled tabs
///
/// This prevents race conditions with deferred commands.
pub fn update_sqlite_preview_on_tab_change(
  mut preview_state: ResMut<SqlitePreviewState>,
  active_tabs: Query<&FileTab, (With<EditorTab>, With<Active>)>,
) {
  if !preview_state.enabled {
    return;
  }

  let Some(file_tab) = active_tabs.iter().next() else {
    return;
  };

  // Only handle switching between different SQLite files
  if file_tab.is_sqlite()
    && preview_state.current_file.as_ref() != Some(&file_tab.path)
  {
    preview_state.current_file = Some(file_tab.path.clone());
    preview_state.tables.clear();
    preview_state.selected_table = None;
    preview_state.current_page = 0;
    preview_state.data = QueryResult::default();
    preview_state.needs_reload = true;
  }
}

/// System to handle SQLite table selection requests.
pub fn select_sqlite_table_system(
  mut commands: Commands,
  requests: Query<(Entity, &SelectTableRequest)>,
  mut preview_state: ResMut<SqlitePreviewState>,
) {
  for (entity, request) in requests.iter() {
    preview_state.select_table(request.0);
    commands.entity(entity).despawn();
  }
}

/// System to handle SQLite page change requests.
pub fn change_sqlite_page_system(
  mut commands: Commands,
  requests: Query<(Entity, &ChangePageRequest)>,
  mut preview_state: ResMut<SqlitePreviewState>,
) {
  for (entity, request) in requests.iter() {
    let target_page = request.0;
    let total = preview_state.total_pages();

    if target_page < total {
      preview_state.current_page = target_page;
      preview_state.needs_reload = true;
    }

    commands.entity(entity).despawn();
  }
}

/// System to handle SQLite custom SQL execution requests.
pub fn execute_sqlite_sql_system(
  mut commands: Commands,
  requests: Query<Entity, With<ExecuteSqlRequest>>,
  mut preview_state: ResMut<SqlitePreviewState>,
) {
  for entity in requests.iter() {
    // Set needs_reload to trigger execution in Coder update loop
    preview_state.needs_reload = true;
    commands.entity(entity).despawn();
  }
}

/// System to handle SQLite export requests.
///
/// Note: The actual file save dialog is spawned from here but runs
/// in tokio's blocking thread pool since rfd::FileDialog is blocking.
pub fn export_sqlite_data_system(
  mut commands: Commands,
  requests: Query<(Entity, &ExportRequest)>,
  preview_state: Res<SqlitePreviewState>,
  runtime: Option<Res<RuntimeHandle>>,
) {
  let Some(runtime) = runtime else {
    return;
  };

  for (entity, request) in requests.iter() {
    // Get table name for filename
    let table_name = preview_state
      .selected_table
      .and_then(|idx| preview_state.tables.get(idx))
      .map(|t| t.name.clone())
      .unwrap_or_else(|| "export".to_string());

    // Generate export content
    let (content, extension, filter_name) = match request {
      ExportRequest::Csv => {
        let csv = export_to_csv(&preview_state.data);
        (csv, "csv", "CSV")
      }
      ExportRequest::Json => {
        let json = export_to_json(&preview_state.data, Some(&table_name));
        (json, "json", "JSON")
      }
    };

    let default_filename = format!("{table_name}.{extension}");

    // Spawn file save dialog in tokio's blocking thread pool
    runtime.spawn_blocking(move || {
      let dialog = rfd::FileDialog::new()
        .set_file_name(&default_filename)
        .add_filter(filter_name, &[extension]);

      if let Some(path) = dialog.save_file() {
        if let Err(e) = std::fs::write(&path, content) {
          log::error!("[SQLite Export] Failed to write file: {e}");
        } else {
          log::info!("[SQLite Export] Saved to: {}", path.display());
        }
      }
    });

    commands.entity(entity).despawn();
  }
}

// ============================================================================
// SQLite Export Helpers
// ============================================================================

/// Exports query result to CSV format using the csv crate.
fn export_to_csv(data: &QueryResult) -> String {
  let mut writer = csv::Writer::from_writer(Vec::new());

  // Write header row.
  if writer.write_record(&data.columns).is_err() {
    return String::new();
  }

  // Write data rows.
  for row in &data.rows {
    if writer.write_record(row).is_err() {
      break;
    }
  }

  // Flush and convert to string.
  writer
    .into_inner()
    .ok()
    .and_then(|bytes| String::from_utf8(bytes).ok())
    .unwrap_or_default()
}

/// Exports query result to JSON format using sonic_rs.
fn export_to_json(data: &QueryResult, table_name: Option<&str>) -> String {
  use sonic_rs::{Number, Object, Value};

  // Convert each row to a JSON object.
  let rows: Vec<Value> = data
    .rows
    .iter()
    .map(|row| {
      let mut obj = Object::new();
      for (col_idx, cell) in row.iter().enumerate() {
        let col_name =
          data.columns.get(col_idx).map(|s| s.as_str()).unwrap_or("_");

        // Try to preserve numeric types, otherwise use string or null.
        let value: Value = if cell == "NULL" || cell.is_empty() {
          Value::from(())
        } else if let Ok(n) = cell.parse::<i64>() {
          Value::from(n)
        } else if let Ok(n) = cell.parse::<f64>() {
          Number::from_f64(n)
            .map(Value::from)
            .unwrap_or_else(|| Value::from(cell.as_str()))
        } else {
          Value::from(cell.as_str())
        };

        obj.insert(col_name, value);
      }

      Value::from(obj)
    })
    .collect();

  let array = Value::from(rows);

  // Optionally wrap in an object with table name as key.
  let output = if let Some(name) = table_name {
    let mut wrapper = Object::new();

    wrapper.insert(name, array);
    Value::from(wrapper)
  } else {
    array
  };

  sonic_rs::to_string_pretty(&output).unwrap_or_else(|_| "[]".to_string())
}

// ============================================================================
// XLS Preview Systems
// ============================================================================

/// System to handle XLS sheet selection requests.
pub fn select_xls_sheet_system(
  mut commands: Commands,
  requests: Query<(Entity, &SelectSheetRequest)>,
  mut preview_state: ResMut<XlsPreviewState>,
) {
  for (entity, request) in requests.iter() {
    preview_state.select_sheet(request.0);
    commands.entity(entity).despawn();
  }
}

/// System to handle XLS page change requests.
pub fn change_xls_page_system(
  mut commands: Commands,
  requests: Query<(Entity, &ChangeXlsPageRequest)>,
  mut preview_state: ResMut<XlsPreviewState>,
) {
  for (entity, request) in requests.iter() {
    preview_state.go_to_page(request.0);
    commands.entity(entity).despawn();
  }
}

// ============================================================================
// SVG Preview Systems
// ============================================================================

/// System to handle SVG zoom in requests.
pub fn svg_zoom_in_system(
  mut commands: Commands,
  requests: Query<Entity, With<SvgZoomInRequest>>,
  mut preview_state: ResMut<SvgPreviewState>,
) {
  for entity in requests.iter() {
    preview_state.zoom_in();
    commands.entity(entity).despawn();
  }
}

/// System to handle SVG zoom out requests.
pub fn svg_zoom_out_system(
  mut commands: Commands,
  requests: Query<Entity, With<SvgZoomOutRequest>>,
  mut preview_state: ResMut<SvgPreviewState>,
) {
  for entity in requests.iter() {
    preview_state.zoom_out();
    commands.entity(entity).despawn();
  }
}

/// System to handle SVG zoom reset requests.
pub fn svg_zoom_reset_system(
  mut commands: Commands,
  requests: Query<Entity, With<SvgZoomResetRequest>>,
  mut preview_state: ResMut<SvgPreviewState>,
) {
  for entity in requests.iter() {
    preview_state.zoom_reset();
    commands.entity(entity).despawn();
  }
}
