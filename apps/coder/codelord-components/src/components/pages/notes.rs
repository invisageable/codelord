use codelord_core::ecs::world::World;

use eframe::egui;

pub fn show(ui: &mut egui::Ui, _world: &mut World) {
  ui.vertical_centered(|ui| {
    ui.add_space(100.0);
    ui.heading("Notes");
    ui.label("Click the Code icon to see this page");
  });
}
