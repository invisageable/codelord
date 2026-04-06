use codelord_coder::Coder;

fn main() -> eframe::Result<()> {
  env_logger::init();

  eframe::run_native(
    "codelord",
    eframe::NativeOptions {
      viewport: eframe::egui::ViewportBuilder::default()
        .with_decorations(false)
        .with_inner_size([1024.0, 600.0])
        .with_min_inner_size([400.0, 240.0])
        .with_transparent(true),
      ..Default::default()
    },
    Box::new(|cc| Ok(Box::new(Coder::new(cc)))),
  )
}
