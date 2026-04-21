pub mod resources;

/// Insert about page resources.
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(resources::AboutResource::default());
}
