//! Terminal ECS systems.
//!
//! Note: The actual terminal bridge (AlacrittyBridge) is managed in
//! codelord-components due to heavy dependencies. These systems handle
//! ECS event processing.

use crate::events::{
  CloseTerminalRequest, NewTerminalRequest, NewTerminalTabRequest,
  TerminalInputEvent, TerminalResizeEvent, TerminalScrollEvent,
};
use crate::tabbar::Tab;
use crate::terminal::components::{
  TerminalCursor, TerminalGrid, TerminalScroll, TerminalTab,
};
use crate::terminal::resources::{
  TerminalIdCounter, TerminalRegistry, TerminalTabOrderCounter,
};
use crate::ui::component::Active;

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::*;

/// System to process new terminal requests.
/// Creates terminal entities with initial components.
pub fn new_terminal_system(
  mut commands: Commands,
  query: Query<(Entity, &NewTerminalRequest)>,
  mut id_counter: ResMut<TerminalIdCounter>,
  mut order_counter: ResMut<TerminalTabOrderCounter>,
  mut registry: ResMut<TerminalRegistry>,
) {
  for (event_entity, _request) in query.iter() {
    // Generate ID and order
    let terminal_id = id_counter.bump();
    let order = order_counter.allocate();

    // Spawn terminal entity with components
    let terminal_entity = commands
      .spawn((
        Tab::new(format!("Terminal {}", terminal_id.0 + 1), order),
        TerminalTab,
        TerminalGrid::default(),
        TerminalCursor::new(),
        TerminalScroll::default(),
        Active, // New terminals are active by default
      ))
      .id();

    registry.register(terminal_entity, terminal_id);
    commands.entity(event_entity).despawn();
  }
}

/// System to process NewTerminalTabRequest (from tabbar + button).
/// Deactivates all active terminal tabs before creating new one.
pub fn new_terminal_tab_system(
  mut commands: Commands,
  requests: Query<Entity, With<NewTerminalTabRequest>>,
  active_tabs: Query<Entity, (With<TerminalTab>, With<Active>)>,
  mut id_counter: ResMut<TerminalIdCounter>,
  mut order_counter: ResMut<TerminalTabOrderCounter>,
  mut registry: ResMut<TerminalRegistry>,
) {
  for event_entity in requests.iter() {
    // Deactivate all active terminal tabs
    for active_entity in active_tabs.iter() {
      commands.entity(active_entity).remove::<Active>();
    }

    // Generate ID and order
    let terminal_id = id_counter.bump();
    let order = order_counter.allocate();

    // Spawn terminal entity with components
    let terminal_entity = commands
      .spawn((
        Tab::new(format!("Terminal {}", terminal_id.0 + 1), order),
        TerminalTab,
        TerminalGrid::default(),
        TerminalCursor::new(),
        TerminalScroll::default(),
        Active,
      ))
      .id();

    registry.register(terminal_entity, terminal_id);
    commands.entity(event_entity).despawn();
  }
}

/// System to process close terminal requests.
pub fn close_terminal_system(
  mut commands: Commands,
  query: Query<(Entity, &CloseTerminalRequest)>,
  mut registry: ResMut<TerminalRegistry>,
) {
  for (event_entity, request) in query.iter() {
    registry.unregister(request.entity);
    commands.entity(request.entity).despawn();
    commands.entity(event_entity).despawn();
  }
}

/// System to process terminal input events.
/// Note: Actual input sending is handled by the bridge in codelord-components.
/// This system just cleans up the event entities.
pub fn terminal_input_system(
  mut commands: Commands,
  query: Query<Entity, With<TerminalInputEvent>>,
) {
  for event_entity in query.iter() {
    // Input is processed by the UI layer which has access to the bridge
    // We just clean up the event entity here
    commands.entity(event_entity).despawn();
  }
}

/// System to process terminal resize events.
pub fn terminal_resize_system(
  mut commands: Commands,
  query: Query<(Entity, &TerminalResizeEvent)>,
  mut grids: Query<&mut TerminalGrid>,
) {
  for (event_entity, event) in query.iter() {
    if let Ok(mut grid) = grids.get_mut(event.entity) {
      grid.resize(event.cols, event.rows);
    }

    commands.entity(event_entity).despawn();
  }
}

/// System to process terminal scroll events.
pub fn terminal_scroll_system(
  mut commands: Commands,
  query: Query<(Entity, &TerminalScrollEvent)>,
  mut scrolls: Query<&mut TerminalScroll>,
) {
  for (event_entity, event) in query.iter() {
    if let Ok(mut scroll) = scrolls.get_mut(event.entity) {
      scroll.offset += event.delta as f32;
      scroll.offset = scroll.offset.max(0.0);
    }

    commands.entity(event_entity).despawn();
  }
}

/// System to activate terminal tabs (similar to editor tabs).
pub fn activate_terminal_system(
  mut commands: Commands,
  query: Query<(Entity, &crate::events::ActivateTabRequest)>,
  terminals: Query<Entity, With<TerminalTab>>,
) {
  for (event_entity, request) in query.iter() {
    if terminals.get(request.entity).is_ok() {
      for entity in terminals.iter() {
        commands.entity(entity).remove::<Active>();
      }

      commands.entity(request.entity).insert(Active);
      commands.entity(event_entity).despawn();
    }
  }
}
