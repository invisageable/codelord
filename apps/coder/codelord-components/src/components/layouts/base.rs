use crate::components::pages::{
  about, notes, playground, presenter, settings, text_editor, welcome,
};

use codelord_core::ecs::world::World;
use codelord_core::page::components::{Page, SlideDirection};
use codelord_core::page::resources::PageResource;

use eframe::egui;

/// egui's GUI rounding constant (1/32) for pixel-aligned coordinates.
const GUI_ROUNDING: f32 = 1.0 / 32.0;

/// Round a value to egui's GUI_ROUNDING to prevent unaligned warnings.
#[inline(always)]
fn round_ui(value: f32) -> f32 {
  (value / GUI_ROUNDING).round() * GUI_ROUNDING
}

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  // Extract the page state we need before borrowing world mutably
  let (active_page, transition_data) =
    if let Some(page_res) = world.get_resource::<PageResource>() {
      let active = page_res.active_page;
      let trans = page_res
        .transition
        .as_ref()
        .map(|t| (t.from_page, t.to_page, t.eased_progress(), t.direction));
      (active, trans)
    } else {
      (Page::Welcome, None)
    };

  // Now we can render with mutable access to world
  if let Some((from_page, to_page, progress, direction)) = transition_data {
    // Render transition with slide animation
    let available_rect = ui.available_rect_before_wrap();
    let width = available_rect.width();

    // Set clip rect to prevent overflow
    ui.set_clip_rect(available_rect);

    // Calculate the positions for the FROM and TO pages based on progress
    // Use round_ui on offsets to prevent unaligned warnings during animation
    let (from_rect, to_rect) = match direction {
      SlideDirection::Left => {
        let offset = round_ui(-progress * width);
        let from_rect = available_rect.translate(egui::vec2(offset, 0.0));
        let to_rect = from_rect.translate(egui::vec2(width, 0.0));
        (from_rect, to_rect)
      }
      SlideDirection::Right => {
        let offset = round_ui(progress * width);
        let from_rect = available_rect.translate(egui::vec2(offset, 0.0));
        let to_rect = from_rect.translate(egui::vec2(-width, 0.0));
        (from_rect, to_rect)
      }
    };

    // Each page renders on its own layer during transition.
    // The warn_if_rect_changes_id check is per-layer, so
    // widgets from different pages never collide.

    if from_rect.intersects(available_rect) {
      ui.scope_builder(
        egui::UiBuilder::new()
          .layer_id(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("page_transition_from"),
          ))
          .max_rect(from_rect),
        |ui| {
          ui.set_clip_rect(available_rect);
          render_page(ui, &from_page, world);
        },
      );
    }

    if to_rect.intersects(available_rect) {
      ui.scope_builder(
        egui::UiBuilder::new()
          .layer_id(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("page_transition_to"),
          ))
          .max_rect(to_rect),
        |ui| {
          ui.set_clip_rect(available_rect);
          render_page(ui, &to_page, world);
        },
      );
    }
  } else {
    // No transition - render active page normally
    render_page(ui, &active_page, world);
  }
}

/// Render a specific page
fn render_page(ui: &mut egui::Ui, page: &Page, world: &mut World) {
  match page {
    Page::Welcome => welcome::show(ui, world),
    Page::Editor => text_editor::show(ui, world),
    Page::Playground => playground::show(ui, world),
    Page::Notes => notes::show(ui, world),
    Page::Presenter => presenter::show(ui, world),
    Page::Settings => settings::show(ui, world),
    Page::About => about::show(ui, world),
  }
}
