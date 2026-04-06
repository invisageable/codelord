//! Focus state component

use bevy_ecs::component::Component;

/// Marker: this entity has keyboard focus.
/// Presence = focused, absence = not focused.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Focused;
