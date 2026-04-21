pub mod components;
pub mod resources;
pub mod systems;

pub use resources::PlaygroundMode;

/// Insert explorer / workspace-navigation resources.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::navigation::resources::{
    ActiveWorkspaceRoot, BreadcrumbData, ExplorerContextTarget,
    ExplorerEditingState, ExplorerItemsCounter, ExplorerState, FileClipboard,
    IndentationLinesState,
  };

  world.insert_resource(ExplorerItemsCounter::default());
  world.insert_resource(ExplorerState::default());
  world.insert_resource(ActiveWorkspaceRoot::default());
  world.insert_resource(BreadcrumbData::default());
  world.insert_resource(IndentationLinesState::default());
  world.insert_resource(ExplorerContextTarget::default());
  world.insert_resource(ExplorerEditingState::default());
  world.insert_resource(FileClipboard::default());
}

/// Register navigation / explorer systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::poll_folder_dialog_system,
    systems::folder_selected_system,
    systems::scan_directory_system,
    systems::expand_folder_system,
    systems::collapse_folder_system,
    systems::update_breadcrumbs_system,
    systems::create_file_system,
    systems::create_folder_system,
    systems::rename_system,
    systems::delete_system,
    systems::paste_system,
    systems::add_folder_to_workspace_dialog_system,
    systems::poll_workspace_folder_dialog_system,
    systems::add_root_system,
    systems::remove_root_system,
    systems::refresh_explorer_system,
    systems::collapse_all_folders_system,
    systems::toggle_hidden_files_system,
    systems::sync_explorer_selection_system,
  ));
}
