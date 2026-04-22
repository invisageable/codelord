use crate::components::navigation::{breadcrumbs, tabbar};
use crate::components::renderers::pdf;
use crate::components::renderers::{
  csv_table, font, markdown, sqlite, svg, xls,
};
use crate::components::views::{color_picker, editor_content};

use codelord_core::animation::resources::ActiveAnimations;
use codelord_core::ecs::world::World;
use codelord_core::loading::{GlobalLoading, LoadingTask};
use codelord_core::navigation::resources::BreadcrumbData;
use codelord_core::previews::csv::CsvData;
use codelord_core::previews::xls::{
  ChangeXlsPageRequest, SelectSheetRequest, XlsPreviewState,
};
use codelord_core::previews::{
  CsvPreviewState, FontPreviewState, MarkdownPreviewState, SqlitePreviewState,
  SvgPreviewState,
};
use codelord_core::previews::{
  PageLayout, PdfNavAction, PdfPageCache, PdfPreviewState, PdfSelection,
  PdfTextCache, PdfViewData, PdfZoomAction, extract_selected_text,
  hit_test_char,
};
use codelord_core::tabbar::components::EditorTab;

use eframe::egui;

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let has_active_file = world
    .get_resource::<BreadcrumbData>()
    .map(|b| !b.segments.is_empty())
    .unwrap_or(false);

  // Check if markdown, CSV, PDF, SQLite, XLS, SVG, or font preview is active
  let font_preview = get_font_preview_active(world);
  let svg_preview = get_svg_preview_active(world);
  let markdown_preview = get_markdown_preview_state(world);
  let csv_preview = get_csv_preview_state(world);
  let pdf_preview = get_pdf_preview_active(world);
  let sqlite_preview = get_sqlite_preview_active(world);
  let xls_preview = get_xls_preview_active(world);

  egui::Panel::top("text_editor_tabbar")
    .frame(egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill))
    .exact_size(24.0)
    .resizable(false)
    .show_separator_line(true)
    .show_inside(ui, |ui| tabbar::show::<EditorTab>(ui, world));

  egui::Panel::top("text_editor_breadcrumbs")
    .frame(
      egui::Frame::NONE
        .fill(ui.ctx().global_style().visuals.window_fill)
        .inner_margin(egui::Margin::symmetric(8, 0)),
    )
    .exact_size(20.0)
    .resizable(false)
    .show_separator_line(has_active_file)
    .show_animated_inside(ui, has_active_file, |ui| {
      breadcrumbs::show(ui, world)
    });

  egui::CentralPanel::default()
    .frame(egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill))
    .show_inside(ui, |ui| {
      ui.set_width(ui.available_width());
      ui.set_height(ui.available_height());

      // Show preview if enabled, otherwise show code editor
      if pdf_preview {
        render_pdf_preview(ui, world);
        return;
      }

      if font_preview {
        render_font_preview(ui, world);
      } else if svg_preview {
        render_svg_preview(ui, world);
      } else if sqlite_preview {
        render_sqlite_preview(ui, world);
      } else if xls_preview {
        render_xls_preview(ui, world);
      } else if let Some((content, base_path)) = markdown_preview {
        markdown::render_with_base_path(ui, &content, base_path.as_deref());
      } else if let Some(csv_data) = csv_preview {
        csv_table::render(ui, &csv_data);
      } else {
        editor_content::show::<EditorTab>(ui, world, "text_editor_scroll");

        // Render color picker popup (overlays editor content).
        if let Some(replace_event) = color_picker::show(ui, world) {
          color_picker::apply_replace_event(world, replace_event);
        }
      }
    });
}

/// Gets markdown preview state if active for the current file.
///
/// Returns Some((content, base_path)) if preview is active, None otherwise.
fn get_markdown_preview_state(
  world: &mut World,
) -> Option<(String, Option<std::path::PathBuf>)> {
  let preview_state = world.get_resource::<MarkdownPreviewState>()?;

  if !preview_state.enabled {
    return None;
  }

  let content = preview_state.cached_content.clone()?;
  let base_path = preview_state.current_file.clone();

  Some((content, base_path))
}

/// Gets CSV preview state if active for the current file.
///
/// Returns Some(CsvData) if preview is active, None otherwise.
fn get_csv_preview_state(world: &mut World) -> Option<CsvData> {
  let preview_state = world.get_resource::<CsvPreviewState>()?;

  if !preview_state.enabled {
    return None;
  }

  preview_state.cached_data.clone()
}

/// Checks if PDF preview is active.
fn get_pdf_preview_active(world: &mut World) -> bool {
  world
    .get_resource::<PdfPreviewState>()
    .map(|p| p.enabled)
    .unwrap_or(false)
}

