use bevy_ecs::component::Component;

/// Unique identifier for popup instances.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PopupId(pub usize);

impl PopupId {
  pub fn new() -> Self {
    static COUNTER: std::sync::atomic::AtomicUsize =
      std::sync::atomic::AtomicUsize::new(0);
    Self(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
  }
}

impl Default for PopupId {
  fn default() -> Self {
    Self::new()
  }
}

/// Position of the popup relative to trigger.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PopupPosition {
  #[default]
  Below,
  Above,
  Right,
  Left,
  Cursor,
  Absolute(f32, f32),
}

/// Type of popup content.
#[derive(Debug, Clone, PartialEq)]
pub enum PopupContent {
  Menu(Vec<MenuItem>),
  Custom(String),
}

/// Individual menu item.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuItem {
  pub id: String,
  pub label: String,
  pub icon: Option<String>,
  pub shortcut: Option<String>,
  pub enabled: bool,
  pub separator_after: bool,
}

impl MenuItem {
  pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
    Self {
      id: id.into(),
      label: label.into(),
      icon: None,
      shortcut: None,
      enabled: true,
      separator_after: false,
    }
  }

  pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
    self.shortcut = Some(shortcut.into());
    self
  }

  pub fn disabled(mut self) -> Self {
    self.enabled = false;
    self
  }

  pub fn with_separator(mut self) -> Self {
    self.separator_after = true;
    self
  }
}

/// Popup state component attached to popup entities.
#[derive(Component, Debug, Clone)]
pub struct Popup {
  pub position: PopupPosition,
  pub content: PopupContent,
  pub anchor_rect: Option<[f32; 4]>,
  pub auto_close: bool,
}

impl Popup {
  pub fn new(content: PopupContent) -> Self {
    Self {
      position: PopupPosition::Below,
      content,
      anchor_rect: None,
      auto_close: true,
    }
  }

  pub fn with_position(mut self, position: PopupPosition) -> Self {
    self.position = position;
    self
  }
}

/// Marker component for visible popups.
#[derive(Component, Debug, Default)]
pub struct PopupVisible;
