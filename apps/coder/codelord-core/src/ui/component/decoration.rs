use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;

/// Window decoration type
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationType {
  Close,
  MinimizeMaximize,
  Fullscreen,
}

/// Window decoration entity bundle
#[derive(Bundle)]
pub struct DecorationBundle {
  pub decoration_type: DecorationType,
  pub hovered: super::hovered::Hovered,
  pub focused: super::focused::Focused,
  pub clickable: super::clickable::Clickable,
}

impl DecorationBundle {
  pub fn new(decoration_type: DecorationType) -> Self {
    Self {
      decoration_type,
      hovered: super::hovered::Hovered,
      focused: super::focused::Focused,
      clickable: super::clickable::Clickable::default(),
    }
  }
}
