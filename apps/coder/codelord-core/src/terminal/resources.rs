//! Terminal ECS resources.

use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use rustc_hash::FxHashMap as HashMap;

use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;

/// Unique terminal identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerminalId(pub usize);

/// Counter for generating unique terminal IDs.
#[derive(Resource, Debug, Default)]
pub struct TerminalIdCounter {
  next_id: usize,
}

impl TerminalIdCounter {
  pub fn bump(&mut self) -> TerminalId {
    let id = TerminalId(self.next_id);
    self.next_id += 1;
    id
  }
}

/// Maps terminal entities to their IDs.
#[derive(Resource, Debug, Default)]
pub struct TerminalRegistry {
  pub entity_to_id: HashMap<Entity, TerminalId>,
  pub id_to_entity: HashMap<TerminalId, Entity>,
}

impl TerminalRegistry {
  pub fn register(&mut self, entity: Entity, id: TerminalId) {
    self.entity_to_id.insert(entity, id);
    self.id_to_entity.insert(id, entity);
  }

  pub fn unregister(&mut self, entity: Entity) {
    if let Some(id) = self.entity_to_id.remove(&entity) {
      self.id_to_entity.remove(&id);
    }
  }

  pub fn get_id(&self, entity: Entity) -> Option<TerminalId> {
    self.entity_to_id.get(&entity).copied()
  }

  pub fn get_entity(&self, id: TerminalId) -> Option<Entity> {
    self.id_to_entity.get(&id).copied()
  }
}

/// Configuration for terminal creation.
#[derive(Debug, Clone)]
pub struct TerminalConfig {
  pub rows: u16,
  pub cols: u16,
  pub working_directory: Option<PathBuf>,
  pub shell: Option<String>,
}

impl Default for TerminalConfig {
  fn default() -> Self {
    Self {
      rows: 24,
      cols: 80,
      working_directory: None,
      shell: None,
    }
  }
}

/// Terminal tab order counter (similar to TabOrderCounter).
#[derive(Resource, Debug, Default)]
pub struct TerminalTabOrderCounter {
  next_order: u32,
}

impl TerminalTabOrderCounter {
  pub fn allocate(&mut self) -> u32 {
    let order = self.next_order;
    self.next_order += 1;
    order
  }
}

/// Type-erased terminal bridge storage.
/// Maps terminal entities to their bridges using type erasure.
/// The concrete bridge type (e.g., AlacrittyBridge) lives in
/// codelord-components.
#[derive(Resource, Default)]
pub struct TerminalBridges {
  bridges: HashMap<Entity, Arc<dyn Any + Send + Sync>>,
}

impl TerminalBridges {
  /// Insert a bridge for an entity.
  pub fn insert<T: Any + Send + Sync>(&mut self, entity: Entity, bridge: T) {
    self.bridges.insert(entity, Arc::new(bridge));
  }

  /// Get a bridge for an entity, downcasting to the concrete type.
  pub fn get<T: Any + Send + Sync>(&self, entity: Entity) -> Option<Arc<T>> {
    self.bridges.get(&entity).and_then(|b| {
      // Clone the Arc and try to downcast
      Arc::clone(b).downcast::<T>().ok()
    })
  }

  /// Remove a bridge for an entity.
  pub fn remove(&mut self, entity: Entity) {
    self.bridges.remove(&entity);
  }

  /// Check if a bridge exists for an entity.
  pub fn contains(&self, entity: Entity) -> bool {
    self.bridges.contains_key(&entity)
  }
}
