//! HTML preview serving endpoints.
//!
//! Serves HTML files with proper MIME types and no-cache headers
//! for hot reloading during development.

use crate::state::ServerState;

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::{Json, Router, routing};
use serde::Deserialize;

use std::sync::Arc;

#[derive(Deserialize)]
struct SetFileRequest {
  file_path: String,
}

/// Validates a file path to prevent path traversal attacks.
///
/// Returns the canonicalized path if valid, or an error status.
fn validate_file_path(path: &str) -> Result<String, StatusCode> {
  // Canonicalize to resolve symlinks and ".." components
  let abs_path =
    std::fs::canonicalize(path).map_err(|_| StatusCode::BAD_REQUEST)?;

  // Must be an HTML file
  let extension = abs_path.extension().and_then(|e| e.to_str());
  if !matches!(extension, Some("html") | Some("htm")) {
    return Err(StatusCode::BAD_REQUEST);
  }

  // Convert back to string
  abs_path
    .to_str()
    .map(|s| s.to_string())
    .ok_or(StatusCode::BAD_REQUEST)
}

pub fn router(state: Arc<ServerState>) -> Router {
  Router::new()
    .route("/", routing::get(serve_html))
    .route("/set", routing::post(set_file))
    .route("/playground", routing::get(serve_playground_html))
    .with_state(state)
}

/// Serves the current HTML file from shared state.
async fn serve_html(
  State(state): State<Arc<ServerState>>,
) -> Result<Response<String>, (StatusCode, String)> {
  let file_path = state.current_html_file.lock().await;

  if file_path.is_empty() {
    return Err((StatusCode::NOT_FOUND, "No HTML file loaded".to_string()));
  }

  tracing::debug!("[Preview] Serving: {file_path}");

  let content = tokio::fs::read_to_string(&*file_path).await.map_err(|e| {
    // Log full details internally, return generic message to client
    tracing::error!("[Preview] Failed to read file: {e}");
    (StatusCode::NOT_FOUND, "Resource not found".to_string())
  })?;

  Ok(
    Response::builder()
      .status(StatusCode::OK)
      .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
      .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
      .body(content)
      .unwrap(),
  )
}

/// Sets the current HTML file to preview.
async fn set_file(
  State(state): State<Arc<ServerState>>,
  Json(req): Json<SetFileRequest>,
) -> StatusCode {
  // Validate and canonicalize the path
  let validated_path = match validate_file_path(&req.file_path) {
    Ok(p) => p,
    Err(status) => {
      tracing::warn!("[Preview] Invalid file path: {}", req.file_path);
      return status;
    }
  };

  tracing::debug!("[Preview] Setting file to: {validated_path}");

  let mut file_path = state.current_html_file.lock().await;
  *file_path = validated_path;

  StatusCode::OK
}

/// Serves the playground-generated HTML from compilation.
async fn serve_playground_html(
  State(state): State<Arc<ServerState>>,
) -> Result<Response<String>, (StatusCode, String)> {
  let html = state.preview_html.lock().await;

  if html.is_empty() {
    return Err((
      StatusCode::NOT_FOUND,
      "No playground HTML generated. Run compilation in Ui mode first."
        .to_string(),
    ));
  }

  tracing::debug!("[Preview] Serving playground HTML ({} bytes)", html.len());

  Ok(
    Response::builder()
      .status(StatusCode::OK)
      .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
      .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
      .body(html.clone())
      .unwrap(),
  )
}
