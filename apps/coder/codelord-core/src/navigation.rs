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

/// Spawn the explorer right-click context menu popup and register its
/// entity in [`PopupResource`].
pub fn spawn_context_popup(world: &mut crate::ecs::world::World) {
  use crate::popup::components::{
    MenuItem, Popup, PopupContent, PopupPosition,
  };
  use crate::popup::resources::PopupResource;

  let menu = PopupContent::Menu(vec![
    MenuItem::new("new_file", "New File"),
    MenuItem::new("new_folder", "New Folder").with_separator(),
    MenuItem::new("add_folder_to_workspace", "Add Folder to Workspace"),
    MenuItem::new("remove_from_workspace", "Remove from Workspace")
      .with_separator(),
    MenuItem::new("cut", "Cut"),
    MenuItem::new("copy", "Copy"),
    MenuItem::new("paste", "Paste").with_separator(),
    MenuItem::new("copy_path", "Copy Path"),
    MenuItem::new("copy_relative_path", "Copy Relative Path"),
    MenuItem::new("reveal_in_finder", "Reveal in Finder").with_separator(),
    MenuItem::new("rename", "Rename"),
    MenuItem::new("delete", "Delete"),
  ]);

  let entity = world
    .spawn(Popup::new(menu).with_position(PopupPosition::Cursor))
    .id();

  if let Some(mut popup_res) = world.get_resource_mut::<PopupResource>() {
    popup_res.explorer_context_popup = Some(entity);
  }
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
