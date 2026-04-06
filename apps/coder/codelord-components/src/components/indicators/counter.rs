use codelord_core::ui::component::Counter;

use eframe::egui;

/// Render a counter indicator.
pub fn show(ui: &mut egui::Ui, counter: &Counter) {
  let value = counter.value();

  let text = if value.fract() == 0.0 {
    format!("{} {}", value as i64, counter.unit)
  } else {
    format!("{value:.3} {}", counter.unit)
  };

  ui.label(egui::RichText::new(text).size(10.0));
}
