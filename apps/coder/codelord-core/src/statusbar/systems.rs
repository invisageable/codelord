//! Statusbar systems for ECS

use super::resources::LineColumnAnimation;
use crate::animation::components::DeltaTime;
use crate::animation::resources::ActiveAnimations;
use crate::tabbar::components::EditorTab;
use crate::text_editor::components::{Cursor, TextBuffer};
use crate::ui::component::Active;

use bevy_ecs::prelude::*;

/// System to update line/column animation.
///
/// Reads cursor position from active editor tab, updates animation target,
/// and manages ActiveAnimations counter properly.
#[allow(clippy::type_complexity)]
pub fn line_column_animation_system(
  query: Query<(&Cursor, &TextBuffer), (With<EditorTab>, With<Active>)>,
  time: Res<DeltaTime>,
  mut anim: ResMut<LineColumnAnimation>,
  mut active_animations: ResMut<ActiveAnimations>,
) {
  // Get cursor position from active editor tab
  let (target_line, target_col) = query
    .iter()
    .next()
    .map(|(cursor, buffer)| {
      let (line, col) = buffer.char_to_line_col(cursor.position);
      (line + 1, col + 1) // 1-indexed
    })
    .unwrap_or((1, 1));

  // Set target - returns true if animation was started (wasn't already active)
  if anim.set_target(target_line, target_col) {
    active_animations.increment();
  }

  // Update animation - returns true if animation completed
  if anim.update(time.delta()) {
    active_animations.decrement();
  }
}
