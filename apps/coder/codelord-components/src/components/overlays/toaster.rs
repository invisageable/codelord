//! Toaster overlay - renders all active toast notifications.

use super::toast;

use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::ecs::world::World;
use codelord_core::toast::components::{Toast, ToastId};
use codelord_core::toast::resources::{ToastActionEvent, ToasterResource};

use eframe::egui;

/// Result of rendering toasts.
pub struct ToasterResult {
  pub dismissed_ids: Vec<ToastId>,
  pub action_events: Vec<ToastActionEvent>,
}

/// Renders all active toasts (top-right corner, stacked vertically).
/// Returns dismissed IDs and action events.
pub fn show(ui: &mut egui::Ui, world: &mut World) -> ToasterResult {
  // Extract data from toaster resource first
  let (toasts, toast_width, toast_height): (Vec<Toast>, f32, f32) = {
    let Some(toaster) = world.get_resource::<ToasterResource>() else {
      return ToasterResult {
        dismissed_ids: Vec::new(),
        action_events: Vec::new(),
      };
    };

    if toaster.is_empty() {
      return ToasterResult {
        dismissed_ids: Vec::new(),
        action_events: Vec::new(),
      };
    }

    (
      toaster.iter().cloned().collect(),
      toaster.toast_width,
      toaster.toast_height,
    )
  };

  // Signal that toast animations are active (now safe - toaster borrow dropped)
  if let Some(mut cont) = world.get_resource_mut::<ContinuousAnimations>() {
    cont.set_toast_active();
  }

  let content_rect = ui.ctx().content_rect();
  let start_x = content_rect.right() - toast_width - 8.0;
  let mut dismissed_ids = Vec::new();
  let mut action_events = Vec::new();

  for toast in &toasts {
    let y_position = toast.animation.y_position;
    let x_position = start_x + toast.animation.x_offset;

    let interaction = toast::show(
      ui,
      world,
      toast,
      egui::pos2(x_position, y_position),
      toast_width,
      toast_height,
    );

    if interaction.dismiss {
      dismissed_ids.push(toast.id);
    }

    if let Some(action_id) = interaction.action_clicked {
      action_events.push(ToastActionEvent {
        toast_id: toast.id,
        action_id,
      });
    }
  }

  ToasterResult {
    dismissed_ids,
    action_events,
  }
}
