use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;

/// Resource: tracks which entity currently has keyboard focus.
///
/// Only one entity can have keyboard focus at a time.
/// When an entity has focus, it receives keyboard events.
#[derive(Resource, Debug, Default)]
pub struct KeyboardFocus {
  /// The entity that currently has keyboard focus.
  focused: Option<Entity>,
}

impl KeyboardFocus {
  /// Create a new KeyboardFocus with no focus.
  pub fn new() -> Self {
    Self { focused: None }
  }

  /// Get the currently focused entity.
  pub fn get(&self) -> Option<Entity> {
    self.focused
  }

  /// Set focus to an entity.
  pub fn set(&mut self, entity: Entity) {
    self.focused = Some(entity);
  }

  /// Clear focus (no entity has focus).
  pub fn clear(&mut self) {
    self.focused = None;
  }

  /// Check if a specific entity has focus.
  pub fn has_focus(&self, entity: Entity) -> bool {
    self.focused == Some(entity)
  }

  /// Check if any entity has focus.
  pub fn is_focused(&self) -> bool {
    self.focused.is_some()
  }
}
