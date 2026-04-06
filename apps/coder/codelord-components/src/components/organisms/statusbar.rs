//! Statusbar organism - bottom bar with status info
//!
//! Pure UI component that reads from ECS World

use crate::assets::icon;
#[cfg(debug_assertions)]
use crate::components::indicators::frame_history;
use crate::components::structure::divider;
use crate::components::structure::divider::Axis;

use codelord_core::ecs::prelude::With;
use codelord_core::ecs::world::World;
use codelord_core::icon::components::Icon;
use codelord_core::panel::resources::{
  BottomPanelResource, LeftPanelResource, PanelAction, PanelCommand,
};
use codelord_core::statusbar::resources::{
  LineColumnAnimation, StatusbarResource,
};
use codelord_core::tabbar::components::EditorTab;
use codelord_core::text_editor::components::FileTab;
use codelord_core::ui::component::Active;
use codelord_core::voice::resources::{VoiceResource, VoiceToggleCommand};

use eframe::egui;

/// Render the statusbar
///
/// Shows cursor position, file info, theme mode, etc.
/// Reads state from ECS World but doesn't mutate it.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let statusbar = match world.get_resource::<StatusbarResource>() {
    Some(s) => s,
    None => return,
  };

  let left_entities = statusbar.left.clone();
  let right_entities = statusbar.right.clone();

  // Read panel visibility for active state coloring
  let left_panel_visible = world
    .get_resource::<LeftPanelResource>()
    .map(|r| r.is_visible)
    .unwrap_or(false);

  let bottom_panel_visible = world
    .get_resource::<BottomPanelResource>()
    .map(|r| r.is_visible)
    .unwrap_or(false);

  let voice_active = world
    .get_resource::<VoiceResource>()
    .map(|r| r.state.is_active())
    .unwrap_or(false);

  let mut toggle_left = false;
  let mut toggle_bottom = false;
  let mut toggle_voice = false;

  ui.horizontal_centered(|ui| {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
      for entity in &left_entities {
        let icon = world.get::<Icon>(*entity).copied();

        let is_active =
          matches!(icon, Some(Icon::Explorer)) && left_panel_visible;

        if show_icon_button(ui, world, *entity, is_active)
          && matches!(icon, Some(Icon::Explorer))
        {
          toggle_left = true;
        }

        divider::show(ui, Axis::Vertical);
      }
    });

    #[cfg(debug_assertions)]
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
      frame_history::show(ui)
    });

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      for entity in &right_entities {
        let icon = world.get::<Icon>(*entity).copied();
        let is_active = matches!(icon, Some(Icon::Voice)) && voice_active;

        if show_icon_button(ui, world, *entity, is_active)
          && matches!(icon, Some(Icon::Voice))
        {
          toggle_voice = true;
        }

        divider::show(ui, Axis::Vertical);
      }

      if show_terminal_button(ui, bottom_panel_visible) {
        toggle_bottom = true;
      }

      divider::show(ui, Axis::Vertical);

      let has_active_file = world
        .query_filtered::<&FileTab, (With<EditorTab>, With<Active>)>()
        .iter(world)
        .next()
        .is_some();

      if has_active_file {
        show_syntax_label(ui, world);
        show_numlines_label(ui, world);
      }
    });
  });

  if toggle_left {
    world.write_message(PanelCommand {
      action: PanelAction::ToggleLeft,
    });
  }

  if toggle_bottom {
    world.write_message(PanelCommand {
      action: PanelAction::ToggleBottom,
    });
  }

  if toggle_voice {
    world.write_message(VoiceToggleCommand);
  }
}

/// Render terminal button. Returns true if clicked.
fn show_terminal_button(ui: &mut egui::Ui, is_active: bool) -> bool {
  let tint = if is_active {
    ui.style().visuals.widgets.hovered.fg_stroke.color
  } else {
    ui.style().visuals.text_color()
  };

  let image = icon::icon_to_image(&Icon::Terminal)
    .fit_to_exact_size(egui::Vec2::splat(16.0))
    .tint(tint);

  let button = egui::Button::image(image)
    .fill(egui::Color32::TRANSPARENT)
    .frame(false);

  let response = ui.add(button);

  if response.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  response.clicked()
}

/// Render syntax label showing file extension with dot prefix.
fn show_syntax_label(ui: &mut egui::Ui, world: &mut World) {
  // Get file extension from active editor tab
  let extension = world
    .query_filtered::<&FileTab, (With<EditorTab>, With<Active>)>()
    .iter(world)
    .next()
    .and_then(|ft| ft.path.extension())
    .map(|ext| format!(".{}", ext.to_string_lossy()))
    .unwrap_or_else(|| ".txt".to_string());

  let text_color = ui.style().visuals.text_color();

  let button = egui::Button::new(
    egui::RichText::new(extension)
      .color(text_color)
      .font(egui::FontId::new(10.0, egui::FontFamily::Monospace)),
  )
  .frame(false);

  let response = ui.add(button);

  if response.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }
}

/// Render numlines label showing (line, col).
///
/// Reads animated values from LineColumnAnimation resource.
/// Animation logic is handled by the system, render only displays.
fn show_numlines_label(ui: &mut egui::Ui, world: &World) {
  // Read animated values from resource (updated by system)
  let (line, col) = world
    .get_resource::<LineColumnAnimation>()
    .map(|anim| (anim.line, anim.column))
    .unwrap_or((1, 1));

  let text_color = ui.style().visuals.text_color();
  let numlines_fmt = format!("({line},{col})");

  let button = egui::Button::new(
    egui::RichText::new(numlines_fmt)
      .color(text_color)
      .font(egui::FontId::new(10.0, egui::FontFamily::Monospace)),
  )
  .frame(false);

  let response = ui.add(button);

  if response.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }
}

/// Render an icon button for a statusbar entity.
fn show_icon_button(
  ui: &mut egui::Ui,
  world: &mut World,
  entity: codelord_core::ecs::entity::Entity,
  is_active: bool,
) -> bool {
  let icon = match world.get::<Icon>(entity) {
    Some(i) => *i,
    None => return false,
  };

  let tint = if is_active {
    ui.style().visuals.widgets.hovered.fg_stroke.color
  } else {
    ui.style().visuals.text_color()
  };

  let image = icon::icon_to_image(&icon)
    .fit_to_exact_size(egui::Vec2::splat(16.0))
    .tint(tint);

  let button = egui::Button::image(image)
    .fill(egui::Color32::TRANSPARENT)
    .frame(false);

  let response = ui.add(button);

  if response.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  response.clicked()
}
