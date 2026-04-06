//! Drag and drop animation state.

use bevy_ecs::resource::Resource;
use eazy::easing::{Curve, Easing};
use rustc_hash::FxHashMap;

/// Animation entry for a single item.
#[derive(Clone)]
struct AnimEntry {
  start: f32,
  target: f32,
  progress: f32,
}

/// Animated position state for drag reordering.
#[derive(Resource, Default)]
pub struct DragAnimationState {
  entries: FxHashMap<String, AnimEntry>,
}

impl DragAnimationState {
  /// Set target position for an item. Returns current animated position.
  pub fn animate_to(&mut self, key: &str, target: f32, dt: f32) -> f32 {
    let speed = 5.0;

    let entry = self.entries.entry(key.to_string()).or_insert(AnimEntry {
      start: target,
      target,
      progress: 1.0,
    });

    // If target changed, restart animation from current position
    if (entry.target - target).abs() > 0.01 {
      let current = Self::interpolate(entry);
      entry.start = current;
      entry.target = target;
      entry.progress = 0.0;
    }

    // Update progress
    if entry.progress < 1.0 {
      entry.progress = (entry.progress + dt * speed).min(1.0);
    }

    Self::interpolate(entry)
  }

  fn interpolate(entry: &AnimEntry) -> f32 {
    if entry.progress >= 1.0 {
      return entry.target;
    }

    let eased = Easing::InOutElastic.y(entry.progress);
    entry.start + (entry.target - entry.start) * eased
  }

  /// Set position directly (no animation).
  pub fn set_immediate(&mut self, key: &str, pos: f32) {
    self.entries.insert(
      key.to_string(),
      AnimEntry {
        start: pos,
        target: pos,
        progress: 1.0,
      },
    );
  }
}
