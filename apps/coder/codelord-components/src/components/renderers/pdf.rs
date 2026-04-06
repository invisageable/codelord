//! Pure PDF renderer component for egui.
//!
//! This component is a pure function: receives data, returns events.
//! No world access - orchestration happens in coder.rs.
//!
//! ## Usage
//!
//! ```ignore
//! let data = PdfViewData { ... };
//! let output = pdf::render(ui, &data);
//! // Process output.nav_action, output.drag_event, etc.
//! ```

use codelord_core::previews::pdf::{
  PageLayout, PdfDragEvent, PdfNavAction, PdfRenderOutput, PdfViewData,
  PdfZoomAction, RenderedPage, char_range_for_page,
};

use eframe::egui;
use rustc_hash::FxHashMap as HashMap;

// =============================================================================
// Constants
// =============================================================================

/// US Letter width at 72 DPI (points).
const PAGE_WIDTH_PT: f32 = 612.0;
/// US Letter height at 72 DPI (points).
const PAGE_HEIGHT_PT: f32 = 792.0;
/// US Letter aspect ratio.
const PAGE_ASPECT: f32 = PAGE_HEIGHT_PT / PAGE_WIDTH_PT;
/// Spacing between pages in pixels.
const PAGE_SPACING: f32 = 16.0;
/// Selection highlight color (cornflower blue, semi-transparent).
const SELECTION_COLOR: egui::Color32 =
  egui::Color32::from_rgba_premultiplied(100, 149, 237, 100);
/// Placeholder background color.
const PLACEHOLDER_BG: egui::Color32 = egui::Color32::from_gray(40);
/// Placeholder text color.
const PLACEHOLDER_TEXT: egui::Color32 = egui::Color32::from_gray(100);
/// UV rect for full texture.
const UV_FULL: egui::Rect =
  egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

// =============================================================================
// Texture Context (UI-specific, stored in egui memory)
// =============================================================================

/// Cached egui texture for a rendered page.
#[derive(Clone)]
struct CachedTexture {
  texture: egui::TextureHandle,
  width: u32,
  height: u32,
}

/// PDF texture context stored in egui memory.
/// This is UI-specific state that persists across frames.
#[derive(Clone, Default)]
pub struct PdfTextureContext {
  texture_cache: HashMap<usize, CachedTexture>,
  last_generation: u64,
  scroll_to_page: Option<usize>,
}

impl PdfTextureContext {
  fn clear_cache(&mut self) {
    self.texture_cache.clear();
  }

  fn get_or_create_texture(
    &mut self,
    ctx: &egui::Context,
    page: usize,
    rendered: &RenderedPage,
  ) -> (egui::TextureId, u32, u32) {
    if let Some(cached) = self.texture_cache.get(&page) {
      return (cached.texture.id(), cached.width, cached.height);
    }

    let image = egui::ColorImage::from_rgba_unmultiplied(
      [rendered.width as usize, rendered.height as usize],
      &rendered.pixels,
    );

    let texture = ctx.load_texture(
      format!("pdf_page_{page}"),
      image,
      egui::TextureOptions::LINEAR,
    );

    let (id, w, h) = (texture.id(), rendered.width, rendered.height);
    self.texture_cache.insert(
      page,
      CachedTexture {
        texture,
        width: w,
        height: h,
      },
    );
    (id, w, h)
  }
}

// =============================================================================
// Pure Render Function
// =============================================================================

