use crate::components::renderers::webview;
use crate::components::views::copilord;

use codelord_core::ecs::world::World;
use codelord_core::panel::components::RightPanelView;
use codelord_core::panel::resources::RightPanelResource;

use eframe::egui;

/// Shows the right panel content.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let active_view = world
    .get_resource::<RightPanelResource>()
    .map(|r| r.active_view)
    .unwrap_or_default();

  match active_view {
    RightPanelView::Copilord => copilord::show(ui, world),
    RightPanelView::WebView => webview::show(ui, world),
  }
}
