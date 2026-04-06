use bevy_ecs::entity::Entity;
use bevy_ecs::message::Message;

/// Request to reorder an item.
#[derive(Message, Debug, Clone)]
pub struct ReorderRequest {
  pub source: String,
  pub entity: Entity,
  pub from_index: u32,
  pub to_index: u32,
}
