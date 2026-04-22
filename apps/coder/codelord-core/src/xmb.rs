pub mod components;
pub mod resources;
pub mod systems;

/// Insert XMB welcome-screen resource + message queue.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::ecs::message::Messages;

  world.insert_resource(resources::XmbResource::new());
  world.init_resource::<Messages<resources::XmbCommand>>();
}

/// Register XMB systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule
    .add_systems((systems::xmb_command_system, systems::xmb_action_system));
}