/// Render PDF preview. Pure function: data in, events out.
pub fn render(ui: &mut egui::Ui, data: &PdfViewData<'_>) -> PdfRenderOutput {
  let mut output = PdfRenderOutput::default();

  // Get texture context from egui memory
  let ctx_id = egui::Id::new("pdf_texture_context");
  let mut tex_ctx = ui.ctx().data_mut(|d| {
    d.get_temp_mut_or_default::<PdfTextureContext>(ctx_id)
      .clone()
  });

  // Handle generation change (file switch or zoom)
  if tex_ctx.last_generation != data.generation {
    tex_ctx.clear_cache();
    tex_ctx.last_generation = data.generation;
  }

  // Error state
  if let Some(err) = data.error {
    ui.centered_and_justified(|ui| {
      ui.label(egui::RichText::new(err).color(egui::Color32::RED));
    });
    store_context(ui, ctx_id, tex_ctx);
    return output;
  }

  // Loading / empty states
  if data.page_count == 0 {
    if !data.is_loading {
      ui.centered_and_justified(|ui| ui.label("No pages in PDF"));
    }
    store_context(ui, ctx_id, tex_ctx);
    return output;
  }

  let egui_ctx = ui.ctx().clone();
  let available_width = ui.available_width();
  let logical_scale = (available_width / PAGE_WIDTH_PT) * data.zoom;

  // Navigation bar
  output.nav_action = render_nav_bar(ui, data);
  output.zoom_action = render_zoom_bar(ui, data);
  ui.add_space(8.0);

  // Check for pending scroll
  let scroll_to_page = tex_ctx.scroll_to_page.take().or_else(|| {
    output.nav_action.and_then(|a| match a {
      PdfNavAction::GoToPage(p) => Some(p),
      PdfNavAction::PrevPage if data.current_page > 0 => {
        Some(data.current_page - 1)
      }
      PdfNavAction::NextPage if data.current_page + 1 < data.page_count => {
        Some(data.current_page + 1)
      }
      _ => None,
    })
  });

  // Setup scroll area
  let mut scroll_area = egui::ScrollArea::vertical()
    .id_salt("pdf_scroll")
    .auto_shrink([false, false])
    .scroll_source(egui::scroll_area::ScrollSource {
      scroll_bar: true,
      drag: false,
      mouse_wheel: true,
    });

  if let Some(target) = scroll_to_page {
    let page_height = available_width * data.zoom * PAGE_ASPECT + PAGE_SPACING;
    scroll_area =
      scroll_area.vertical_scroll_offset(target as f32 * page_height);
  }

  // Page layouts for hit testing (built during render)
  let mut page_layouts: HashMap<usize, PageLayout> = HashMap::default();

  // Drag state collected during render
  let mut drag_event: Option<PdfDragEvent> = None;

  // Render pages
  scroll_area.show(ui, |ui| {
    ui.vertical(|ui| {
      for page_idx in 0..data.page_count {
        if page_idx > 0 {
          ui.add_space(PAGE_SPACING);
        }

        // Try to get rendered page from cache
        let rendered = data.page_cache.get(&data.file.to_path_buf(), page_idx);

        if let Some(rendered) = rendered {
          let (tex_id, w, h) =
            tex_ctx.get_or_create_texture(&egui_ctx, page_idx, rendered);

          // Calculate display size
          let aspect = h as f32 / w as f32;
          let display_w = available_width * data.zoom;
          let display_h = display_w * aspect;
          let size = egui::vec2(display_w, display_h);

          // Allocate and draw
          let (rect, response) =
            ui.allocate_exact_size(size, egui::Sense::drag());
          ui.painter()
            .image(tex_id, rect, UV_FULL, egui::Color32::WHITE);

          // Store layout for hit testing
          let scale = display_w / PAGE_WIDTH_PT;
          page_layouts.insert(
            page_idx,
            PageLayout {
              min_x: rect.min.x,
              min_y: rect.min.y,
              scale,
            },
          );

          // Draw selection highlight
          draw_selection_highlight(ui, data, page_idx, rect, scale);

          // Track drag events
          if response.drag_started()
            && let Some(pos) = response.interact_pointer_pos()
          {
            drag_event = Some(PdfDragEvent::Started {
              page: page_idx,
              x: pos.x,
              y: pos.y,
            });
          } else if response.dragged()
            && let Some(pos) = response.interact_pointer_pos()
          {
            drag_event = Some(PdfDragEvent::Moved {
              page: page_idx,
              x: pos.x,
              y: pos.y,
            });
          }

          if response.drag_stopped() {
            drag_event = Some(PdfDragEvent::Released);
          }

          output.any_content = true;
        } else {
          // Page not yet rendered - show placeholder
          output.all_pages_ready = false;
          draw_placeholder(ui, logical_scale, page_idx + 1);
        }
      }
    });
  });

  // Set drag event in output
  output.drag_event = drag_event;

  // If we rendered all pages, mark as ready
  if output.any_content && page_layouts.len() == data.page_count {
    output.all_pages_ready = true;
  }

  // Store context back
  store_context(ui, ctx_id, tex_ctx);

  output
}

// =============================================================================
// Helper Render Functions
// =============================================================================

fn store_context(ui: &egui::Ui, id: egui::Id, ctx: PdfTextureContext) {
  ui.ctx().data_mut(|d| d.insert_temp(id, ctx));
}

