pub mod components;
pub mod resources;
pub mod systems;

/// Insert git blame / branch resources.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::git::resources::{
    GitBlameSettings, GitBranchState, PendingBlameRequests,
    PendingBranchRequests,
  };

  world.insert_resource(GitBlameSettings::default());
  world.insert_resource(PendingBlameRequests::default());
  world.insert_resource(GitBranchState::default());
  world.insert_resource(PendingBranchRequests::default());
}

/// Register git systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::sync_blame_settings_system,
    systems::toggle_blame_system,
    systems::fetch_blame_system,
    systems::poll_blame_results_system,
    systems::invalidate_blame_on_edit_system,
    systems::detect_branch_system,
    systems::poll_branch_results_system,
    systems::check_dirty_status_system,
    systems::poll_status_results_system,
  ));
}