/// Renders PDF preview with proper orchestration.
/// Builds view data from resources, calls pure component, processes output.
fn render_pdf_preview(ui: &mut egui::Ui, world: &mut World) {
  // Handle keyboard input first (returns actions, doesn't mutate world)
  let input = pdf::handle_input(ui);

  // Process keyboard input actions
  if input.copy_requested
    && let Some(selection) = world.get_resource::<PdfSelection>()
    && let Some((start, end)) = selection.get_range()
    && let Some(state) = world.get_resource::<PdfPreviewState>()
    && let Some(file) = &state.current_file
    && let Some(text_cache) = world.get_resource::<PdfTextCache>()
  {
    let text = extract_selected_text(text_cache, file, start, end);
    if !text.is_empty() {
      ui.ctx().copy_text(text);
    }
  }

  if input.clear_selection
    && let Some(mut selection) = world.get_resource_mut::<PdfSelection>()
  {
    selection.clear();
  }

  // Apply keyboard nav/zoom actions to state
  apply_nav_action(world, input.nav_action);
  apply_zoom_action(world, input.zoom_action);

  // Build view data from resources and render
  // Scope the immutable borrow of world
  let output = {
    let Some(view_data) = build_pdf_view_data(world) else {
      return;
    };
    pdf::render(ui, &view_data)
  };

  // Process render output actions (world is no longer borrowed)
  apply_nav_action(world, output.nav_action);
  apply_zoom_action(world, output.zoom_action);

  // Process drag events for text selection
  if let Some(drag_event) = output.drag_event {
    process_drag_event(world, drag_event);
  }

  // Finish loading indicator when content is displayed
  if output.any_content
    && let Some(mut loading) = world.get_resource_mut::<GlobalLoading>()
  {
    // Only decrement active animations once when transitioning from loading
    let was_loading = loading.is_task_active(LoadingTask::PdfRender);
    loading.finish(LoadingTask::PdfRender);

    if was_loading
      && let Some(mut active) = world.get_resource_mut::<ActiveAnimations>()
    {
      active.decrement();
    }
  }
}

/// Builds PdfViewData from world resources.
fn build_pdf_view_data(world: &World) -> Option<PdfViewData<'_>> {
  let state = world.get_resource::<PdfPreviewState>()?;
  let file = state.current_file.as_ref()?;
  let page_cache = world.get_resource::<PdfPageCache>()?;
  let text_cache = world.get_resource::<PdfTextCache>()?;

  let selection = world
    .get_resource::<PdfSelection>()
    .and_then(|s| s.get_range());

  Some(PdfViewData {
    file,
    current_page: state.current_page,
    page_count: state.page_count,
    zoom: state.zoom,
    is_loading: state.is_loading,
    error: state.error.as_deref(),
    generation: state.generation,
    page_cache,
    text_cache,
    selection,
  })
}

/// Applies navigation action to PDF state.
fn apply_nav_action(world: &mut World, action: Option<PdfNavAction>) {
  let Some(action) = action else { return };
  let Some(mut state) = world.get_resource_mut::<PdfPreviewState>() else {
    return;
  };

  match action {
    PdfNavAction::PrevPage => state.prev_page(),
    PdfNavAction::NextPage => state.next_page(),
    PdfNavAction::GoToPage(page) => state.go_to_page(page),
  }
}

/// Applies zoom action to PDF state.
fn apply_zoom_action(world: &mut World, action: Option<PdfZoomAction>) {
  let Some(action) = action else { return };
  let Some(mut state) = world.get_resource_mut::<PdfPreviewState>() else {
    return;
  };

  match action {
    PdfZoomAction::ZoomIn => state.zoom_in(),
    PdfZoomAction::ZoomOut => state.zoom_out(),
    PdfZoomAction::ZoomReset => state.zoom_reset(),
  }
}

/// Processes drag events for text selection.
fn process_drag_event(
  world: &mut World,
  event: codelord_core::previews::PdfDragEvent,
) {
  use codelord_core::previews::PdfDragEvent;

  // Helper to do hit testing
  let do_hit_test =
    |world: &World, page: usize, x: f32, y: f32| -> Option<(usize, usize)> {
      let state = world.get_resource::<PdfPreviewState>()?;
      let text_cache = world.get_resource::<PdfTextCache>()?;
      let file = state.current_file.as_ref()?;
      let page_text = text_cache.get(file, page)?;

      let layout = PageLayout {
        min_x: 0.0,
        min_y: 0.0,
        scale: state.zoom,
      };

      hit_test_char(page, x, y, &layout, page_text)
    };

  match event {
    PdfDragEvent::Started { page, x, y } => {
      let hit = do_hit_test(world, page, x, y);

      if let Some((page_idx, char_idx)) = hit
        && let Some(mut selection) = world.get_resource_mut::<PdfSelection>()
      {
        selection.start_selection(page_idx, char_idx);
      }
    }
    PdfDragEvent::Moved { page, x, y } => {
      let hit = do_hit_test(world, page, x, y);

      if let Some((page_idx, char_idx)) = hit
        && let Some(mut selection) = world.get_resource_mut::<PdfSelection>()
      {
        selection.update_selection(page_idx, char_idx);
      }
    }
    PdfDragEvent::Released => {
      if let Some(mut selection) = world.get_resource_mut::<PdfSelection>() {
        selection.finish_selection();
      }
    }
  }
}

