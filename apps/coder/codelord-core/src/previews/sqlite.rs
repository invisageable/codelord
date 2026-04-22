//! SQLite preview state and systems.
//!
//! Provides database preview for .sqlite, .sqlite3, .db files.
//! Uses async sqlx for database operations with flume channels.

use bevy_ecs::component::Component;
use bevy_ecs::prelude::Resource;

use crate::ecs::world::World;

use std::path::PathBuf;

/// Active tab in the SQLite preview panel.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SqliteTab {
  /// Data view showing table rows with pagination.
  #[default]
  Data,
  /// Schema view showing column definitions and constraints.
  Schema,
  /// Custom SQL query editor and results.
  Sql,
}

/// Column metadata extracted from database schema via PRAGMA table_info.
#[derive(Clone, Debug)]
pub struct ColumnInfo {
  /// Column name as defined in the table schema.
  pub name: String,
  /// SQLite data type (TEXT, INTEGER, REAL, BLOB, NULL).
  pub dtype: String,
  /// Whether this column is part of the primary key.
  pub is_pk: bool,
  /// Whether this column has a NOT NULL constraint.
  pub is_not_null: bool,
  /// Default value expression if any.
  pub default_value: Option<String>,
}

/// Table metadata loaded from sqlite_master.
#[derive(Clone, Debug)]
pub struct TableInfo {
  /// Table name as stored in the database.
  pub name: String,
  /// List of column definitions for this table.
  pub columns: Vec<ColumnInfo>,
  /// Total number of rows in the table (from COUNT(*)).
  pub row_count: u64,
}

/// Query result containing column headers and row data.
#[derive(Default, Clone, Debug)]
pub struct QueryResult {
  /// Column names from the result set header.
  pub columns: Vec<String>,
  /// Row data as strings for display (each inner Vec is one row).
  pub rows: Vec<Vec<String>>,
  /// Error message if the query failed.
  pub error: Option<String>,
}

/// Filter operator for column-based filtering.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
  /// LIKE '%value%' - substring match.
  #[default]
  Contains,
  /// = value - exact match.
  Equals,
  /// != value - not equal.
  NotEquals,
  /// > value - greater than (numeric comparison).
  GreaterThan,
  /// < value - less than (numeric comparison).
  LessThan,
}

/// Single filter condition applied to a column.
#[derive(Default, Clone)]
pub struct Filter {
  /// Column name to filter on.
  pub column: String,
  /// Comparison operator to use.
  pub op: FilterOp,
  /// Value to compare against.
  pub value: String,
}

/// Main preview state resource for SQLite database viewer.
///
/// This resource is the single source of truth for the SQLite preview UI.
/// It tracks the current database, selected table, pagination, and query
/// results.
#[derive(Resource, Default)]
pub struct SqlitePreviewState {
  /// Whether the SQLite preview panel is currently visible.
  pub enabled: bool,
  /// Path to the currently opened database file.
  pub current_file: Option<PathBuf>,

  /// List of tables in the database (loaded on open).
  pub tables: Vec<TableInfo>,
  /// Index of the currently selected table in `tables` vector.
  pub selected_table: Option<usize>,

  /// Currently active tab (Data, Schema, or SQL).
  pub active_tab: SqliteTab,
  /// Text filter applied to search within data.
  pub search_query: String,
  /// Column-based filters applied to the query.
  pub filters: Vec<Filter>,

  /// Current page number (0-indexed) for pagination.
  pub current_page: usize,
  /// Number of rows to display per page.
  pub page_size: usize,
  /// Query result data for the selected table.
  pub data: QueryResult,
  /// Total row count in the selected table (for pagination).
  pub total_rows: u64,

  /// Custom SQL query text entered by the user.
  pub custom_sql: String,
  /// Result from executing the custom SQL query.
  pub custom_sql_result: QueryResult,

  /// Flag indicating data needs to be reloaded from the database.
  pub needs_reload: bool,
  /// Flag indicating an async query is in progress.
  pub is_loading: bool,
}

