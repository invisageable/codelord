//! SVG renderer.
//!
//! Renders SVG files using resvg/usvg and displays as egui texture.

use eframe::egui::{self, Color32, RichText, TextureHandle, Ui};
use resvg::tiny_skia::Pixmap;

/// Cached rendered SVG texture.
pub struct SvgTextureCache {
  pub texture: TextureHandle,
  pub generation: u64,
  /// Logical size (for display).
  pub logical_size: (f32, f32),
}

/// Non-Send resource wrapper for SVG texture cache.
#[derive(Default)]
pub struct SvgTextureCacheResource(pub Option<SvgTextureCache>);

/// View data for SVG preview rendering (borrowed).
pub struct SvgViewData<'a> {
  pub file_name: &'a str,
  pub svg_data: Option<&'a [u8]>,
  pub has_error: bool,
  pub error_msg: Option<&'a str>,
  pub zoom: f32,
  pub generation: u64,
}

/// Owned view data for SVG preview (avoids borrow issues).
pub struct SvgViewDataOwned {
  pub file_name: String,
  pub svg_data: Option<Vec<u8>>,
  pub has_error: bool,
  pub error_msg: Option<String>,
  pub zoom: f32,
  pub generation: u64,
}

impl SvgViewDataOwned {
  /// Creates a borrowed view from owned data.
  pub fn as_ref(&self) -> SvgViewData<'_> {
    SvgViewData {
      file_name: &self.file_name,
      svg_data: self.svg_data.as_deref(),
      has_error: self.has_error,
      error_msg: self.error_msg.as_deref(),
      zoom: self.zoom,
      generation: self.generation,
    }
  }
}

/// Actions returned from renderer.
pub enum SvgAction {
  ZoomIn,
  ZoomOut,
  ZoomReset,
}

/// Renders SVG preview.
pub fn render(
  ui: &mut Ui,
  data: &SvgViewData<'_>,
  cache: &mut Option<SvgTextureCache>,
) {
  ui.vertical(|ui| {
    if data.has_error {
      ui.centered_and_justified(|ui| {
        ui.label(
          RichText::new(data.error_msg.unwrap_or("Failed to load SVG"))
            .size(14.0)
            .color(Color32::from_rgb(255, 100, 100)),
        );
      });
      return;
    }

    // Render SVG to texture if needed (account for HiDPI/retina)
    let pixels_per_point = ui.ctx().pixels_per_point();
    let needs_render = cache
      .as_ref()
      .map(|c| c.generation != data.generation)
      .unwrap_or(true);

    if needs_render
      && let Some(svg_bytes) = data.svg_data
      && let Some(new_cache) = render_svg_to_texture(
        ui.ctx(),
        svg_bytes,
        data.zoom,
        pixels_per_point,
        data.generation,
      )
    {
      *cache = Some(new_cache);
    }

    // Get viewport size before entering ScrollArea
    let viewport_size = ui.available_size();

    // Display cached texture centered in available space
    egui::ScrollArea::both().show(ui, |ui| {
      if let Some(cache) = cache {
        // Use logical size for layout (texture is rendered at physical
        // resolution)
        let img_size = egui::vec2(cache.logical_size.0, cache.logical_size.1);

        // Calculate padding to center the image in the viewport
        let pad_x = ((viewport_size.x - img_size.x) / 2.0).max(0.0);
        let pad_y = ((viewport_size.y - img_size.y) / 2.0).max(0.0);

        ui.vertical(|ui| {
          ui.add_space(pad_y);
          ui.horizontal(|ui| {
            ui.add_space(pad_x);
            ui.image(egui::load::SizedTexture::new(
              cache.texture.id(),
              img_size,
            ));
          });
        });
      } else {
        ui.centered_and_justified(|ui| {
          ui.spinner();
        });
      }
    });
  });
}

/// Renders SVG bytes to an egui texture at native resolution.
fn render_svg_to_texture(
  ctx: &egui::Context,
  svg_bytes: &[u8],
  zoom: f32,
  pixels_per_point: f32,
  generation: u64,
) -> Option<SvgTextureCache> {
  // Parse SVG
  let tree =
    usvg::Tree::from_data(svg_bytes, &usvg::Options::default()).ok()?;
  let svg_size = tree.size();

  // Calculate logical size (what user sees)
  let logical_width = svg_size.width() * zoom;
  let logical_height = svg_size.height() * zoom;

  // Calculate physical pixel dimensions (for crisp rendering on HiDPI)
  let physical_scale = zoom * pixels_per_point;
  let physical_width = (svg_size.width() * physical_scale).ceil() as u32;
  let physical_height = (svg_size.height() * physical_scale).ceil() as u32;

  // Avoid zero-size textures
  if physical_width == 0 || physical_height == 0 {
    return None;
  }

  // Render to pixmap at physical resolution
  let mut pixmap = Pixmap::new(physical_width, physical_height)?;
  let transform =
    resvg::tiny_skia::Transform::from_scale(physical_scale, physical_scale);
  resvg::render(&tree, transform, &mut pixmap.as_mut());

  // Convert to egui ColorImage (RGBA)
  let pixels: Vec<egui::Color32> = pixmap
    .pixels()
    .iter()
    .map(|p| {
      egui::Color32::from_rgba_premultiplied(
        p.red(),
        p.green(),
        p.blue(),
        p.alpha(),
      )
    })
    .collect();

  let size = [physical_width as usize, physical_height as usize];
  let image = egui::ColorImage {
    size,
    pixels,
    source_size: egui::Vec2::new(physical_width as f32, physical_height as f32),
  };

  // Create texture
  let texture = ctx.load_texture(
    format!("svg_preview_{generation}"),
    image,
    egui::TextureOptions::LINEAR,
  );

  Some(SvgTextureCache {
    texture,
    generation,
    logical_size: (logical_width, logical_height),
  })
}
