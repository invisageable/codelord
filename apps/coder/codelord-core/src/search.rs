//! Search functionality for the IDE.
//!
//! This module provides search state and resources using pure ECS principles.

pub mod engine;
pub mod resources;

pub use engine::{perform_search, perform_search_str, validate_regex};
pub use resources::{SearchOption, SearchState};

/// Insert search state resource. Search systems are currently owned by
/// the `panel` feature (find/next/previous/toggle).
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(SearchState::default());
}
