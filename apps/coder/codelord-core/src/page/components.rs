use bevy_ecs::prelude::*;

/// Page component for different IDE views
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
  #[default]
  Welcome,
  Editor,
  Playground,
  Notes,
  Presenter,
  Settings,
  About,
}

impl Page {
  /// Get the next page in navigation order
  pub fn next(self) -> Self {
    match self {
      Page::Welcome => Page::Editor,
      Page::Editor => Page::Playground,
      Page::Playground => Page::Notes,
      Page::Notes => Page::Presenter,
      Page::Presenter => Page::Welcome,
      Page::Settings => Page::Welcome,
      Page::About => Page::Welcome,
    }
  }

  /// Get the previous page in navigation order
  pub fn previous(self) -> Self {
    match self {
      Page::Welcome => Page::Presenter,
      Page::Editor => Page::Welcome,
      Page::Playground => Page::Editor,
      Page::Notes => Page::Playground,
      Page::Presenter => Page::Notes,
      Page::Settings => Page::Welcome,
      Page::About => Page::Welcome,
    }
  }
}

/// Marker component for the currently active page entity
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePage;

/// Direction of the slide animation between pages (distinct from
/// [`crate::codeshow::SlideDirection`], which is presentation-slide
/// navigation).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum TransitionDirection {
  /// New page slides in from the right.
  Left,
  /// New page slides in from the left.
  Right,
}
