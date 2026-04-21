pub mod components;
pub mod resources;
pub mod systems;

/// Insert filescope state + matcher into the world.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::filescope::resources::{FilescopeMatcher, FilescopeState};

  world.insert_resource(FilescopeState::default());
  world.insert_resource(FilescopeMatcher::new());
}

/// Register filescope systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::filescope_populate_system,
    systems::filescope_tick_system,
  ));
}
