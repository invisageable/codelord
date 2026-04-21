pub mod components;
pub mod resources;
pub mod systems;

/// Insert toast resources + message queues.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::ecs::message::Messages;

  world.insert_resource(resources::ToasterResource::default());
  world.init_resource::<Messages<resources::ToastCommand>>();
  world.init_resource::<Messages<resources::DismissToastCommand>>();
}

/// Register toast systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::process_toast_commands,
    systems::process_dismiss_commands,
    systems::update_toast_animations,
  ));
}
