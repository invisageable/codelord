//! PDF preview state resource and async communication types.
//!
//! PDF preview renders inline, replacing the code editor with rendered pages.
//! Uses background thread for CPU-intensive rendering operations.
//!
//! ## Architecture
//!
//! - **Resources**: `PdfPreviewState`, `PdfPageCache`, `PdfTextCache`,
//!   `PdfSelection`
//! - **Systems**: Handle background worker communication and state updates
//! - **View types**: `PdfViewData` (input) and `PdfRenderOutput` (events)
//!
//! The component layer receives `PdfViewData` and returns `PdfRenderOutput`.
//! The orchestration layer (coder.rs) bridges resources ↔ view types.

use crate::ecs::world::World;

use bevy_ecs::prelude::Resource;
use rustc_hash::FxHashMap as HashMap;

use std::path::{Path, PathBuf};

// ============================================================================
// Rendered Page Data
// ============================================================================

/// Rendered page as RGBA bitmap.
#[derive(Clone)]
pub struct RenderedPage {
  pub width: u32,
  pub height: u32,
  pub pixels: Vec<u8>,
}

/// Character with position information for text selection.
#[derive(Debug, Clone)]
pub struct TextChar {
  /// The character.
  pub ch: char,
  /// Bounding box in page coordinates (points, 72 DPI).
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

/// Text content of a page with character positions.
#[derive(Debug, Clone, Default)]
pub struct PageText {
  /// All characters with positions.
  pub chars: Vec<TextChar>,
  /// Full text content.
  pub text: String,
}

// ============================================================================
// Query/Result Types (for background worker communication)
// ============================================================================

/// Query request sent to the PDF background worker.
#[derive(Debug, Clone)]
pub enum PdfQuery {
  /// Render a page at given scale.
  RenderPage {
    /// Page index (0-indexed).
    page: usize,
    /// Scale factor (1.0 = 72 DPI).
    scale: f32,
  },
  /// Extract text from a page.
  ExtractText {
    /// Page index (0-indexed).
    page: usize,
  },
}

/// Result received from the PDF background worker.
#[derive(Clone)]
pub enum PdfResult {
  /// PDF loaded successfully with page count.
  Loaded {
    /// Total number of pages.
    page_count: usize,
  },
  /// Rendered page data.
  Page {
    /// Page index that was rendered.
    page: usize,
    /// Scale factor used.
    scale: f32,
    /// Rendered RGBA data.
    data: RenderedPage,
  },
  /// Extracted text data.
  Text {
    /// Page index.
    page: usize,
    /// Text content with character positions.
    data: PageText,
  },
  /// Error message.
  Error(String),
}

// ============================================================================
// Connection Resource
// ============================================================================

/// Resource holding PDF connection channels.
#[derive(Resource, Default)]
pub struct PdfConnection {
  /// Query sender to the background worker.
  pub query_tx: Option<flume::Sender<PdfQuery>>,
  /// Result receiver from the background worker.
  pub result_rx: Option<flume::Receiver<PdfResult>>,
}

impl PdfConnection {
  /// Checks if connection is active.
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
    query_tx: flume::Sender<PdfQuery>,
    result_rx: flume::Receiver<PdfResult>,
  ) {
    self.query_tx = Some(query_tx);
    self.result_rx = Some(result_rx);
  }

  /// Sends a query if connected.
  pub fn send(&self, query: PdfQuery) -> bool {
    if let Some(tx) = &self.query_tx {
      tx.send(query).is_ok()
    } else {
      false
    }
  }
}

// ============================================================================
// Preview State Resource
// ============================================================================

/// Resource for tracking PDF preview state.
///
/// PDF preview replaces the code editor content with rendered PDF pages.
#[derive(Resource, Default)]
pub struct PdfPreviewState {
  /// Whether PDF preview is currently enabled.
  pub enabled: bool,
  /// The file path currently being previewed.
  pub current_file: Option<PathBuf>,
  /// Current page being displayed (0-indexed).
  pub current_page: usize,
  /// Total page count.
  pub page_count: usize,
  /// Zoom level (1.0 = fit width, 2.0 = 200%, etc).
  pub zoom: f32,
  /// Generation counter for detecting content changes.
  pub generation: u64,
  /// Parse/render error if any.
  pub error: Option<String>,
  /// Whether we're waiting for a page render.
  pub is_loading: bool,
  /// Page that needs to be rendered.
  pub pending_render: Option<(usize, f32)>,
}

