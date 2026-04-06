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
