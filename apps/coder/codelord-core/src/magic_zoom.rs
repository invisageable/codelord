//! Screen-Studio-style magic zoom.
//!
//! Hold a hotkey: content smoothly zooms toward the cursor. Release: eases
//! back. Applied as an egui `TSTransform` on a dedicated layer — no
//! per-frame allocations, no font re-rasterization (v1 accepts text blur
//! during motion).

pub mod messages;
pub mod resources;
pub mod systems;

pub use messages::MagicZoomCommand;
pub use resources::MagicZoomState;
pub use systems::update_magic_zoom_system;

/// Insert magic zoom state + command queue into the world.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::ecs::message::Messages;

  world.insert_resource(MagicZoomState::default());
  world.init_resource::<Messages<MagicZoomCommand>>();
}

/// Register the magic zoom camera update system.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems(update_magic_zoom_system);
}