impl PdfPreviewState {
  /// Opens PDF preview for a file.
  pub fn open(&mut self, file: PathBuf) {
    // Check if we're reopening the same file (e.g., switching back to tab)
    let same_file = self.current_file.as_ref() == Some(&file);

    self.enabled = true;
    self.current_file = Some(file);
    self.error = None;

    // Only reset state if opening a different file
    if !same_file {
      self.current_page = 0;
      self.zoom = 1.0;
      self.generation += 1;
      self.is_loading = true;
      self.pending_render = None;
    }
  }

  /// Sets the page count after loading.
  pub fn set_page_count(&mut self, count: usize) {
    self.page_count = count;
  }

  /// Sets an error message.
  pub fn set_error(&mut self, error: String) {
    self.error = Some(error);
    self.is_loading = false;
  }

  /// Checks if preview is active for a specific file.
  pub fn is_active_for(&self, file: &PathBuf) -> bool {
    self.enabled && self.current_file.as_ref() == Some(file)
  }

  /// Disables preview (when switching tabs). Keeps file for quick restore.
  pub fn disable(&mut self) {
    self.enabled = false;
  }

  /// Closes the preview completely (when tab is closed).
  pub fn close(&mut self) {
    self.enabled = false;
    self.current_file = None;
    self.current_page = 0;
    self.page_count = 0;
    self.zoom = 1.0;
    self.error = None;
    self.is_loading = false;
    self.pending_render = None;
  }

  /// Go to next page.
  pub fn next_page(&mut self) {
    if self.current_page + 1 < self.page_count {
      self.current_page += 1;
    }
  }

  /// Go to previous page.
  pub fn prev_page(&mut self) {
    if self.current_page > 0 {
      self.current_page -= 1;
    }
  }

  /// Go to specific page.
  pub fn go_to_page(&mut self, page: usize) {
    if page < self.page_count {
      self.current_page = page;
    }
  }

  /// Zoom in.
  pub fn zoom_in(&mut self) {
    self.zoom = (self.zoom * 1.25).min(4.0);
    self.generation += 1;
  }

  /// Zoom out.
  pub fn zoom_out(&mut self) {
    self.zoom = (self.zoom / 1.25).max(0.25);
    self.generation += 1;
  }

  /// Reset zoom to fit width.
  pub fn zoom_reset(&mut self) {
    self.zoom = 1.0;
    self.generation += 1;
  }

  /// Request a page render at given scale.
  pub fn request_render(&mut self, page: usize, scale: f32) {
    self.pending_render = Some((page, scale));
  }
}

// ============================================================================
// ECS Systems
// ============================================================================

