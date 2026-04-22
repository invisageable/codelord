pub mod components;
pub mod resources;
pub mod systems;

pub use components::{Focusable, KeyboardHandler};
pub use resources::KeyboardFocus;
pub use systems::{ClearFocusRequest, FocusRequest};

/// Insert keyboard focus resource.
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(KeyboardFocus::new());
}

/// Register keyboard focus systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule
    .add_systems((systems::focus_request_system, systems::clear_focus_system));
}
