use crate::icon::components::Icon;

use bevy_ecs::component::Component;

/// Button variant style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
  #[default]
  Primary,
  Secondary,
  Ghost,
}

/// Button content.
#[derive(Debug, Clone)]
pub enum ButtonContent {
  Label(&'static str),
  Icon(Icon),
  IconLabel(Icon, &'static str),
}

/// Button component.
#[derive(Component, Debug, Clone)]
pub struct Button {
  pub content: ButtonContent,
  pub variant: ButtonVariant,
}

impl Button {
  pub fn primary(content: ButtonContent) -> Self {
    Self {
      content,
      variant: ButtonVariant::Primary,
    }
  }

  pub fn secondary(content: ButtonContent) -> Self {
    Self {
      content,
      variant: ButtonVariant::Secondary,
    }
  }

  pub fn ghost(content: ButtonContent) -> Self {
    Self {
      content,
      variant: ButtonVariant::Ghost,
    }
  }
}
