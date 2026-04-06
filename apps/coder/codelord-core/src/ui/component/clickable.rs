//! Clickable component for interactive UI elements

use bevy_ecs::component::Component;

/// Component indicating this entity can be clicked
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clickable {
  /// Whether this element is currently enabled
  pub enabled: bool,
}

impl Default for Clickable {
  fn default() -> Self {
    Self { enabled: true }
  }
}

impl Clickable {
  pub fn new(enabled: bool) -> Self {
    Self { enabled }
  }

  pub fn is_enabled(&self) -> bool {
    self.enabled
  }
}

/// Component indicating this entity was clicked this frame
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clicked;