/// Polls PDF results from background worker and updates preview state.
pub fn poll_pdf_results_system(world: &mut World) {
  // Collect results from channel (non-blocking)
  let results: Vec<PdfResult> = world
    .get_resource::<PdfConnection>()
    .and_then(|conn| conn.result_rx.as_ref())
    .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
    .unwrap_or_default();

  if results.is_empty() {
    return;
  }

  // Collect data to cache (to avoid simultaneous mutable borrows)
  let mut pages_to_cache: Vec<(usize, RenderedPage)> = Vec::new();
  let mut text_to_cache: Vec<(usize, PageText)> = Vec::new();
  let mut current_file: Option<PathBuf> = None;

  // Process results - update state
  if let Some(mut state) = world.get_resource_mut::<PdfPreviewState>() {
    current_file = state.current_file.clone();

    for result in results {
      match result {
        PdfResult::Loaded { page_count } => {
          log::info!("[PDF] Loaded {page_count} pages");
          state.set_page_count(page_count);
          state.is_loading = false;

          // Request first page render
          if page_count > 0 {
            let zoom = state.zoom;
            state.request_render(0, zoom);
          }
        }
        PdfResult::Page { page, data, .. } => {
          log::debug!(
            "[PDF] Cached page {page} ({}x{})",
            data.width,
            data.height
          );
          state.is_loading = false;
          pages_to_cache.push((page, data));
        }
        PdfResult::Text { page, data } => {
          log::debug!(
            "[PDF] Cached text for page {page} ({} chars)",
            data.chars.len()
          );
          text_to_cache.push((page, data));
        }
        PdfResult::Error(msg) => {
          log::error!("[PDF] Error: {msg}");
          state.set_error(msg);
        }
      }
    }
  }

  // Insert pages into cache (separate borrow)
  // Note: GlobalLoading.finish() is called by the renderer when content is
  // displayed
  if !pages_to_cache.is_empty()
    && let Some(file) = &current_file
    && let Some(mut cache) = world.get_resource_mut::<PdfPageCache>()
  {
    for (page, data) in pages_to_cache {
      cache.insert(file, page, data);
    }
  }

  // Insert text into cache
  if !text_to_cache.is_empty()
    && let Some(file) = &current_file
    && let Some(mut cache) = world.get_resource_mut::<PdfTextCache>()
  {
    for (page, data) in text_to_cache {
      cache.insert(file, page, data);
    }
  }
}

/// Dispatches PDF render queries based on pending requests.
pub fn dispatch_pdf_queries_system(world: &mut World) {
  // Check for pending render requests
  let pending = world
    .get_resource::<PdfPreviewState>()
    .filter(|s| s.enabled && !s.is_loading)
    .and_then(|s| s.pending_render);

  let Some((page, scale)) = pending else {
    return;
  };

  // Check if connected
  let is_connected = world
    .get_resource::<PdfConnection>()
    .map(|c| c.is_connected())
    .unwrap_or(false);

  if !is_connected {
    return;
  }

  // Send render request
  let sent = world
    .get_resource::<PdfConnection>()
    .map(|c| c.send(PdfQuery::RenderPage { page, scale }))
    .unwrap_or(false);

  if sent && let Some(mut state) = world.get_resource_mut::<PdfPreviewState>() {
    state.is_loading = true;
    state.pending_render = None;
  }
}

/// Closes PDF connection when file is removed (not just when switching tabs).
pub fn close_pdf_connection_system(world: &mut World) {
  // Only close when file is actually removed (current_file is None)
  // Not when just switching tabs (enabled = false but file still set)
  let should_close = world
    .get_resource::<PdfPreviewState>()
    .map(|s| s.current_file.is_none())
    .unwrap_or(true);

  let is_connected = world
    .get_resource::<PdfConnection>()
    .map(|c| c.is_connected())
    .unwrap_or(false);

  if should_close && is_connected {
    log::info!("[PDF] File removed, closing connection");

    if let Some(mut conn) = world.get_resource_mut::<PdfConnection>() {
      conn.close();
    }

    // Clear page cache
    if let Some(mut cache) = world.get_resource_mut::<PdfPageCache>() {
      cache.clear();
    }
  }
}

// ============================================================================
// Page Cache Resource
// ============================================================================

/// Cache for rendered PDF pages (scale-independent).
/// Keyed by file path to support switching between multiple PDFs.
#[derive(Resource, Default)]
pub struct PdfPageCache {
  /// Cached pages: file path -> (page index -> rendered data).
  pages: HashMap<PathBuf, HashMap<usize, RenderedPage>>,
}

impl PdfPageCache {
  /// Insert a rendered page into the cache for a specific file.
  pub fn insert(&mut self, file: &Path, page: usize, data: RenderedPage) {
    self
      .pages
      .entry(file.to_path_buf())
      .or_default()
      .insert(page, data);
  }

  /// Get a cached page for a specific file if available.
  pub fn get(&self, file: &PathBuf, page: usize) -> Option<&RenderedPage> {
    self.pages.get(file).and_then(|pages| pages.get(&page))
  }

  /// Clear all cached pages for all files.
  pub fn clear(&mut self) {
    self.pages.clear();
  }

