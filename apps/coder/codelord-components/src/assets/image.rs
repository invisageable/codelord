use eframe::egui;

// include_bytes!("../../../codelord-assets/image/image-logo-codelord.png");

// /// Gets the `codelord` app icon.
// pub fn app_icon() -> egui::IconData {
//   let result =
//     image::load_from_memory(LOGO_PATH).map(|dynamic_image| egui::IconData {
//       width: dynamic_image.width(),
//       height: dynamic_image.height(),
//       rgba: dynamic_image.into_rgba8().to_vec(),
//     });

//   match result {
//     Ok(logo) => logo,
//     Err(error) => panic!("{error}"),
//   }
// }

/// Installs loader and images.
pub fn install_images(ctx: &egui::Context) {
  egui_extras::install_image_loaders(ctx);
}

/// Gets the [`egui::Image`] from it's [`egui::ImageSource`].
pub(crate) fn image_from_source<'a>(
  source: impl Into<egui::ImageSource<'a>>,
) -> egui::Image<'a> {
  egui::Image::new(source)
    .maintain_aspect_ratio(true)
    .tint(egui::Color32::WHITE)
}
