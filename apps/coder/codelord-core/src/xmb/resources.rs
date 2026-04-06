use super::components::{XmbAction, XmbCategory, XmbItem, XmbNavigation};
use crate::animation::hacker::HackerAnimation;
use crate::icon::components::{Dot, Folder, Icon, Structure};
use crate::page::components::Page;

use bevy_ecs::message::Message;
use bevy_ecs::resource::Resource;
use eazy::Curve;
use eazy::easing::Easing;

/// Command to control XMB navigation.
#[derive(Message, Debug, Clone, Copy)]
pub struct XmbCommand {
  pub navigation: XmbNavigation,
}

/// Resource tracking XMB state.
#[derive(Resource, Debug, Clone)]
pub struct XmbResource {
  /// The main data structure for the menu.
  pub categories: Vec<XmbCategory>,
  /// The index of the currently focused category (X-axis).
  pub focused_x: usize,
  /// The index of the currently focused item in that category (Y-axis).
  pub focused_y: usize,
  /// The smoothly animated focus position (x, y).
  pub current_focus_pos: (f32, f32),
  /// The destination focus position (x, y).
  pub target_focus_pos: (f32, f32),
  /// Elapsed animation time (accumulated from DeltaTime).
  pub animation_elapsed: f32,
  /// The last selected action (for systems to process).
  pub pending_action: Option<XmbAction>,
  /// Track if animation was active last frame (for ActiveAnimations).
  was_animating: bool,
  /// Hacker animation for description text.
  pub description_anim: Option<HackerAnimation>,
}

impl Default for XmbResource {
  fn default() -> Self {
    let categories = Self::create_default_categories();
    // Initialize description animation with first item's description.
    let description_anim = categories
      .first()
      .and_then(|c| c.items.first())
      .map(|item| HackerAnimation::new(&item.description));

    Self {
      categories,
      focused_x: 0,
      focused_y: 0,
      current_focus_pos: (0.0, 0.0),
      target_focus_pos: (0.0, 0.0),
      animation_elapsed: 1.0, // Start as complete
      pending_action: None,
      was_animating: false,
      description_anim,
    }
  }
}

impl XmbResource {
  pub fn new() -> Self {
    Self::default()
  }

  /// Create the default category structure (matching codelord).
  fn create_default_categories() -> Vec<XmbCategory> {
    vec![
      XmbCategory {
        name: "HACKERSPACE".into(),
        icon: Icon::Hacker,
        color: [255, 255, 255, 255],
        items: vec![
          XmbItem {
            name: "New File".into(),
            description: "Start with a blank canvas. Your next creation awaits."
              .into(),
            icon: Icon::Add,
            action: XmbAction::NewFile,
          },
          XmbItem {
            name: "Open File".into(),
            description:
              "Jump back into a specific file and continue your work.".into(),
            icon: Icon::Structure(Structure::File),
            action: XmbAction::OpenFile,
          },
          XmbItem {
            name: "Open Folder".into(),
            description:
              "Load an entire project. A codebase is a world to explore.".into(),
            icon: Icon::Folder(Folder::Open),
            action: XmbAction::OpenFolder,
          },
        ],
      },
      XmbCategory {
        name: "SiMULATiONS".into(),
        icon: Icon::Binary,
        color: [255, 255, 255, 255],
        items: vec![
          XmbItem {
            name: "Code Editor".into(),
            description:
              "Enter the heart of the DEVOLUTiON. A high-performance environment built for speed and joy."
                .into(),
            icon: Icon::Code,
            action: XmbAction::SwitchToPage(Page::Editor),
          },
          XmbItem {
            name: "Playground".into(),
            description:
              "The zo laboratory. An interactive space to test, learn, and get instant feedback.".into(),
            icon: Icon::Ufo,
            action: XmbAction::SwitchToPage(Page::Playground),
          },
          XmbItem {
            name: "Copilord".into(),
            description: "Establish a direct link with your Ai mentor.".into(),
            icon: Icon::Copilord,
            action: XmbAction::SwitchToPage(Page::Playground),
          },
        ],
      },
      XmbCategory {
        name: "SETTiNGS".into(),
        icon: Icon::Server,
        color: [255, 255, 255, 255],
        items: vec![
          XmbItem {
            name: "Ui Theme".into(),
            description: "Cycle through installed visual interface themes.".into(),
            icon: Icon::Theme,
            action: XmbAction::OpenThemeSelector,
          },
          XmbItem {
            name: "Keybinds".into(),
            description: "Remap your keyboard and controller input schemes."
              .into(),
            icon: Icon::Keyboard,
            action: XmbAction::OpenSettings,
          },
          XmbItem {
            name: "Audio Hack".into(),
            description: "Adjust system sound levels and effects.".into(),
            icon: Icon::Sound,
            action: XmbAction::OpenSettings,
          },
          XmbItem {
            name: "More".into(),
            description: "Go deeper with the full settings.".into(),
            icon: Icon::Dot(Dot::Horizontal),
            action: XmbAction::OpenSettings,
          },
        ],
      },
    ]
  }

