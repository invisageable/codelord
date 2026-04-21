pub mod resources;
pub mod system_info;

/// Insert settings resources.
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(resources::SettingsResource::default());
}
