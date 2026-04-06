//! SQLite database client for local database preview.
//!
//! Provides async database operations running in background tasks,
//! communicating results via channels.
//!
//! **Architecture:**
//! ```text
//! open_database(path)
//!   ↓
//! Returns (query_tx, result_rx) channels
//!   ↓
//! Send SqliteQuery via query_tx
//!   ↓
//! Background task processes query
//!   ↓
//! Receive SqliteResult via result_rx
//! ```

use codelord_core::previews::sqlite::{
  ColumnInfo, QueryResult, SqliteQuery, SqliteResult, TableInfo,
};

use sqlx::sqlite::SqlitePool;
use sqlx::{Column, Row};

/// Maximum page size to prevent abuse.
const MAX_PAGE_SIZE: usize = 10_000;

/// Maximum page number to prevent overflow.
const MAX_PAGE: usize = 1_000_000;

/// Validates and escapes a table name to prevent SQL injection.
///
/// SQLite identifiers can be quoted with double quotes, and any embedded
/// double quotes must be escaped by doubling them.
fn escape_identifier(name: &str) -> Result<String, String> {
  // Reject empty names
  if name.is_empty() {
    return Err("Empty identifier".to_string());
  }

  // Reject excessively long names (SQLite limit is ~1GB but be reasonable)
  if name.len() > 128 {
    return Err("Identifier too long".to_string());
  }

  // Escape double quotes by doubling them and wrap in quotes
  let escaped = name.replace('"', "\"\"");
  Ok(format!("\"{}\"", escaped))
}

/// Opens a SQLite database and spawns a background worker.
///
/// Returns channels for sending queries and receiving results.
/// The worker runs until the query sender is dropped.
pub async fn open_database(
  path: &str,
  runtime: &tokio::runtime::Handle,
) -> Result<(flume::Sender<SqliteQuery>, flume::Receiver<SqliteResult>), String>
{
  // Use read-only mode for safety
  let url = format!("sqlite:{}?mode=ro", path);

  let pool = SqlitePool::connect(&url)
    .await
    .map_err(|e| format!("Failed to open database: {e}"))?;

  let (query_tx, query_rx) = flume::unbounded::<SqliteQuery>();
  let (result_tx, result_rx) = flume::unbounded::<SqliteResult>();

  // Spawn background worker
  runtime.spawn(async move {
    run_worker(pool, query_rx, result_tx).await;
  });

  Ok((query_tx, result_rx))
}

/// Background worker that processes SQLite queries.
async fn run_worker(
  pool: SqlitePool,
  query_rx: flume::Receiver<SqliteQuery>,
  result_tx: flume::Sender<SqliteResult>,
) {
  log::info!("[SqliteClient] Worker started");

  while let Ok(query) = query_rx.recv_async().await {
    let result = match query {
      SqliteQuery::LoadTables => load_tables(&pool).await,
      SqliteQuery::LoadTableData {
        table,
        page,
        page_size,
      } => load_table_data(&pool, &table, page, page_size).await,
      SqliteQuery::ExecuteSql(sql) => execute_sql(&pool, &sql).await,
    };

    if result_tx.send(result).is_err() {
      log::warn!("[SqliteClient] Result channel closed");
      break;
    }
  }

  log::info!("[SqliteClient] Worker stopped");
}

/// Loads all table names and metadata from the database.
async fn load_tables(pool: &SqlitePool) -> SqliteResult {
  let tables_result: Result<Vec<String>, _> = sqlx::query_scalar(
    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
  )
  .fetch_all(pool)
  .await;

  let table_names = match tables_result {
    Ok(names) => names,
    Err(e) => {
      return SqliteResult::Error(format!("Failed to load tables: {e}"));
    }
  };

  let mut tables = Vec::with_capacity(table_names.len());

  for name in table_names {
    let columns = load_columns(pool, &name).await;
    let row_count = count_rows(pool, &name).await;

    tables.push(TableInfo {
      name,
      columns,
      row_count,
    });
  }

  SqliteResult::Tables(tables)
}

