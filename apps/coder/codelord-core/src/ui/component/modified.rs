//! Modified state component

use bevy_ecs::component::Component;

/// Marker: this entity has unsaved changes.
/// Presence = modified, absence = clean.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modified;
