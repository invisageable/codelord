pub mod components;
pub mod resources;

/// Insert instructions resource (for empty editor state hints).
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(resources::InstructionsResource::default());
}
