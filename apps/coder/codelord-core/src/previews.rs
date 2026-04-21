pub mod csv;
pub mod csv_preview;
pub mod font;
pub mod html;
pub mod markdown;
pub mod pdf;
pub mod sqlite;
pub mod svg;
pub mod xls;

pub use csv_preview::CsvPreviewState;
pub use font::FontPreviewState;
pub use html::{DEFAULT_PREVIEW_URL, HtmlPreviewState, WebViewRect};
pub use markdown::MarkdownPreviewState;
pub use pdf::{
  PageLayout, PageText, PdfConnection, PdfDragEvent, PdfNavAction,
  PdfPageCache, PdfPreviewState, PdfQuery, PdfRenderOutput, PdfSelection,
  PdfTextCache, PdfViewData, PdfZoomAction, TextChar, char_range_for_page,
  close_pdf_connection_system, dispatch_pdf_queries_system,
  extract_selected_text, hit_test_char, poll_pdf_results_system,
};
pub use sqlite::{
  SqliteConnection, SqlitePreviewState, close_sqlite_connection_system,
  dispatch_sqlite_queries_system, poll_sqlite_results_system,
};
pub use svg::SvgPreviewState;
pub use xls::XlsPreviewState;

/// Insert preview-related resources (HTML, Markdown, CSV, PDF, SQLite,
/// XLS, Font, SVG).
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(HtmlPreviewState::default());
  world.insert_resource(MarkdownPreviewState::default());
  world.insert_resource(CsvPreviewState::default());
  world.insert_resource(PdfPreviewState::default());
  world.insert_resource(PdfConnection::default());
  world.insert_resource(PdfPageCache::default());
  world.insert_resource(PdfTextCache::default());
  world.insert_resource(PdfSelection::default());
  world.insert_resource(SqlitePreviewState::new());
  world.insert_resource(SqliteConnection::default());
  world.insert_resource(XlsPreviewState::default());
  world.insert_resource(FontPreviewState::default());
  world.insert_resource(SvgPreviewState::default());
}

/// Register preview polling systems (SQLite + PDF async round-trips).
/// Native-only — WebView/rfd don't run on wasm32.
#[cfg(not(target_arch = "wasm32"))]
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    poll_sqlite_results_system,
    dispatch_sqlite_queries_system,
    close_sqlite_connection_system,
  ));
  schedule.add_systems((
    poll_pdf_results_system,
    dispatch_pdf_queries_system,
    close_pdf_connection_system,
  ));
}

#[cfg(target_arch = "wasm32")]
pub fn register_systems(_schedule: &mut crate::ecs::schedule::Schedule) {}
