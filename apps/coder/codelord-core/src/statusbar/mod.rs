pub mod resources;
pub mod systems;

/// Insert statusbar resources.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::statusbar::resources::{LineColumnAnimation, StatusbarResource};

  world.insert_resource(StatusbarResource::new());
  world.insert_resource(LineColumnAnimation::new());
}

/// Register statusbar systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems(systems::line_column_animation_system);
}
