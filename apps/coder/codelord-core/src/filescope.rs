pub mod components;
pub mod resources;
pub mod systems;

/// Insert filescope state + matcher into the world.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::filescope::resources::{FilescopeMatcher, FilescopeState};

  world.insert_resource(FilescopeState::default());
  world.insert_resource(FilescopeMatcher::new());
}

/// Register filescope systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::filescope_populate_system,
    systems::filescope_tick_system,
  ));
}

/// Apply a filescope response: spawn an `OpenFileRequest` for a
/// selection, or close the picker. Called once per frame from the app
/// shell with whatever the UI layer produced.
pub fn apply_response(
  world: &mut crate::ecs::world::World,
  response: crate::filescope::resources::FilescopeResponse,
) {
  use crate::events::OpenFileRequest;
  use crate::filescope::resources::{FilescopeResponse, FilescopeState};

  match response {
    FilescopeResponse::Select(path, _action) => {
      world.spawn(OpenFileRequest::new(path));
      world.resource_mut::<FilescopeState>().hide();
    }
    FilescopeResponse::Close => {
      world.resource_mut::<FilescopeState>().hide();
    }
    FilescopeResponse::None => {}
  }
}
