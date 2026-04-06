//! Copilord AI assistant view.
//!
//! The AI-powered coding assistant panel for the right side panel.

use codelord_core::ecs::world::World;
use codelord_core::previews::HtmlPreviewState;

use eframe::egui;

/// Shows the Copilord AI assistant interface.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  // Clear WebView rect when showing Copilord
  if let Some(mut s) = world.get_resource_mut::<HtmlPreviewState>() {
    s.webview_rect = None;
  }

  // TODO: Implement full Copilord AI assistant UI
  ui.centered_and_justified(|ui| {
    ui.label(egui::RichText::new("Copilord").size(14.0));
  });
}
