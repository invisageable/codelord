//! Terminal ECS module.
//!
//! Provides pure ECS components, resources, and systems for terminal
//! management. The actual PTY bridge (AlacrittyBridge) is managed in
//! codelord-components since it has heavy dependencies.

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{
  CELL_FLAG_BOLD, CELL_FLAG_DIM, CELL_FLAG_ITALIC, CELL_FLAG_REVERSE,
  CELL_FLAG_STRIKETHROUGH, CELL_FLAG_UNDERLINE, TerminalCell, TerminalCursor,
  TerminalGrid, TerminalScroll, TerminalSelection, TerminalTab, pack_color,
  unpack_color,
};

pub use resources::{
  TerminalBridges, TerminalConfig, TerminalId, TerminalIdCounter,
  TerminalRegistry, TerminalTabOrderCounter,
};

/// Insert terminal resources.
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(TerminalIdCounter::default());
  world.insert_resource(TerminalTabOrderCounter::default());
  world.insert_resource(TerminalRegistry::default());
  world.insert_resource(TerminalBridges::default());
}

/// Register terminal systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::new_terminal_system,
    systems::new_terminal_tab_system,
    systems::close_terminal_system,
    systems::activate_terminal_system,
  ));
}
