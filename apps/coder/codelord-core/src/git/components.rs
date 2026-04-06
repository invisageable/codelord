//! Git ECS components.

use crate::animation::hacker::HackerAnimation;
use crate::animation::opacity::OpacityAnimation;

use codelord_git::Blame;

use bevy_ecs::component::Component;

use std::path::PathBuf;

/// Cached blame data for a file tab.
#[derive(Component, Debug)]
pub struct TabBlame {
  /// Parsed blame data.
  pub blame: Option<Blame>,
  /// Whether blame is currently being fetched.
  pub loading: bool,
  /// Whether blame display is enabled for this tab.
  pub enabled: bool,
  /// Repository root path (for blame commands).
  pub repo_root: Option<PathBuf>,
  /// Current line being animated (to detect line changes).
  pub animated_line: Option<usize>,
  /// Opacity animation for fade-in effect.
  pub opacity_anim: OpacityAnimation,
  /// Hacker animation for text effect.
  pub hacker_anim: Option<HackerAnimation>,
}

impl Default for TabBlame {
  fn default() -> Self {
    Self::new()
  }
}

impl TabBlame {
  /// Create new empty blame state with blame enabled by default.
  pub fn new() -> Self {
    Self {
      blame: None,
      loading: false,
      enabled: true,
      repo_root: None,
      animated_line: None,
      opacity_anim: OpacityAnimation::fade_in(),
      hacker_anim: None,
    }
  }

  /// Create with specific enabled state (from settings).
  pub fn with_enabled(enabled: bool) -> Self {
    Self {
      blame: None,
      loading: false,
      enabled,
      repo_root: None,
      animated_line: None,
      opacity_anim: OpacityAnimation::fade_in(),
      hacker_anim: None,
    }
  }

  /// Enable blame display.
  pub fn enable(&mut self) {
    self.enabled = true;
  }

  /// Disable blame display.
  pub fn disable(&mut self) {
    self.enabled = false;
  }

  /// Toggle blame display.
  pub fn toggle(&mut self) {
    self.enabled = !self.enabled;
  }

  /// Set blame data after loading.
  pub fn set_blame(&mut self, blame: Blame) {
    self.blame = Some(blame);
    self.loading = false;
    // Reset animations when new data arrives
    self.animated_line = None;
  }

  /// Mark as loading.
  pub fn start_loading(&mut self) {
    self.loading = true;
  }

  /// Clear blame data.
  pub fn clear(&mut self) {
    self.blame = None;
    self.loading = false;
    self.animated_line = None;
    self.hacker_anim = None;
  }

  /// Start animation for a new line.
  pub fn start_line_animation(&mut self, line: usize, text: &str) {
    self.animated_line = Some(line);
    self.opacity_anim.reset();
    self.hacker_anim = Some(HackerAnimation::new(text));
  }

  /// Update animations. Returns true if still animating.
  pub fn update_animation(&mut self, dt: f32) -> bool {
    let opacity_animating = self.opacity_anim.update(dt);
    let hacker_animating = self
      .hacker_anim
      .as_mut()
      .map(|h| h.update(dt))
      .unwrap_or(false);

    opacity_animating || hacker_animating
  }

  /// Get current opacity value.
  pub fn opacity(&self) -> f32 {
    self.opacity_anim.opacity()
  }

  /// Get animated text (hacker effect).
  pub fn animated_text(&self) -> Option<String> {
    self.hacker_anim.as_ref().map(|h| h.visible_text())
  }

  /// Check if animation is currently active (without mutating).
  pub fn is_animating(&self) -> bool {
    !self.opacity_anim.is_finished()
      || self
        .hacker_anim
        .as_ref()
        .map(|h| !h.is_finished())
        .unwrap_or(false)
  }
}
