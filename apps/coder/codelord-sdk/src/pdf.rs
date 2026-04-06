//! PDF renderer client for async page rendering and text extraction.
//!
//! Provides async PDF operations running in background thread,
//! communicating results via channels.
//!
//! **Architecture:**
//! ```text
//! open_pdf(path)
//!   ↓
//! Returns (query_tx, result_rx) channels
//!   ↓
//! Background thread pre-renders ALL pages in parallel
//! AND extracts text for text selection support
//!   ↓
//! Receive PdfResult via result_rx as pages complete
//! ```

use codelord_core::previews::pdf::{
  PageText, PdfQuery, PdfResult, RenderedPage, TextChar,
};

use rayon::prelude::*;
use std::path::Path;
use std::sync::Arc;
use swisskit::renderer::pdf::renderer::PdfRenderer;

/// High-resolution render scale for pre-rendering.
/// Render at 3x for crisp display on all screens, scale down if needed.
const RENDER_SCALE: f32 = 3.0;

/// Opens a PDF file and spawns a background worker thread.
///
/// Returns channels for sending queries and receiving results.
/// The worker immediately pre-renders all pages and extracts text in parallel.
pub fn open_pdf(
  path: &Path,
) -> Result<(flume::Sender<PdfQuery>, flume::Receiver<PdfResult>), String> {
  // Read and parse on current thread to get immediate error feedback
  let renderer = PdfRenderer::from_path(path)
    .map_err(|e| format!("Failed to load PDF: {e}"))?;

  let page_count = renderer.page_count();

  let (query_tx, _query_rx) = flume::unbounded::<PdfQuery>();
  let (result_tx, result_rx) = flume::unbounded::<PdfResult>();

  // Send initial page count immediately
  let _ = result_tx.send(PdfResult::Loaded { page_count });

  // Wrap renderer in Arc for sharing across threads
  let renderer = Arc::new(renderer);

  // Pre-render all pages and extract text in parallel using rayon
  let result_tx_clone = result_tx.clone();
  let renderer_clone = renderer.clone();
  rayon::spawn(move || {
    pre_render_all_pages(
      &renderer_clone,
      page_count,
      RENDER_SCALE,
      &result_tx_clone,
    );
  });

  // Extract text for all pages in parallel (for text selection)
  rayon::spawn(move || {
    extract_all_text(&renderer, page_count, &result_tx);
  });

  Ok((query_tx, result_rx))
}

/// Pre-renders all pages in parallel using rayon.
fn pre_render_all_pages(
  renderer: &Arc<PdfRenderer>,
  page_count: usize,
  scale: f32,
  result_tx: &flume::Sender<PdfResult>,
) {
  log::info!("[PDF] Pre-rendering {page_count} pages at scale {scale}");

  // Render all pages in parallel
  (0..page_count).into_par_iter().for_each(|page| {
    let result = render_page(renderer, page, scale);
    let _ = result_tx.send(result);
  });

  log::info!("[PDF] Pre-rendering complete");
}

/// Renders a single page to RGBA bitmap.
fn render_page(renderer: &PdfRenderer, page: usize, scale: f32) -> PdfResult {
  match renderer.render_page(page, scale) {
    Ok(rendered) => PdfResult::Page {
      page,
      scale,
      data: RenderedPage {
        width: rendered.width,
        height: rendered.height,
        pixels: rendered.pixels,
      },
    },
    Err(e) => PdfResult::Error(format!("Failed to render page {page}: {e}")),
  }
}

/// Extracts text from all pages in parallel using rayon.
fn extract_all_text(
  renderer: &Arc<PdfRenderer>,
  page_count: usize,
  result_tx: &flume::Sender<PdfResult>,
) {
  log::info!("[PDF] Extracting text from {page_count} pages");

  // Extract text from all pages in parallel
  (0..page_count).into_par_iter().for_each(|page| {
    let result = extract_text(renderer, page);
    let _ = result_tx.send(result);
  });

  log::info!("[PDF] Text extraction complete");
}

/// Extracts text from a single page.
fn extract_text(renderer: &PdfRenderer, page: usize) -> PdfResult {
  match renderer.extract_text(page) {
    Ok(text) => PdfResult::Text {
      page,
      data: PageText {
        chars: text
          .chars
          .into_iter()
          .map(|c| TextChar {
            ch: c.ch,
            x: c.x,
            y: c.y,
            width: c.width,
            height: c.height,
          })
          .collect(),
        text: text.text,
      },
    },
    Err(e) => {
      log::warn!("[PDF] Failed to extract text from page {page}: {e}");
      // Return empty text on error (not fatal)
      PdfResult::Text {
        page,
        data: PageText::default(),
      }
    }
  }
}
