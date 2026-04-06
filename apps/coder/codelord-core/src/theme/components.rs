use bevy_ecs::component::Component;

/// Marker component for entities that should respond to theme changes
///
/// Any entity with this component will be notified when the theme changes
/// and can update its visual appearance accordingly.
#[derive(Component, Debug, Clone, Copy)]
pub struct Themed;

/// Component tracking the last theme generation this entity saw
///
/// Used for change detection - when ThemeResource generation increments,
/// entities with stale ThemeGeneration are updated.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeGeneration(pub u64);

impl ThemeGeneration {
  pub fn new(generation: u64) -> Self {
    Self(generation)
  }

  pub fn is_stale(&self, current: u64) -> bool {
    self.0 < current
  }
}

/// Component for entities that want to override the global theme
///
/// Useful for preview panels, theme selection UI, or special-purpose
/// UI elements that need different theming.
#[derive(Component, Debug, Clone, Copy)]
pub struct ThemeOverride {
  /// The theme kind to use instead of the global theme
  pub kind: ThemeKind,
}

/// Theme kind selector - the source of truth for theme selection
///
/// This is the single source of truth for which theme is active.
/// codelord-components reads this to get the actual color palette.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeKind {
  /// Dark theme
  #[default]
  Dark,
  /// Light theme
  Light,
  /// Custom theme - TODO: load from TOML file
  Custom,
}
