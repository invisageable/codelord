use super::resources::{
  PageResource, PageSwitchCommand, PageSwitchEvent, PageTransition,
};

use crate::animation::components::DeltaTime;
use crate::animation::resources::ActiveAnimations;

use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::system::{Res, ResMut};

/// System to handle page switch commands
pub fn page_switch_command_system(
  mut page_res: ResMut<PageResource>,
  mut commands_reader: MessageReader<PageSwitchCommand>,
  mut events_writer: MessageWriter<PageSwitchEvent>,
  mut active_animations: ResMut<ActiveAnimations>,
) {
  for command in commands_reader.read() {
    let new_page = command.page;

    // Don't switch if already on this page or if transitioning
    if page_res.active_page == new_page || page_res.is_transitioning() {
      continue;
    }

    // Start transition
    let transition = PageTransition::new(page_res.active_page, new_page);
    page_res.transition = Some(transition);

    // Emit event
    events_writer.write(PageSwitchEvent {
      from_page: page_res.active_page,
      to_page: new_page,
    });

    // Register active animation
    active_animations.increment();
  }
}

/// System to update page transitions
pub fn page_transition_update_system(
  mut page_res: ResMut<PageResource>,
  delta_time: Res<DeltaTime>,
  mut active_animations: ResMut<ActiveAnimations>,
) {
  let is_complete = if let Some(transition) = &mut page_res.transition {
    transition.update(delta_time.delta())
  } else {
    false
  };

  if is_complete && let Some(transition) = &page_res.transition {
    // Transition complete - update active page
    let to_page = transition.to_page;
    let from_page = page_res.active_page;

    page_res.previous_page = Some(from_page);
    page_res.active_page = to_page;
    page_res.transition = None;

    // Unregister animation
    active_animations.decrement();
  }
}