/// Checks if SQLite preview is active.
fn get_sqlite_preview_active(world: &mut World) -> bool {
  world
    .get_resource::<SqlitePreviewState>()
    .map(|p| p.enabled)
    .unwrap_or(false)
}

/// Renders SQLite preview.
/// The renderer spawns ECS request entities for actions (select, page, export).
fn render_sqlite_preview(ui: &mut egui::Ui, world: &mut World) {
  sqlite::render(ui, world);
}

/// Checks if XLS preview is active.
fn get_xls_preview_active(world: &mut World) -> bool {
  world
    .get_resource::<XlsPreviewState>()
    .map(|p| p.enabled)
    .unwrap_or(false)
}

/// Renders XLS preview and handles returned actions.
fn render_xls_preview(ui: &mut egui::Ui, world: &mut World) {
  let Some(state) = world.get_resource::<XlsPreviewState>() else {
    return;
  };

  if !state.enabled {
    return;
  }

  // Render and get any user action
  let action = xls::render(ui, state);

  // Spawn ECS entity for the action
  if let Some(action) = action {
    match action {
      xls::XlsAction::SelectSheet(idx) => {
        world.spawn(SelectSheetRequest(idx));
      }
      xls::XlsAction::ChangePage(page) => {
        world.spawn(ChangeXlsPageRequest(page));
      }
    }
  }
}

/// Checks if font preview is active.
fn get_font_preview_active(world: &mut World) -> bool {
  world
    .get_resource::<FontPreviewState>()
    .map(|p| p.enabled)
    .unwrap_or(false)
}

/// Renders font preview.
fn render_font_preview(ui: &mut egui::Ui, world: &mut World) {
  use std::cell::RefCell;

  // Cache: (generation, family_name, frames_since_register)
  thread_local! {
    static FONT_CACHE: RefCell<(u64, Option<String>, u8)> = const { RefCell::new((0, None, 0)) };
  }

  let Some(state) = world.get_resource::<FontPreviewState>() else {
    return;
  };

  if !state.enabled {
    return;
  }

  // Register font with egui if we have data (done once per font change)
  let (family_name, ready) = if let Some(data) = &state.font_data {
    FONT_CACHE.with(|cache| {
      let mut cache = cache.borrow_mut();
      if cache.0 != state.generation {
        // New font - register it
        let name =
          font::register_preview_font(ui.ctx(), &state.font_name, data);
        cache.0 = state.generation;
        cache.1 = Some(name.clone());
        cache.2 = 0; // Reset frame counter
        (Some(name), false) // Not ready this frame
      } else {
        // Same font - check if ready
        cache.2 = cache.2.saturating_add(1);
        (cache.1.clone(), cache.2 >= 2) // Ready after 2 frames
      }
    })
  } else {
    (None, true)
  };

  // Skip rendering until font is ready
  if !ready {
    ui.ctx().request_repaint();
    ui.vertical_centered(|ui| {
      ui.add_space(80.0);
      ui.spinner();
    });
    return;
  }

  let view_data = font::FontViewData {
    font_name: &state.font_name,
    has_error: state.error.is_some(),
    error_msg: state.error.as_deref(),
    family_name: family_name.as_deref(),
  };

  font::render(ui, &view_data);
}

/// Checks if SVG preview is active.
fn get_svg_preview_active(world: &mut World) -> bool {
  world
    .get_resource::<SvgPreviewState>()
    .map(|p| p.enabled)
    .unwrap_or(false)
}

/// Renders SVG preview.
fn render_svg_preview(ui: &mut egui::Ui, world: &mut World) {
  // Build owned view data from state
  let view_data = {
    let Some(state) = world.get_resource::<SvgPreviewState>() else {
      return;
    };

    if !state.enabled {
      return;
    }

    svg::SvgViewDataOwned {
      file_name: state.file_name.clone(),
      svg_data: state.svg_data.clone(),
      has_error: state.error.is_some(),
      error_msg: state.error.clone(),
      zoom: state.zoom,
      generation: state.generation,
    }
  };

  // Render SVG using non-send resource for texture cache
  let Some(mut cache_res) =
    world.get_non_send_resource_mut::<svg::SvgTextureCacheResource>()
  else {
    return;
  };
  svg::render(ui, &view_data.as_ref(), &mut cache_res.0);
}