impl SqlitePreviewState {
  /// Creates a new SQLite preview state with default page size of 100.
  pub fn new() -> Self {
    Self {
      page_size: 100,
      ..Default::default()
    }
  }

  /// Toggles SQLite preview for a file.
  ///
  /// Returns `true` if preview was enabled, `false` if disabled.
  pub fn toggle(&mut self, file: PathBuf) -> bool {
    if self.enabled && self.current_file.as_ref() == Some(&file) {
      self.close();
      false
    } else {
      self.enabled = true;
      self.current_file = Some(file);
      self.needs_reload = true;
      true
    }
  }

  /// Closes the preview and resets all state.
  pub fn close(&mut self) {
    self.enabled = false;
    self.current_file = None;
    self.tables.clear();
    self.selected_table = None;
    self.active_tab = SqliteTab::Data;
    self.search_query.clear();
    self.filters.clear();
    self.current_page = 0;
    self.data = QueryResult::default();
    self.custom_sql.clear();
    self.custom_sql_result = QueryResult::default();
    self.needs_reload = false;
    self.is_loading = false;
    self.total_rows = 0;
  }

  /// Checks if preview is active for a specific file.
  pub fn is_active_for(&self, file: &PathBuf) -> bool {
    self.enabled && self.current_file.as_ref() == Some(file)
  }

  /// Selects a table by index and triggers data reload.
  pub fn select_table(&mut self, index: usize) {
    if index < self.tables.len() && self.selected_table != Some(index) {
      self.selected_table = Some(index);
      self.current_page = 0;
      self.needs_reload = true;
    }
  }

  /// Advances to the next page if available.
  pub fn next_page(&mut self) {
    let total_pages = self.total_pages();
    if self.current_page + 1 < total_pages {
      self.current_page += 1;
      self.needs_reload = true;
    }
  }

  /// Returns to the previous page if available.
  pub fn prev_page(&mut self) {
    if self.current_page > 0 {
      self.current_page -= 1;
      self.needs_reload = true;
    }
  }

  /// Calculates total number of pages based on row count and page size.
  pub fn total_pages(&self) -> usize {
    if self.page_size == 0 {
      return 1;
    }
    (self.total_rows as usize).div_ceil(self.page_size).max(1)
  }
}

// ============================================================================
// Events (ECS Components for request-based communication)
// ============================================================================

/// Request to select a table by its index in the tables list.
#[derive(Component, Debug, Clone, Copy)]
pub struct SelectTableRequest(
  /// Index of the table to select.
  pub usize,
);

/// Request to execute the custom SQL query.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ExecuteSqlRequest;

/// Request to navigate to a specific page.
#[derive(Component, Debug, Clone, Copy)]
pub struct ChangePageRequest(
  /// Target page number (0-indexed).
  pub usize,
);

/// Request to add a new filter condition.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AddFilterRequest;

/// Request to remove a filter by its index.
#[derive(Component, Debug, Clone, Copy)]
pub struct RemoveFilterRequest(
  /// Index of the filter to remove.
  pub usize,
);

/// Request to export data in a specific format.
#[derive(Component, Debug, Clone, Copy)]
pub enum ExportRequest {
  /// Export as comma-separated values.
  Csv,
  /// Export as JSON array of objects.
  Json,
}

// ============================================================================
// SQLite Query Types (for async worker communication)
// ============================================================================

/// Query request sent to the async SQLite worker.
#[derive(Debug, Clone)]
pub enum SqliteQuery {
  /// Load all table names and metadata from the database.
  LoadTables,
  /// Load paginated data from a specific table.
  LoadTableData {
    /// Name of the table to query.
    table: String,
    /// Page number (0-indexed).
    page: usize,
    /// Number of rows per page.
    page_size: usize,
  },
  /// Execute a custom SQL query.
  ExecuteSql(String),
}