fn render_nav_bar(
  ui: &mut egui::Ui,
  data: &PdfViewData<'_>,
) -> Option<PdfNavAction> {
  let mut action = None;

  ui.horizontal(|ui| {
    ui.spacing_mut().item_spacing.x = 8.0;

    // Previous page
    if ui
      .add_enabled(data.current_page > 0, egui::Button::new("<"))
      .clicked()
    {
      action = Some(PdfNavAction::PrevPage);
    }

    ui.label(format!("{} / {}", data.current_page + 1, data.page_count));

    // Next page
    if ui
      .add_enabled(
        data.current_page + 1 < data.page_count,
        egui::Button::new(">"),
      )
      .clicked()
    {
      action = Some(PdfNavAction::NextPage);
    }
  });

  action
}

fn render_zoom_bar(
  ui: &mut egui::Ui,
  data: &PdfViewData<'_>,
) -> Option<PdfZoomAction> {
  let mut action = None;

  ui.horizontal(|ui| {
    ui.separator();

    if ui.button("-").clicked() {
      action = Some(PdfZoomAction::ZoomOut);
    }

    ui.label(format!("{}%", (data.zoom * 100.0) as u32));

    if ui.button("+").clicked() {
      action = Some(PdfZoomAction::ZoomIn);
    }

    if ui.button("Fit").clicked() {
      action = Some(PdfZoomAction::ZoomReset);
    }
  });

  action
}

fn draw_placeholder(ui: &mut egui::Ui, logical_scale: f32, page_num: usize) {
  let width = PAGE_WIDTH_PT * logical_scale;
  let height = PAGE_HEIGHT_PT * logical_scale;
  let (rect, _) =
    ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

  ui.painter().rect_filled(rect, 0.0, PLACEHOLDER_BG);
  ui.painter().text(
    rect.center(),
    egui::Align2::CENTER_CENTER,
    format!("Page {page_num}"),
    egui::FontId::proportional(14.0),
    PLACEHOLDER_TEXT,
  );
}

fn draw_selection_highlight(
  ui: &mut egui::Ui,
  data: &PdfViewData<'_>,
  page_idx: usize,
  rect: egui::Rect,
  scale: f32,
) {
  let Some((start, end)) = data.selection else {
    return;
  };

  let Some(page_text) = data.text_cache.get(&data.file.to_path_buf(), page_idx)
  else {
    return;
  };

  let Some((char_start, char_end)) =
    char_range_for_page(page_idx, start, end, page_text.chars.len())
  else {
    return;
  };

  // Draw highlight for selected chars
  for ch in page_text
    .chars
    .iter()
    .skip(char_start)
    .take(char_end - char_start + 1)
  {
    let char_rect = egui::Rect::from_min_size(
      egui::pos2(rect.min.x + ch.x * scale, rect.min.y + ch.y * scale),
      egui::vec2(ch.width * scale, ch.height * scale),
    );
    ui.painter().rect_filled(char_rect, 0.0, SELECTION_COLOR);
  }
}

// =============================================================================
// Keyboard Input (Returns actions, no world access)
// =============================================================================

/// Input events from keyboard.
#[derive(Default)]
pub struct PdfInputResult {
  pub nav_action: Option<PdfNavAction>,
  pub zoom_action: Option<PdfZoomAction>,
  pub copy_requested: bool,
  pub clear_selection: bool,
}

/// Handle keyboard input for PDF. Returns actions to process.
pub fn handle_input(ui: &egui::Ui) -> PdfInputResult {
  let mut result = PdfInputResult::default();

  ui.input(|i| {
    // Copy
    if i.modifiers.command && i.key_pressed(egui::Key::C) {
      result.copy_requested = true;
    }

    // Clear selection
    if i.key_pressed(egui::Key::Escape) {
      result.clear_selection = true;
    }

    // Navigation
    if i.key_pressed(egui::Key::PageUp) {
      result.nav_action = Some(PdfNavAction::PrevPage);
    }
    if i.key_pressed(egui::Key::PageDown) {
      result.nav_action = Some(PdfNavAction::NextPage);
    }
    if i.key_pressed(egui::Key::Home) {
      result.nav_action = Some(PdfNavAction::GoToPage(0));
    }
    // Note: End key needs page_count, handled in orchestration

    // Zoom
    if i.modifiers.command {
      if i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus) {
        result.zoom_action = Some(PdfZoomAction::ZoomIn);
      }
      if i.key_pressed(egui::Key::Minus) {
        result.zoom_action = Some(PdfZoomAction::ZoomOut);
      }
      if i.key_pressed(egui::Key::Num0) {
        result.zoom_action = Some(PdfZoomAction::ZoomReset);
      }
    }
  });

  result
}