/// Loads column metadata for a table.
async fn load_columns(pool: &SqlitePool, table: &str) -> Vec<ColumnInfo> {
  let safe_table = match escape_identifier(table) {
    Ok(t) => t,
    Err(_) => return Vec::new(),
  };

  let query = format!("PRAGMA table_info({safe_table})");

  let rows = match sqlx::query(&query).fetch_all(pool).await {
    Ok(rows) => rows,
    Err(_) => return Vec::new(),
  };

  rows
    .into_iter()
    .map(|row| {
      let name: String = row.get(1);
      let dtype: String = row.get(2);
      let not_null: i32 = row.get(3);
      let default_val: Option<String> = row.get(4);
      let pk: i32 = row.get(5);

      ColumnInfo {
        name,
        dtype,
        is_not_null: not_null == 1,
        is_pk: pk > 0,
        default_value: default_val,
      }
    })
    .collect()
}

/// Counts rows in a table.
async fn count_rows(pool: &SqlitePool, table: &str) -> u64 {
  let safe_table = match escape_identifier(table) {
    Ok(t) => t,
    Err(_) => return 0,
  };

  let query = format!("SELECT COUNT(*) FROM {safe_table}");

  sqlx::query_scalar::<_, i64>(&query)
    .fetch_one(pool)
    .await
    .unwrap_or(0) as u64
}

/// Loads table data with pagination.
async fn load_table_data(
  pool: &SqlitePool,
  table: &str,
  page: usize,
  page_size: usize,
) -> SqliteResult {
  // Validate pagination parameters to prevent overflow
  if page > MAX_PAGE || page_size > MAX_PAGE_SIZE {
    return SqliteResult::Error("Invalid pagination parameters".to_string());
  }

  let offset = match page.checked_mul(page_size) {
    Some(o) => o,
    None => return SqliteResult::Error("Pagination overflow".to_string()),
  };

  let safe_table = match escape_identifier(table) {
    Ok(t) => t,
    Err(e) => return SqliteResult::Error(e),
  };

  let total_rows = count_rows(pool, table).await;

  let query =
    format!("SELECT * FROM {safe_table} LIMIT {page_size} OFFSET {offset}");

  match sqlx::query(&query).fetch_all(pool).await {
    Ok(rows) => {
      if rows.is_empty() {
        return SqliteResult::Data {
          result: QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            error: None,
          },
          total_rows,
        };
      }

      let columns = rows
        .first()
        .map(|r| {
          r.columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect::<Vec<_>>()
        })
        .unwrap_or_default();

      let data = rows
        .iter()
        .map(|row| {
          (0..row.len())
            .map(|i| get_column_as_string(row, i))
            .collect()
        })
        .collect::<Vec<_>>();

      SqliteResult::Data {
        result: QueryResult {
          columns,
          rows: data,
          error: None,
        },
        total_rows,
      }
    }
    Err(e) => SqliteResult::Error(format!("Query failed: {e}")),
  }
}

/// Executes custom SQL query.
async fn execute_sql(pool: &SqlitePool, sql: &str) -> SqliteResult {
  match sqlx::query(sql).fetch_all(pool).await {
    Ok(rows) => {
      if rows.is_empty() {
        return SqliteResult::CustomSqlResult(QueryResult {
          columns: Vec::new(),
          rows: Vec::new(),
          error: None,
        });
      }

      let columns = rows
        .first()
        .map(|r| {
          r.columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect::<Vec<_>>()
        })
        .unwrap_or_default();

      let data = rows
        .iter()
        .map(|row| {
          (0..row.len())
            .map(|i| get_column_as_string(row, i))
            .collect()
        })
        .collect::<Vec<_>>();

      SqliteResult::CustomSqlResult(QueryResult {
        columns,
        rows: data,
        error: None,
      })
    }
    Err(e) => SqliteResult::CustomSqlResult(QueryResult {
      columns: Vec::new(),
      rows: Vec::new(),
      error: Some(format!("SQL error: {e}")),
    }),
  }
}

/// Extracts a column value as a string, handling different SQLite types.
fn get_column_as_string(row: &sqlx::sqlite::SqliteRow, index: usize) -> String {
  if let Ok(val) = row.try_get::<String, _>(index) {
    return val;
  }
  if let Ok(val) = row.try_get::<i64, _>(index) {
    return val.to_string();
  }
  if let Ok(val) = row.try_get::<f64, _>(index) {
    return val.to_string();
  }
  if let Ok(val) = row.try_get::<Vec<u8>, _>(index) {
    return format!("[BLOB {} bytes]", val.len());
  }

  "NULL".to_string()
}
