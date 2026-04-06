//! Animation systems that update all animated components

use super::components::{Animatable, DeltaTime};
use super::interpolate::Interpolate;

use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Commands, Query, Res};

/// Generic system to update all animations of a specific type
///
/// This is a helper function - you need to instantiate it for each type you
/// want to animate.
///
/// # Example
/// ```ignore
/// // In your schedule:
/// schedule.add_systems(update_animations::<f32>);
/// schedule.add_systems(update_animations::<Color>);
/// ```
pub fn update_animations<T>(
  delta: Res<DeltaTime>,
  mut query: Query<&mut Animatable<T>>,
) where
  T: Interpolate + Send + Sync + 'static,
{
  for mut anim in query.iter_mut() {
    anim.update(delta.delta());
  }
}

/// System to clean up completed animations
///
/// Removes Animatable component from entities after animation completes.
/// This keeps the World clean and prevents unnecessary updates.
pub fn cleanup_completed_animations<T>(
  mut commands: Commands,
  query: Query<(Entity, &Animatable<T>)>,
) where
  T: Interpolate + Send + Sync + 'static,
{
  for (entity, anim) in query.iter() {
    if anim.is_complete {
      commands.entity(entity).remove::<Animatable<T>>();
    }
  }
}
