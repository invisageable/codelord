//! Keyboard focus components.

use bevy_ecs::component::Component;

/// Component: marks an entity as focusable.
///
/// Entities with this component can receive keyboard focus.
/// When focused, they will receive keyboard events.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Focusable;

/// Component: keyboard action handler.
///
/// Attach to a focusable entity to define what actions it supports.
/// The UI layer reads this to know what keyboard events to spawn.
#[derive(Component, Debug, Clone, Default)]
pub struct KeyboardHandler {
  /// Supports text input (typing characters).
  pub text_input: bool,
  /// Supports cursor movement (arrow keys).
  pub cursor_movement: bool,
  /// Supports deletion (backspace, delete).
  pub deletion: bool,
  /// Supports selection (shift + movement).
  pub selection: bool,
}

impl KeyboardHandler {
  /// Create a handler for a text editor (supports everything).
  pub fn text_editor() -> Self {
    Self {
      text_input: true,
      cursor_movement: true,
      deletion: true,
      selection: true,
    }
  }

  /// Create a handler for navigation only (arrow keys, no text input).
  pub fn navigation() -> Self {
    Self {
      text_input: false,
      cursor_movement: true,
      deletion: false,
      selection: false,
    }
  }
}
