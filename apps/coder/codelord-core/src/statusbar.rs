pub mod resources;
pub mod systems;

/// Insert statusbar resources.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::statusbar::resources::{LineColumnAnimation, StatusbarResource};

  world.insert_resource(StatusbarResource::new());
  world.insert_resource(LineColumnAnimation::new());
}

/// Register statusbar systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems(systems::line_column_animation_system);
}

/// Spawn the default statusbar icons (Explorer on the left, Voice on
/// the right) and register them in [`resources::StatusbarResource`].
pub fn spawn_default_icons(world: &mut crate::ecs::world::World) {
  use crate::icon::components::{Icon, StatusbarIconBundle};
  use crate::statusbar::resources::StatusbarResource;

  let explorer_btn = world.spawn(StatusbarIconBundle::new(Icon::Explorer)).id();
  let voice_btn = world.spawn(StatusbarIconBundle::new(Icon::Voice)).id();

  if let Some(mut statusbar) = world.get_resource_mut::<StatusbarResource>() {
    statusbar.add_left(explorer_btn);
    statusbar.add_right(voice_btn);
  }
}