  /// Clear cached pages for a specific file.
  pub fn clear_file(&mut self, file: &PathBuf) {
    self.pages.remove(file);
  }

  /// Check if a page is cached for a specific file.
  pub fn has(&self, file: &PathBuf, page: usize) -> bool {
    self
      .pages
      .get(file)
      .map(|p| p.contains_key(&page))
      .unwrap_or(false)
  }
}

// ============================================================================
// Text Cache Resource
// ============================================================================

/// Cache for extracted PDF text with character positions.
/// Keyed by file path to support switching between multiple PDFs.
#[derive(Resource, Default)]
pub struct PdfTextCache {
  /// Cached text: file path -> (page index -> text data).
  text: HashMap<PathBuf, HashMap<usize, PageText>>,
}

impl PdfTextCache {
  /// Insert text data into the cache for a specific file and page.
  pub fn insert(&mut self, file: &Path, page: usize, data: PageText) {
    self
      .text
      .entry(file.to_path_buf())
      .or_default()
      .insert(page, data);
  }

  /// Get cached text for a specific file and page if available.
  pub fn get(&self, file: &PathBuf, page: usize) -> Option<&PageText> {
    self.text.get(file).and_then(|pages| pages.get(&page))
  }

  /// Clear all cached text for all files.
  pub fn clear(&mut self) {
    self.text.clear();
  }

  /// Check if text is cached for a specific file and page.
  pub fn has(&self, file: &PathBuf, page: usize) -> bool {
    self
      .text
      .get(file)
      .map(|p| p.contains_key(&page))
      .unwrap_or(false)
  }
}

// ============================================================================
// Selection State Resource
// ============================================================================

/// Resource for tracking text selection in PDF preview.
#[derive(Resource, Default, Clone)]
pub struct PdfSelection {
  /// Start of selection (page, char index).
  pub start: Option<(usize, usize)>,
  /// End of selection (page, char index).
  pub end: Option<(usize, usize)>,
  /// Whether a drag selection is in progress.
  pub is_selecting: bool,
}

impl PdfSelection {
  /// Start a new selection.
  pub fn start_selection(&mut self, page: usize, char_idx: usize) {
    self.start = Some((page, char_idx));
    self.end = Some((page, char_idx));
    self.is_selecting = true;
  }

  /// Update selection end point.
  pub fn update_selection(&mut self, page: usize, char_idx: usize) {
    if self.is_selecting {
      self.end = Some((page, char_idx));
    }
  }

  /// Finish selection.
  pub fn finish_selection(&mut self) {
    self.is_selecting = false;
  }

  /// Clear selection.
  pub fn clear(&mut self) {
    self.start = None;
    self.end = None;
    self.is_selecting = false;
  }

  /// Check if there's an active selection.
  pub fn has_selection(&self) -> bool {
    self.start.is_some() && self.end.is_some()
  }

  /// Get normalized selection range (start <= end).
  pub fn get_range(&self) -> Option<((usize, usize), (usize, usize))> {
    match (self.start, self.end) {
      (Some(start), Some(end)) => {
        if start <= end {
          Some((start, end))
        } else {
          Some((end, start))
        }
      }
      _ => None,
    }
  }
}

// ============================================================================
// View Data (Input to Component)
// ============================================================================

/// Data snapshot for PDF rendering. Built from resources, passed to component.
pub struct PdfViewData<'a> {
  /// Current file path.
  pub file: &'a Path,
  /// Current page index.
  pub current_page: usize,
  /// Total page count.
  pub page_count: usize,
  /// Zoom level.
  pub zoom: f32,
  /// Whether loading is in progress.
  pub is_loading: bool,
  /// Error message if any.
  pub error: Option<&'a str>,
  /// Generation for cache invalidation.
  pub generation: u64,
  /// Page cache reference.
  pub page_cache: &'a PdfPageCache,
  /// Text cache reference.
  pub text_cache: &'a PdfTextCache,
  /// Current selection range (normalized).
  pub selection: Option<((usize, usize), (usize, usize))>,
}

