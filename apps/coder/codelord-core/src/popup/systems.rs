use super::components::{Popup, PopupVisible};
use super::resources::{PopupAction, PopupCommand, PopupResource};

use bevy_ecs::entity::Entity;
use bevy_ecs::message::MessageReader;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query, ResMut};

/// System to handle popup commands.
pub fn popup_command_system(
  mut commands: Commands,
  mut popup_commands: MessageReader<PopupCommand>,
  mut popup_resource: ResMut<PopupResource>,
  mut popups: Query<(Entity, &mut Popup)>,
  visible_popups: Query<Entity, With<PopupVisible>>,
) {
  for command in popup_commands.read() {
    match &command.action {
      PopupAction::Show {
        entity,
        anchor_rect,
      } => {
        // Hide all other popups first (auto-close behavior)
        for visible_entity in visible_popups.iter() {
          if visible_entity != *entity {
            commands.entity(visible_entity).remove::<PopupVisible>();
          }
        }

        // Show the requested popup
        if let Ok((_, mut popup)) = popups.get_mut(*entity) {
          popup.anchor_rect = Some(*anchor_rect);
          commands.entity(*entity).insert(PopupVisible);
          popup_resource.active_popup = Some(*entity);
        }
      }

      PopupAction::Hide(entity) => {
        commands.entity(*entity).remove::<PopupVisible>();
        if popup_resource.active_popup == Some(*entity) {
          popup_resource.active_popup = None;
        }
      }

      PopupAction::Toggle {
        entity,
        anchor_rect,
      } => {
        let is_visible = visible_popups.get(*entity).is_ok();

        if is_visible {
          commands.entity(*entity).remove::<PopupVisible>();
          if popup_resource.active_popup == Some(*entity) {
            popup_resource.active_popup = None;
          }
        } else {
          // Hide all other popups first
          for visible_entity in visible_popups.iter() {
            if visible_entity != *entity {
              commands.entity(visible_entity).remove::<PopupVisible>();
            }
          }

          if let Ok((_, mut popup)) = popups.get_mut(*entity) {
            popup.anchor_rect = Some(*anchor_rect);
            commands.entity(*entity).insert(PopupVisible);
            popup_resource.active_popup = Some(*entity);
          }
        }
      }

      PopupAction::HideAll => {
        for visible_entity in visible_popups.iter() {
          commands.entity(visible_entity).remove::<PopupVisible>();
        }
        popup_resource.active_popup = None;
      }
    }
  }
}

/// System to handle click outside popup to close it.
/// This should be called from the render layer with click position.
pub fn handle_click_outside(
  commands: &mut Commands,
  popup_resource: &mut PopupResource,
  visible_popups: &Query<(Entity, &Popup), With<PopupVisible>>,
  click_pos: [f32; 2],
) {
  let [click_x, click_y] = click_pos;

  for (entity, popup) in visible_popups.iter() {
    if popup.auto_close {
      // Check if click is outside the popup area
      // Note: This checks anchor_rect, but actual popup rect may differ
      // The render layer should provide accurate bounds
      let is_inside = popup
        .anchor_rect
        .map(|[x, y, w, h]| {
          click_x >= x && click_x <= x + w && click_y >= y && click_y <= y + h
        })
        .unwrap_or(false);

      if !is_inside {
        commands.entity(entity).remove::<PopupVisible>();
        if popup_resource.active_popup == Some(entity) {
          popup_resource.active_popup = None;
        }
      }
    }
  }
}
