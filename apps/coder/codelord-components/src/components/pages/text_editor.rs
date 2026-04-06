pub(crate) mod view;

use crate::components::panels::{panel_bottom, panel_left, panel_right};

use codelord_core::ecs::world::World;

use codelord_core::panel::resources::{
  BottomPanelResource, LeftPanelResource, RightPanelResource,
};

use codelord_core::tabbar::{ZoomSource, ZoomState};

use eframe::egui;

/// egui's GUI rounding constant (1/32) for pixel-aligned coordinates.
const GUI_ROUNDING: f32 = 1.0 / 32.0;

/// Round a value to egui's GUI_ROUNDING to prevent unaligned warnings.
#[inline(always)]
fn round_ui(value: f32) -> f32 {
  (value / GUI_ROUNDING).round() * GUI_ROUNDING
}

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  // Read zoom state (pure data, no method calls)
  let (is_zoomed, is_animating, zoom_source, zoom_progress, zooming_in) = world
    .get_resource::<ZoomState>()
    .map(|z| {
      let progress = z
        .transition
        .as_ref()
        .map(|t| t.eased_progress)
        .unwrap_or(if z.is_zoomed { 1.0 } else { 0.0 });

      let target_zoomed = z
        .transition
        .as_ref()
        .map(|t| t.target_zoomed)
        .unwrap_or(z.is_zoomed);

      (
        z.is_zoomed,
        z.transition.is_some(),
        z.source,
        progress,
        target_zoomed,
      )
    })
    .unwrap_or((false, false, ZoomSource::Editor, 0.0, false));

  let available_height = ui.available_height();

  // Terminal zoom: animate bottom panel to take full height
  if zoom_source == ZoomSource::Terminal && (is_zoomed || is_animating) {
    // Calculate animated height for terminal
    let normal_height = 250.0_f32.min(available_height * 0.5);

    let terminal_height = if is_animating {
      if zooming_in {
        // Zooming in: grow from normal to full (progress 0→1)
        normal_height + (available_height - normal_height) * zoom_progress
      } else {
        // Zooming out: shrink from full to normal (progress 0→1)
        available_height - (available_height - normal_height) * zoom_progress
      }
    } else if is_zoomed {
      available_height
    } else {
      normal_height
    };

    // Show terminal at calculated height
    egui::TopBottomPanel::bottom("bottom_panel_zoomed")
      .frame(egui::Frame::NONE.fill(ui.ctx().style().visuals.window_fill))
      .exact_height(round_ui(terminal_height))
      .show_inside(ui, |ui| panel_bottom::show(ui, world));

    // Show editor in remaining space during animation or when not fully zoomed
    if is_animating || !is_zoomed {
      egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(ui.ctx().style().visuals.window_fill))
        .show_inside(ui, |ui| view::show(ui, world));
    }

    return;
  }

  // Editor zoom: show only editor (panels already hidden by system)
  if zoom_source == ZoomSource::Editor && (is_zoomed || is_animating) {
    egui::CentralPanel::default()
      .frame(egui::Frame::NONE.fill(ui.ctx().style().visuals.window_fill))
      .show_inside(ui, |ui| view::show(ui, world));

    return;
  }

  let bottom_visible = world
    .get_resource::<BottomPanelResource>()
    .map(|r| r.is_visible)
    .unwrap_or(false);

  let left_visible = world
    .get_resource::<LeftPanelResource>()
    .map(|r| r.is_visible)
    .unwrap_or(false);

  let right_visible = world
    .get_resource::<RightPanelResource>()
    .map(|r| r.is_visible)
    .unwrap_or(false);

  egui::SidePanel::left("left_panel")
    .frame(egui::Frame::NONE.fill(ui.ctx().style().visuals.window_fill))
    .min_width(round_ui(250.0))
    .show_animated_inside(ui, left_visible, |ui| panel_left::show(ui, world));

  egui::SidePanel::right("right_panel")
    .frame(egui::Frame::NONE.fill(ui.ctx().style().visuals.window_fill))
    .min_width(round_ui(400.0))
    .show_animated_inside(ui, right_visible, |ui| panel_right::show(ui, world));

  egui::TopBottomPanel::bottom("bottom_panel")
    .frame(egui::Frame::NONE.fill(ui.ctx().style().visuals.window_fill))
    .min_height(round_ui(250.0))
    .show_animated_inside(ui, bottom_visible, |ui| {
      panel_bottom::show(ui, world)
    });

  egui::CentralPanel::default()
    .frame(egui::Frame::NONE.fill(ui.ctx().style().visuals.window_fill))
    .show_inside(ui, |ui| view::show(ui, world));
}
