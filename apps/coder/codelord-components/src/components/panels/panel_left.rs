use crate::components::navigation;

use codelord_core::ecs::world::World;
use codelord_core::panel::components::LeftPanelView;
use codelord_core::panel::resources::LeftPanelResource;

use eframe::egui;

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let active_view = world
    .get_resource::<LeftPanelResource>()
    .map(|r| r.active_view)
    .unwrap_or_default();

  match active_view {
    LeftPanelView::Explorer => navigation::explorer::show(ui, world),
    LeftPanelView::Collaboration => {}
    LeftPanelView::VersionControl => {}
  }
}
