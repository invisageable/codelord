//! Selected state component

use bevy_ecs::component::Component;

/// Marker: this entity is selected.
/// Presence = selected, absence = not selected.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selected;
