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

/// Close out the continuous-animation bookkeeping for the frame:
/// drain `ContinuousAnimations::end_frame()` increments/decrements into
/// `ActiveAnimations`, then report whether any animation is still
/// active so the caller knows to request a repaint.
pub fn end_frame(world: &mut crate::ecs::world::World) -> bool {
  use crate::animation::resources::{ActiveAnimations, ContinuousAnimations};

  let changes = world
    .get_resource_mut::<ContinuousAnimations>()
    .map(|mut cont| cont.end_frame());

  if let Some((increments, decrements)) = changes
    && (increments > 0 || decrements > 0)
    && let Some(mut active) = world.get_resource_mut::<ActiveAnimations>()
  {
    (0..increments).for_each(|_| active.increment());
    (0..decrements).for_each(|_| active.decrement());
  }

  world
    .get_resource::<ActiveAnimations>()
    .map(|a| a.has_active())
    .unwrap_or(false)
}