/// Query result received from the async SQLite worker.
#[derive(Debug, Clone)]
pub enum SqliteResult {
  /// List of tables loaded from the database.
  Tables(Vec<TableInfo>),
  /// Paginated table data with total row count.
  Data {
    /// The query result containing columns and rows.
    result: QueryResult,
    /// Total number of rows in the table.
    total_rows: u64,
  },
  /// Result from executing a custom SQL query.
  CustomSqlResult(QueryResult),
  /// Error message from a failed query.
  Error(String),
}

// ============================================================================
// Connection Resource (holds channels for async communication)
// ============================================================================

/// Resource holding SQLite connection channels.
///
/// This Resource is separate from SqlitePreviewState so that:
/// 1. Connection state can be managed independently
/// 2. ECS systems can access channels without borrowing the entire preview
///    state
#[derive(Resource, Default)]
pub struct SqliteConnection {
  /// Query sender to the background worker.
  pub query_tx: Option<flume::Sender<SqliteQuery>>,
  /// Result receiver from the background worker.
  pub result_rx: Option<flume::Receiver<SqliteResult>>,
}

impl SqliteConnection {
  /// Checks if connection is active (channels exist).
  pub fn is_connected(&self) -> bool {
    self.query_tx.is_some() && self.result_rx.is_some()
  }

  /// Closes the connection by dropping channels.
  pub fn close(&mut self) {
    self.query_tx = None;
    self.result_rx = None;
  }

  /// Sets the connection channels.
  pub fn set(
    &mut self,
    query_tx: flume::Sender<SqliteQuery>,
    result_rx: flume::Receiver<SqliteResult>,
  ) {
    self.query_tx = Some(query_tx);
    self.result_rx = Some(result_rx);
  }

  /// Sends a query if connected.
  pub fn send(&self, query: SqliteQuery) -> bool {
    if let Some(tx) = &self.query_tx {
      tx.send(query).is_ok()
    } else {
      false
    }
  }
}

// ============================================================================
// ECS Systems
// ============================================================================

/// Polls SQLite results from background worker and updates preview state.
///
/// This system runs every frame and non-blockingly checks for results.
pub fn poll_sqlite_results_system(world: &mut World) {
  // Collect results from channel (non-blocking)
  let results: Vec<SqliteResult> = world
    .get_resource::<SqliteConnection>()
    .and_then(|conn| conn.result_rx.as_ref())
    .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
    .unwrap_or_default();

  if results.is_empty() {
    return;
  }

  // Get connection to send follow-up queries
  let query_tx = world
    .get_resource::<SqliteConnection>()
    .and_then(|conn| conn.query_tx.clone());

  // Process results
  let Some(mut state) = world.get_resource_mut::<SqlitePreviewState>() else {
    return;
  };

  for result in results {
    match result {
      SqliteResult::Tables(tables) => {
        log::info!("[SQLite] Loaded {} tables", tables.len());

        state.tables = tables;
        state.is_loading = false;
        state.needs_reload = false;

        // Auto-select first table and request data
        if !state.tables.is_empty() && state.selected_table.is_none() {
          state.selected_table = Some(0);

          // Request data for first table
          if let Some(tx) = &query_tx {
            let table_name = state.tables[0].name.clone();
            let page = state.current_page;
            let page_size = state.page_size;

            let _ = tx.send(SqliteQuery::LoadTableData {
              table: table_name,
              page,
              page_size,
            });

            state.is_loading = true;
          }
        }
      }
      SqliteResult::Data { result, total_rows } => {
        log::info!(
          "[SQLite] Loaded {} rows ({total_rows} total)",
          result.rows.len(),
        );

        state.data = result;
        state.total_rows = total_rows;
        state.is_loading = false;
        state.needs_reload = false;
      }
      SqliteResult::CustomSqlResult(result) => {
        log::info!("[SQLite] Custom SQL result: {} rows", result.rows.len());

        state.custom_sql_result = result;
        state.is_loading = false;
      }
      SqliteResult::Error(msg) => {
        log::error!("[SQLite] Error: {msg}");

        state.data.error = Some(msg);
        state.is_loading = false;
        state.needs_reload = false;
      }
    }
  }
}

