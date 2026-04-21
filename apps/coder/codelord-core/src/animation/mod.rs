pub mod components;
pub mod hacker;
pub mod height;
pub mod interpolate;
pub mod opacity;
pub mod resources;
pub mod shimmer;
pub mod systems;

/// Insert animation core resources (delta time, active/continuous counters).
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::animation::components::DeltaTime;
  use crate::animation::resources::{ActiveAnimations, ContinuousAnimations};

  world.insert_resource(DeltaTime::default());
  world.insert_resource(ActiveAnimations::default());
  world.insert_resource(ContinuousAnimations::default());
}
