//! Hover state component

use bevy_ecs::component::Component;

/// Marker: this entity is being hovered by the mouse.
/// Presence = hovered, absence = not hovered.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hovered;
