use crate::components::navigation::tabbar;
use crate::components::pages::terminal;

use codelord_core::ecs::world::World;
use codelord_core::terminal::TerminalTab;

use eframe::egui;

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  egui::Panel::top("terminal_tabbar")
    .frame(egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill))
    .exact_size(24.0)
    .resizable(false)
    .show_separator_line(true)
    .show_inside(ui, |ui| tabbar::show::<TerminalTab>(ui, world));

  egui::CentralPanel::default()
    .frame(egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill))
    .show_inside(ui, |ui| terminal::view::show_content(ui, world));
}
