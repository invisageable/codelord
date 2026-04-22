pub mod components;
pub mod resources;
pub mod systems;

/// Insert popup resources + message queue.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::ecs::message::Messages;

  world.insert_resource(resources::PopupResource::new());
  world.init_resource::<Messages<resources::PopupCommand>>();
}

/// Register popup systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems(systems::popup_command_system);
}
