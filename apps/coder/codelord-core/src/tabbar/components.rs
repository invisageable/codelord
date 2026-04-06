use crate::ecs::world::World;
use crate::tabbar::ZoomSource;

use bevy_ecs::component::Component;
use eazy::{Curve, Easing};

/// Trait for tab marker components that can spawn new tabs.
/// Implements pure ECS pattern - each marker defines its own event.
pub trait TabMarker: Component {
  /// Spawn the appropriate new tab event for this marker type.
  fn spawn_new_tab_event(world: &mut World);

  /// Get the zoom source for this marker type.
  fn zoom_source() -> ZoomSource;
}

/// Core tab component - context-agnostic.
#[derive(Component, Debug, Clone)]
pub struct Tab {
  pub label: String,
  pub closable: bool,
  pub order: u32,
}

impl Tab {
  pub fn new(label: impl Into<String>, order: u32) -> Self {
    Self {
      label: label.into(),
      closable: true,
      order,
    }
  }

  pub fn with_closable(mut self, closable: bool) -> Self {
    self.closable = closable;
    self
  }
}

/// Marker: tab belongs to the text editor context.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct EditorTab;

impl TabMarker for EditorTab {
  fn spawn_new_tab_event(world: &mut World) {
    world.spawn(crate::events::NewEditorTabRequest);
  }

  fn zoom_source() -> ZoomSource {
    ZoomSource::Editor
  }
}

/// Marker: tab belongs to the playground context.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlaygroundTab;

impl TabMarker for PlaygroundTab {
  fn spawn_new_tab_event(world: &mut World) {
    world.spawn(crate::events::NewPlaygroundTabRequest);
  }

  fn zoom_source() -> ZoomSource {
    ZoomSource::Playground
  }
}

/// Sonar wave animation for modified indicator on tabs.
/// Uses absolute time (from egui) for animation.
/// Direct migration from codelord-data/src/sonarwave_animation.rs
#[derive(Component, Debug, Clone)]
pub struct SonarAnimation {
  /// The animation start time (in seconds, from ui.input(|i| i.time)).
  pub start_time: Option<f64>,
  /// Duration of a single wave cycle (in seconds).
  pub duration: f64,
  /// Whether the animation loops continuously.
  pub is_looping: bool,
  /// Initial scale factor (1.0 = normal size).
  pub initial_scale: f32,
  /// Final scale factor (e.g., 2.0 = 2x size).
  pub final_scale: f32,
  /// Initial opacity (0.0 - 1.0).
  pub initial_opacity: f32,
  /// Final opacity (typically 0.0 for fade out).
  pub final_opacity: f32,
  /// Easing function for the animation.
  pub easing: Easing,
}

impl Default for SonarAnimation {
  fn default() -> Self {
    Self {
      start_time: None,
      duration: 1.2,
      is_looping: true,
      initial_scale: 1.0,
      final_scale: 2.0,
      initial_opacity: 0.4,
      final_opacity: 0.0,
      easing: Easing::OutCubic,
    }
  }
}

impl SonarAnimation {
  /// Start or restart the animation with current time.
  pub fn start(&mut self, current_time: f64) {
    self.start_time = Some(current_time);
  }

  /// Stop the animation.
  pub fn stop(&mut self) {
    self.start_time = None;
  }

  /// Check if animation is active.
  pub fn is_active(&self) -> bool {
    self.start_time.is_some()
  }

  /// Calculate current state (scale, opacity) using easing.
  pub fn calculate_state(&mut self, current_time: f64) -> (f32, f32) {
    let Some(start_time) = self.start_time else {
      return (self.initial_scale, 0.0);
    };

    let elapsed = current_time - start_time;
    let mut progress = (elapsed / self.duration).min(1.0) as f32;

    // Handle looping
    if progress >= 1.0 && self.is_looping {
      self.start_time = Some(current_time);
      progress = 0.0;
    }

    // Use eazy easing
    let eased = self.easing.y(progress);

    let scale =
      self.initial_scale + (self.final_scale - self.initial_scale) * eased;
    let opacity = self.initial_opacity
      + (self.final_opacity - self.initial_opacity) * eased;

    (scale, opacity)
  }
}
