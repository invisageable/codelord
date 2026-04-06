use bevy_ecs::component::Component;

/// Marker: entity is draggable.
#[derive(Component, Default)]
pub struct Draggable;

/// Marker: entity is a drop zone.
#[derive(Component, Default)]
pub struct DropZone;

/// Order component for reorderable items.
#[derive(Component, Debug, Clone, Copy)]
pub struct DragOrder(pub u32);
