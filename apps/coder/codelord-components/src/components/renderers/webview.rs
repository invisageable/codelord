//! WebView renderer for HTML preview.
//!
//! Allocates UI space and updates HtmlPreviewState with the rect bounds.
//! The actual WebView is managed by coder.rs (outside ECS) because
//! wry::WebView is !Send+!Sync.

use codelord_core::ecs::world::World;
use codelord_core::previews::{HtmlPreviewState, WebViewRect};

use eframe::egui;

/// Renders the WebView placeholder and updates bounds in HtmlPreviewState.
///
/// The actual WebView rendering happens in coder.rs using the bounds
/// stored in HtmlPreviewState.webview_rect.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  // Allocate space for HTML preview WebView
  // The actual WebView bounds are updated by the app (not ECS)
  // because wry::WebView is !Send+!Sync
  let (rect, _response) =
    ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());

  // Update HtmlPreviewState with the rect for WebView bounds
  // The rect is in screen coordinates from allocate_exact_size
  if let Some(mut s) = world.get_resource_mut::<HtmlPreviewState>() {
    s.webview_rect = Some(WebViewRect {
      x: rect.min.x as f64,
      y: rect.min.y as f64,
      width: rect.width() as f64,
      height: rect.height() as f64,
    });
  }
}
