use crate::events::{ActivateTabRequest, NewPlaygroundTabRequest};
use crate::keyboard::{Focusable, KeyboardHandler};
use crate::tabbar::components::{PlaygroundTab, SonarAnimation, Tab};
use crate::tabbar::resources::TabOrderCounter;
use crate::text_editor::components::{Cursor, FileTab, TextBuffer};
use crate::ui::component::Active;

use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query, ResMut};

/// System: processes NewPlaygroundTabRequest and creates a playground tab.
pub fn new_playground_tab_system(
  mut commands: Commands,
  requests: Query<Entity, With<NewPlaygroundTabRequest>>,
  active_tabs: Query<Entity, (With<PlaygroundTab>, With<Active>)>,
  mut tab_order: ResMut<TabOrderCounter>,
) {
  for event_entity in requests.iter() {
    // Deactivate all active playground tabs.
    for active_entity in active_tabs.iter() {
      commands.entity(active_entity).remove::<Active>();
    }

    // Create new playground tab.
    let order = tab_order.next();
    let label = format!("playground-{}", order + 1);

    commands.spawn((
      Tab::new(label.clone(), order),
      PlaygroundTab,
      SonarAnimation::default(),
      TextBuffer::empty(),
      Cursor::new(0),
      FileTab::new(format!("{}.zo", label)), // For zo syntax highlighting
      Active,
      Focusable,
      KeyboardHandler::text_editor(),
    ));

    // Despawn the event (one-shot).
    commands.entity(event_entity).despawn();
  }
}

/// System: processes ActivateTabRequest for playground tabs.
pub fn activate_playground_tab_system(
  mut commands: Commands,
  requests: Query<(Entity, &ActivateTabRequest)>,
  tabs: Query<Entity, With<PlaygroundTab>>,
  active_tabs: Query<Entity, (With<PlaygroundTab>, With<Active>)>,
) {
  for (event_entity, request) in requests.iter() {
    let target_tab = request.entity;

    // Check if this is a playground tab.
    if !tabs.iter().any(|e| e == target_tab) {
      continue;
    }

    // Deactivate all active playground tabs.
    for active_entity in active_tabs.iter() {
      commands.entity(active_entity).remove::<Active>();
    }

    // Activate target tab.
    commands.entity(target_tab).insert(Active);

    // Despawn the event (one-shot).
    commands.entity(event_entity).despawn();
  }
}