  /// Handle navigation and return action if Select was pressed.
  pub fn handle_navigation(
    &mut self,
    direction: XmbNavigation,
  ) -> Option<XmbAction> {
    let old_x = self.focused_x;
    let old_y = self.focused_y;
    let mut action = None;

    match direction {
      XmbNavigation::Right => {
        if self.focused_x < self.categories.len() - 1 {
          self.focused_x += 1;
          self.focused_y = 0;
        }
      }
      XmbNavigation::Left => {
        if self.focused_x > 0 {
          self.focused_x -= 1;
          self.focused_y = 0;
        }
      }
      XmbNavigation::Down => {
        let num_items = self.categories[self.focused_x].items.len();
        if num_items > 0 && self.focused_y < num_items - 1 {
          self.focused_y += 1;
        }
      }
      XmbNavigation::Up => {
        if self.focused_y > 0 {
          self.focused_y -= 1;
        }
      }
      XmbNavigation::Select => {
        if let Some(category) = self.categories.get(self.focused_x)
          && let Some(item) = category.items.get(self.focused_y)
        {
          action = Some(item.action.clone());
        }
      }
      XmbNavigation::JumpToCategory(idx) => {
        if idx < self.categories.len() && idx != self.focused_x {
          self.focused_x = idx;
          self.focused_y = 0;
        }
      }
      XmbNavigation::JumpToItem(idx) => {
        let num_items = self.categories[self.focused_x].items.len();
        if idx < num_items {
          if idx == self.focused_y {
            // Already focused - trigger select
            if let Some(category) = self.categories.get(self.focused_x)
              && let Some(item) = category.items.get(self.focused_y)
            {
              action = Some(item.action.clone());
            }
          } else {
            self.focused_y = idx;
          }
        }
      }
    }

    // If focus changed, update target position and reset animation
    if self.focused_x != old_x || self.focused_y != old_y {
      self.target_focus_pos = (self.focused_x as f32, self.focused_y as f32);
      self.animation_elapsed = 0.0;

      // Reset description hacker animation with new item's description.
      self.description_anim = self
        .categories
        .get(self.focused_x)
        .and_then(|c| c.items.get(self.focused_y))
        .map(|item| HackerAnimation::new(&item.description));
    }

    action
  }

  /// Update the focus animation using delta time and InOutSine easing.
  pub fn update_animation(&mut self, delta: f32) {
    let animation_duration = 0.35;

    // Accumulate elapsed time
    self.animation_elapsed += delta;
    let t = (self.animation_elapsed / animation_duration).min(1.0);

    // Use InOutSine easing from eazy
    let progress = Easing::InOutSine.y(t);

    // Interpolate from current toward target (matches codelord)
    let current = self.current_focus_pos;
    let target = self.target_focus_pos;
    self.current_focus_pos = (
      current.0 + (target.0 - current.0) * progress,
      current.1 + (target.1 - current.1) * progress,
    );
  }

  /// Check if animation is still in progress.
  pub fn is_animating(&self) -> bool {
    let epsilon = 0.001;
    (self.current_focus_pos.0 - self.target_focus_pos.0).abs() > epsilon
      || (self.current_focus_pos.1 - self.target_focus_pos.1).abs() > epsilon
  }

  /// Check animation state transition for ActiveAnimations tracking.
  /// Returns: (should_increment, should_decrement)
  pub fn check_animation_transition(&mut self) -> (bool, bool) {
    let currently_animating = self.is_animating();
    let was = self.was_animating;
    self.was_animating = currently_animating;

    match (was, currently_animating) {
      (false, true) => (true, false), // Started animating
      (true, false) => (false, true), // Stopped animating
      _ => (false, false),            // No transition
    }
  }

  /// Update the description hacker animation. Returns true if still animating.
  pub fn update_description_animation(&mut self, delta: f32) -> bool {
    if let Some(ref mut anim) = self.description_anim {
      anim.update(delta)
    } else {
      false
    }
  }

  /// Get the current animated description text.
  pub fn description_text(&self) -> Option<String> {
    self.description_anim.as_ref().map(|a| a.visible_text())
  }
}
