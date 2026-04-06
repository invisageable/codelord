//! Theme-related systems for the ECS

use crate::theme::{Theme, ThemeAnimation};

use super::components::*;
use super::resources::*;

use bevy_ecs::message::MessageReader;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::query::Changed;
use bevy_ecs::schedule::SystemSet;
use bevy_ecs::system::{Commands, Query, Res, ResMut};

/// System to detect stale themed entities and mark them for update
///
/// Queries all entities with Themed + ThemeGeneration components,
/// compares their generation with the global theme generation,
/// and triggers updates for stale entities.
///
/// This is a lightweight change detection mechanism - only entities
/// with outdated generations are processed by downstream systems.
pub fn theme_change_detection_system(
  theme: Res<ThemeResource>,
  mut query: Query<(&mut ThemeGeneration, &Themed)>,
) {
  let current_generation = theme.generation();

  for (mut entity_generation, _themed) in query.iter_mut() {
    if entity_generation.is_stale(current_generation) {
      // Mark as updated
      entity_generation.0 = current_generation;

      // In a real implementation, this is where you'd trigger
      // visual updates, recompute styles, etc.
      // For now, just updating the generation is sufficient.
    }
  }
}

/// System to handle theme switching commands
///
/// Listens for theme change messages and updates the global ThemeResource.
/// Also emits ThemeChangedEvent for systems that prefer message-based updates.
pub fn theme_command_system(
  mut theme: ResMut<ThemeResource>,
  mut commands: MessageReader<ThemeCommand>,
  mut events: MessageWriter<ThemeChangedEvent>,
) {
  for command in commands.read() {
    let old_theme = theme.current;

    let new_theme = match command.action {
      ThemeAction::Set(kind) => kind,
      ThemeAction::Toggle => match theme.current {
        ThemeKind::Dark => ThemeKind::Light,
        ThemeKind::Light => ThemeKind::Dark,
        ThemeKind::Custom => ThemeKind::Dark,
      },
    };

    if old_theme != new_theme {
      theme.set_theme(new_theme);

      events.write(ThemeChangedEvent {
        old_theme,
        new_theme: theme.current,
      });
    }
  }
}

/// System to handle theme hot-reload
///
/// When hot_reload is enabled, watches for theme file changes and
/// automatically reloads the theme.
///
/// TODO: Implement file watching and TOML parsing for custom themes
pub fn theme_hot_reload_system(theme: Res<ThemeResource>) {
  if theme.hot_reload {
    // TODO: Implement file watching
    // 1. Check if theme file modified
    // 2. Parse TOML/config file
    // 3. Update theme if valid
    // 4. Log errors if invalid
  }
}

/// System to apply theme overrides to specific entities
///
/// Entities with ThemeOverride component use a different theme
/// than the global one. This system ensures they stay in sync
/// with their override settings.
pub fn theme_overrcodelord_system(
  mut query: Query<
    (&ThemeOverride, &mut ThemeGeneration),
    Changed<ThemeOverride>,
  >,
) {
  for (_override, mut generation) in query.iter_mut() {
    // When override changes, force a visual update
    // by incrementing the entity's generation
    generation.0 = generation.0.wrapping_add(1);
  }
}

/// Helper system to count themed entities (for debugging/monitoring)
pub fn theme_entity_count_system(query: Query<&Themed>) {
  let count = query.iter().count();
  if count > 0 {
    // Could log or expose as metric
    // log::debug!("Themed entities: {}", count);
  }
}

/// System to create theme animation on theme changes
///
/// Listens for ThemeChangedEvent and creates ThemeAnimation resource.
pub fn theme_animation_system(
  mut commands: Commands,
  mut events: MessageReader<ThemeChangedEvent>,
  mut active_animations: ResMut<crate::animation::resources::ActiveAnimations>,
) {
  for event in events.read() {
    // Get the Theme instances for old and new themes
    let from_theme = match event.old_theme {
      ThemeKind::Dark => &Theme::KURO,
      ThemeKind::Light => &Theme::SHIVA,
      ThemeKind::Custom => &Theme::KURO,
    };

    let to_theme = match event.new_theme {
      ThemeKind::Dark => &Theme::KURO,
      ThemeKind::Light => &Theme::SHIVA,
      ThemeKind::Custom => &Theme::KURO,
    };

    // Create animation
    let animation = ThemeAnimation::new(from_theme, to_theme, 0.3);

    commands.insert_resource(animation);
    active_animations.increment();
  }
}

/// System to update theme animation
///
/// Updates the ThemeAnimation resource with delta time.
/// Removes the animation when complete.
pub fn theme_animation_update_system(
  mut commands: Commands,
  animation: Option<ResMut<ThemeAnimation>>,
  time: Res<crate::animation::components::DeltaTime>,
  mut active_animations: ResMut<crate::animation::resources::ActiveAnimations>,
) {
  if let Some(mut anim) = animation {
    anim.update(time.delta());

    if anim.is_complete {
      commands.remove_resource::<ThemeAnimation>();
      active_animations.decrement();
    }
  }
}

// ============================================================================
// System sets for organizing theme systems
// ============================================================================

/// System set for theme update logic
///
/// Run these systems early in the frame, before rendering,
/// to ensure theme changes are applied before UI is drawn.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeSystemSet {
  /// Detect theme changes and update generation counters
  ChangeDetection,
  /// Apply theme changes to entities
  Apply,
  /// Hot reload and file watching
  HotReload,
}