// ============================================================================
// Render Output (Events from Component)
// ============================================================================

/// Events produced by PDF component. Processed by orchestration layer.
#[derive(Default)]
pub struct PdfRenderOutput {
  /// Navigation action requested.
  pub nav_action: Option<PdfNavAction>,
  /// Zoom action requested.
  pub zoom_action: Option<PdfZoomAction>,
  /// Drag interaction for text selection.
  pub drag_event: Option<PdfDragEvent>,
  /// Whether all pages are ready (for loading indicator).
  pub all_pages_ready: bool,
  /// Whether any content was displayed.
  pub any_content: bool,
}

/// Navigation actions from UI.
#[derive(Debug, Clone, Copy)]
pub enum PdfNavAction {
  PrevPage,
  NextPage,
  GoToPage(usize),
}

/// Zoom actions from UI.
#[derive(Debug, Clone, Copy)]
pub enum PdfZoomAction {
  ZoomIn,
  ZoomOut,
  ZoomReset,
}

/// Drag events for text selection.
#[derive(Debug, Clone, Copy)]
pub enum PdfDragEvent {
  /// Drag started at (page, screen_x, screen_y).
  Started { page: usize, x: f32, y: f32 },
  /// Drag moved to (page, screen_x, screen_y).
  Moved { page: usize, x: f32, y: f32 },
  /// Drag released.
  Released,
}

// ============================================================================
// Hit Testing (Pure Functions)
// ============================================================================

/// Page layout for coordinate transformation.
#[derive(Clone, Copy)]
pub struct PageLayout {
  /// Screen rect min x.
  pub min_x: f32,
  /// Screen rect min y.
  pub min_y: f32,
  /// Scale from page coords to screen coords.
  pub scale: f32,
}

/// Hit test a screen position against text characters on a page.
/// Returns (page_index, char_index) if a character was hit.
pub fn hit_test_char(
  page_idx: usize,
  screen_x: f32,
  screen_y: f32,
  layout: &PageLayout,
  page_text: &PageText,
) -> Option<(usize, usize)> {
  let page_x = (screen_x - layout.min_x) / layout.scale;
  let page_y = (screen_y - layout.min_y) / layout.scale;

  page_text
    .chars
    .iter()
    .enumerate()
    .find(|(_, ch)| {
      page_x >= ch.x
        && page_x <= ch.x + ch.width
        && page_y >= ch.y
        && page_y <= ch.y + ch.height
    })
    .map(|(idx, _)| (page_idx, idx))
}

/// Calculate char range for a page within a selection.
pub fn char_range_for_page(
  page_idx: usize,
  start: (usize, usize),
  end: (usize, usize),
  page_char_count: usize,
) -> Option<(usize, usize)> {
  let (start_page, start_char) = start;
  let (end_page, end_char) = end;

  if page_idx < start_page || page_idx > end_page {
    return None;
  }

  let char_start = if page_idx == start_page {
    start_char
  } else {
    0
  };
  let char_end = if page_idx == end_page {
    end_char
  } else {
    page_char_count.saturating_sub(1)
  };

  Some((char_start, char_end))
}

/// Extract selected text from pages.
pub fn extract_selected_text(
  text_cache: &PdfTextCache,
  file: &Path,
  start: (usize, usize),
  end: (usize, usize),
) -> String {
  let (start_page, _) = start;
  let (end_page, _) = end;
  let mut result = String::new();

  for page_idx in start_page..=end_page {
    let Some(page_text) = text_cache.get(&file.to_path_buf(), page_idx) else {
      continue;
    };
    let Some((char_start, char_end)) =
      char_range_for_page(page_idx, start, end, page_text.chars.len())
    else {
      continue;
    };

    for ch in page_text
      .chars
      .iter()
      .skip(char_start)
      .take(char_end - char_start + 1)
    {
      result.push(ch.ch);
    }

    if page_idx < end_page {
      result.push('\n');
    }
  }

  result
}

/// Returns true if `path` has a PDF extension.
pub fn accepts(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| ext.eq_ignore_ascii_case("pdf"))
    .unwrap_or(false)
}
