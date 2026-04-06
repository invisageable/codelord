use codelord_core::drag_and_drop::{DragAxis, DragState};
use codelord_core::ecs::world::World;

use eframe::egui;

/// Result of a drag source interaction.
pub struct DragResult {
  pub response: egui::Response,
  pub is_dragging: bool,
}

/// Render a drag source.
pub fn source<T, R>(
  ui: &mut egui::Ui,
  world: &mut World,
  id: egui::Id,
  payload: T,
  axis: DragAxis,
  source_name: &str,
  content: impl FnOnce(&mut egui::Ui) -> R,
) -> (R, DragResult)
where
  T: std::any::Any + Send + Sync + Clone,
{
  let is_being_dragged = world
    .get_resource::<DragState>()
    .map(|s| s.has_payload::<T>())
    .unwrap_or(false);

  let inner = ui.scope(|ui| content(ui)).inner;
  let response = ui.interact(ui.min_rect(), id, egui::Sense::click_and_drag());

  // Start drag
  if response.drag_started()
    && let Some(pos) = ui.ctx().pointer_hover_pos()
    && let Some(mut state) = world.get_resource_mut::<DragState>()
  {
    state.set_payload(payload, axis, source_name, [pos.x, pos.y]);
  }

  // Update position while dragging
  if response.dragged() {
    if let Some(pos) = ui.ctx().pointer_hover_pos()
      && let Some(mut state) = world.get_resource_mut::<DragState>()
    {
      state.update_pos([pos.x, pos.y]);
    }
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
  }

  // Clear on release or escape
  if (response.drag_stopped() || ui.input(|i| i.key_pressed(egui::Key::Escape)))
    && let Some(mut state) = world.get_resource_mut::<DragState>()
  {
    state.clear();
  }

  (
    inner,
    DragResult {
      response,
      is_dragging: is_being_dragged,
    },
  )
}

/// Check drop zone and return payload if dropped.
pub fn drop_zone<T>(
  ui: &egui::Ui,
  world: &World,
  rect: egui::Rect,
) -> Option<bool>
where
  T: std::any::Any + Send + Sync,
{
  let has_payload = world
    .get_resource::<DragState>()
    .map(|s| s.has_payload::<T>())
    .unwrap_or(false);

  if !has_payload {
    return None;
  }

  let hovering = ui
    .ctx()
    .pointer_hover_pos()
    .map(|pos| rect.contains(pos))
    .unwrap_or(false);

  Some(hovering)
}

/// Take the payload if mouse released over drop zone.
pub fn take_drop<T>(world: &mut World) -> Option<T>
where
  T: std::any::Any + Send + Sync + Clone,
{
  world
    .get_resource_mut::<DragState>()
    .and_then(|mut s| s.take_payload::<T>())
}

/// Get current drag state info.
pub fn drag_info(world: &World) -> Option<(DragAxis, [f32; 2], [f32; 2])> {
  world.get_resource::<DragState>().and_then(|s| {
    if s.is_dragging() {
      Some((s.axis, s.start_pos?, s.current_pos?))
    } else {
      None
    }
  })
}