/// Dispatches SQLite queries based on preview state.
///
/// This system checks if data needs to be loaded and sends appropriate queries.
/// Note: Database opening is still handled by Coder since it needs runtime
/// access.
pub fn dispatch_sqlite_queries_system(world: &mut World) {
  // Check if we need to load data (not opening new db - that's handled by
  // Coder)
  let action = world
    .get_resource::<SqlitePreviewState>()
    .filter(|s| s.enabled && s.needs_reload && !s.is_loading)
    .map(|s| {
      (
        s.selected_table,
        s.tables
          .get(s.selected_table.unwrap_or(0))
          .map(|t| t.name.clone()),
        s.current_page,
        s.page_size,
        s.active_tab,
        s.custom_sql.clone(),
      )
    });

  let Some((
    selected_table,
    table_name,
    page,
    page_size,
    active_tab,
    custom_sql,
  )) = action
  else {
    return;
  };

  // Check if connected
  let is_connected = world
    .get_resource::<SqliteConnection>()
    .map(|c| c.is_connected())
    .unwrap_or(false);

  if !is_connected {
    // Not connected - database opening is handled by Coder
    return;
  }

  // Send query based on active tab
  let query =
    if matches!(active_tab, SqliteTab::Sql) && !custom_sql.trim().is_empty() {
      log::info!(
        "[SQLite] Executing custom SQL: {}",
        custom_sql.chars().take(50).collect::<String>()
      );

      Some(SqliteQuery::ExecuteSql(custom_sql))
    } else if selected_table.is_some() {
      if let Some(name) = table_name {
        log::info!("[SQLite] Loading table data: {name}");

        Some(SqliteQuery::LoadTableData {
          table: name,
          page,
          page_size,
        })
      } else {
        None
      }
    } else {
      None
    };

  // Send the query
  if let Some(query) = query {
    let sent = world
      .get_resource::<SqliteConnection>()
      .map(|c| c.send(query))
      .unwrap_or(false);

    if sent
      && let Some(mut state) = world.get_resource_mut::<SqlitePreviewState>()
    {
      state.is_loading = true;
      state.needs_reload = false;
    }
  }
}

/// Closes SQLite connection when preview is disabled.
pub fn close_sqlite_connection_system(world: &mut World) {
  let preview_disabled = world
    .get_resource::<SqlitePreviewState>()
    .map(|s| !s.enabled)
    .unwrap_or(true);

  let is_connected = world
    .get_resource::<SqliteConnection>()
    .map(|c| c.is_connected())
    .unwrap_or(false);

  if preview_disabled && is_connected {
    log::info!("[SQLite] Preview disabled, closing connection");

    if let Some(mut conn) = world.get_resource_mut::<SqliteConnection>() {
      conn.close();
    }
  }
}

/// Returns true if `path` has a SQLite database extension.
pub fn accepts(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| {
      matches!(ext.to_lowercase().as_str(), "db" | "sqlite" | "sqlite3")
    })
    .unwrap_or(false)
}

/// Spawn the SQLite export dropdown popup (CSV / JSON) and register its
/// entity in [`crate::popup::resources::PopupResource`].
pub fn spawn_export_popup(world: &mut crate::ecs::world::World) {
  use crate::popup::components::{
    MenuItem, Popup, PopupContent, PopupPosition,
  };
  use crate::popup::resources::PopupResource;

  let menu = PopupContent::Menu(vec![
    MenuItem::new("export_csv", "Export as CSV"),
    MenuItem::new("export_json", "Export as JSON"),
  ]);

  let entity = world
    .spawn(Popup::new(menu).with_position(PopupPosition::Below))
    .id();

  if let Some(mut popup_res) = world.get_resource_mut::<PopupResource>() {
    popup_res.sqlite_export_popup = Some(entity);
  }
}
