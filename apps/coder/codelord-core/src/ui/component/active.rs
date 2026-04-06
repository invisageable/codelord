use bevy_ecs::component::Component;

/// Marker: this entity is currently active/selected.
/// Presence = active, absence = inactive.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Active;
