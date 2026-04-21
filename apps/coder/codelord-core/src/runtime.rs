//! Tokio runtime handle wrapper for World resource access.

use bevy_ecs::resource::Resource;

use std::ops::Deref;

/// Wrapper for tokio runtime handle, stored as World resource.
///
/// This allows systems and UI code to access the tokio runtime
/// for spawning async/blocking tasks.
#[derive(Clone, Resource)]
pub struct RuntimeHandle(pub tokio::runtime::Handle);

impl RuntimeHandle {
  /// Creates a new RuntimeHandle from a tokio Handle.
  pub fn new(handle: tokio::runtime::Handle) -> Self {
    Self(handle)
  }
}

impl Deref for RuntimeHandle {
  type Target = tokio::runtime::Handle;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

/// Install a [`RuntimeHandle`] wrapping the given tokio `Handle` into
/// the world.
pub fn install(
  world: &mut crate::ecs::world::World,
  handle: tokio::runtime::Handle,
) {
  world.insert_resource(RuntimeHandle::new(handle));
}
