use bevy_ecs::component::Component;

/// Views available in the right panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RightPanelView {
  #[default]
  Copilord,
  WebView,
}

/// Views available in the bottom panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BottomPanelView {
  #[default]
  Terminal,
  Problems,
  Output,
}

/// Views available in the left panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LeftPanelView {
  #[default]
  Explorer,
  Collaboration,
  VersionControl,
}
