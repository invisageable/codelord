pub mod animation;
pub mod components;
pub mod messages;
pub mod resources;

pub use animation::DragAnimationState;
pub use components::{DragOrder, Draggable, DropZone};
pub use messages::ReorderRequest;
pub use resources::{DragAxis, DragState};

/// Insert drag-and-drop state resources.
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(DragState::default());
  world.insert_resource(DragAnimationState::default());
}
