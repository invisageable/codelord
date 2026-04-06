use crate::components::views;

use codelord_core::ecs::world::World;
use codelord_core::panel::components::BottomPanelView;
use codelord_core::panel::resources::BottomPanelResource;

use eframe::egui;

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let active_view = world
    .get_resource::<BottomPanelResource>()
    .map(|r| r.active_view)
    .unwrap_or_default();

  match active_view {
    BottomPanelView::Terminal => views::terminal::show(ui, world),
    BottomPanelView::Problems => {}
    BottomPanelView::Output => {}
  }
}
