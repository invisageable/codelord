use crate::page::components::{Page, SlideDirection};

use bevy_ecs::message::Message;
use bevy_ecs::resource::Resource;
use eazy::easing::{Curve, Easing};

/// Message to switch to a different page
#[derive(Message, Debug, Clone, Copy)]
pub struct PageSwitchCommand {
  pub page: Page,
}

/// Event emitted when page switch starts
#[derive(Message, Debug, Clone, Copy)]
pub struct PageSwitchEvent {
  pub from_page: Page,
  pub to_page: Page,
}

/// State for managing smooth page transitions
#[derive(Debug, Clone)]
pub struct PageTransition {
  /// The page we are animating FROM
  pub from_page: Page,
  /// The page we are animating TO
  pub to_page: Page,
  /// Animation progress from 0.0 (start) to 1.0 (end)
  pub progress: f32,
  /// Duration of the animation in seconds
  pub duration: f32,
  /// Elapsed time
  pub elapsed: f32,
  /// The direction of the slide
  pub direction: SlideDirection,
}

impl PageTransition {
  pub fn new(from: Page, to: Page) -> Self {
    // Determine slide direction
    let direction = match (from, to) {
      (Page::Welcome, Page::Editor) => SlideDirection::Left,
      (Page::Editor, Page::Playground) => SlideDirection::Left,
      (Page::Playground, Page::Presenter) => SlideDirection::Left,
      (Page::Presenter, Page::Welcome) => SlideDirection::Left,
      (Page::Editor, Page::Welcome) => SlideDirection::Right,
      (Page::Playground, Page::Editor) => SlideDirection::Right,
      (Page::Presenter, Page::Playground) => SlideDirection::Right,
      (Page::Welcome, Page::Presenter) => SlideDirection::Right,
      _ => SlideDirection::Left,
    };

    Self {
      from_page: from,
      to_page: to,
      progress: 0.0,
      duration: 0.3,
      elapsed: 0.0,
      direction,
    }
  }

  /// Update the animation progress
  pub fn update(&mut self, delta: f32) -> bool {
    self.elapsed += delta;
    self.progress = (self.elapsed / self.duration).min(1.0);

    self.progress >= 1.0 // return true if animation is complete
  }

  /// Get the eased progress (smooth in-out)
  pub fn eased_progress(&self) -> f32 {
    Easing::InOutCubic.y(self.progress)
  }
}

/// Resource managing current page state
#[derive(Resource, Debug)]
pub struct PageResource {
  pub active_page: Page,
  pub previous_page: Option<Page>,
  pub transition: Option<PageTransition>,
}

impl PageResource {
  pub fn new(initial_page: Page) -> Self {
    Self {
      active_page: initial_page,
      previous_page: None,
      transition: None,
    }
  }

  pub fn is_transitioning(&self) -> bool {
    self.transition.is_some()
  }
}

impl Default for PageResource {
  fn default() -> Self {
    Self::new(Page::Welcome)
  }
}
