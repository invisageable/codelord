use super::components::ThemeKind;

use bevy_ecs::message::Message;
use bevy_ecs::resource::Resource;

/// Global theme state resource
///
/// Holds the current active theme and tracks changes via generation counter.
/// When the theme changes, the generation increments, triggering updates
/// in all themed entities.
#[derive(Resource, Debug, Default, Clone)]
pub struct ThemeResource {
  /// Current active theme kind
  pub current: ThemeKind,

  /// Generation counter - increments on every theme change
  ///
  /// Used for efficient change detection. Entities track which generation
  /// they last saw, and only update when the generation increases.
  pub generation: u64,

  /// Whether theme hot-reload is enabled
  ///
  /// When true, the system will watch for theme file changes and
  /// automatically reload. Useful during theme development.
  pub hot_reload: bool,
}

impl ThemeResource {
  /// Create a new theme resource with specified theme
  pub fn new(theme: ThemeKind) -> Self {
    Self {
      current: theme,
      generation: 0,
      hot_reload: false,
    }
  }

  /// Switch to a new theme, incrementing generation
  pub fn set_theme(&mut self, theme: ThemeKind) {
    if self.current != theme {
      self.current = theme;
      self.generation = self.generation.wrapping_add(1);
    }
  }

  /// Get current generation for change detection
  pub fn generation(&self) -> u64 {
    self.generation
  }

  /// Enable/disable hot reload
  pub fn set_hot_reload(&mut self, enabled: bool) {
    self.hot_reload = enabled;
  }
}

/// Command to request theme changes
///
/// UI components send this message to request theme switches.
/// A system will handle it and update the ThemeResource.
#[derive(Message, Debug, Clone, Copy)]
pub struct ThemeCommand {
  pub action: ThemeAction,
}

#[derive(Debug, Clone, Copy)]
pub enum ThemeAction {
  /// Set to specific theme
  Set(ThemeKind),
  /// Toggle between Dark and Light
  Toggle,
}

/// Resource for tracking theme change events
///
/// Alternative to the resource-based approach - this uses a message queue
/// for theme changes. Can be used alongside or instead of generation tracking.
#[derive(Message, Debug, Clone, Copy)]
pub struct ThemeChangedEvent {
  pub old_theme: ThemeKind,
  pub new_theme: ThemeKind,
}
