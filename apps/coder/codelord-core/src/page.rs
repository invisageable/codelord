pub mod components;
pub mod resources;
pub mod systems;

/// Insert page resources + message queues into the world.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::ecs::message::Messages;
  use crate::page::resources::{
    PageResource, PageSwitchCommand, PageSwitchEvent,
  };

  world.insert_resource(PageResource::default());
  world.init_resource::<Messages<PageSwitchCommand>>();
  world.init_resource::<Messages<PageSwitchEvent>>();
}

/// Register all page-related systems. Order matters for animation.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  use crate::ecs::schedule::IntoScheduleConfigs;

  schedule.add_systems(
    (
      systems::page_switch_command_system,
      systems::page_transition_update_system,
    )
      .chain(),
  );
}
