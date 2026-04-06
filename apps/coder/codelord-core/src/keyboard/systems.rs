//! Keyboard focus systems.

use super::components::Focusable;
use super::resources::KeyboardFocus;

use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Commands, Query, ResMut};

/// Event: request to set keyboard focus to an entity.
#[derive(bevy_ecs::component::Component, Debug, Clone, Copy)]
pub struct FocusRequest {
  pub entity: Entity,
}

impl FocusRequest {
  pub fn new(entity: Entity) -> Self {
    Self { entity }
  }
}

/// Event: request to clear keyboard focus.
#[derive(bevy_ecs::component::Component, Debug, Clone, Copy, Default)]
pub struct ClearFocusRequest;

/// System: processes FocusRequest events and updates KeyboardFocus resource.
pub fn focus_request_system(
  mut commands: Commands,
  mut focus: ResMut<KeyboardFocus>,
  requests: Query<(Entity, &FocusRequest)>,
  focusables: Query<&Focusable>,
) {
  for (event_entity, request) in requests.iter() {
    // Only set focus if the target entity is focusable
    if focusables.get(request.entity).is_ok() {
      focus.set(request.entity);
    }

    // Despawn the event
    commands.entity(event_entity).despawn();
  }
}

/// System: processes ClearFocusRequest events.
pub fn clear_focus_system(
  mut commands: Commands,
  mut focus: ResMut<KeyboardFocus>,
  requests: Query<Entity, bevy_ecs::query::With<ClearFocusRequest>>,
) {
  for event_entity in requests.iter() {
    focus.clear();
    commands.entity(event_entity).despawn();
  }
}
