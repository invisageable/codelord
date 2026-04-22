pub mod components;
pub mod resources;
pub mod systems;

/// Insert text-editor resources.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::text_editor::resources::IndentGuidesSettings;

  world.insert_resource(IndentGuidesSettings::default());
}

/// Register text-editor systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::open_file_system,
    systems::new_editor_tab_system,
    systems::activate_tab_system,
    systems::insert_text_system,
    systems::delete_text_system,
    systems::move_cursor_system,
    systems::set_cursor_system,
    systems::save_file_system,
    systems::save_as_dialog_system,
    systems::poll_save_file_dialog_system,
    systems::toggle_fold_system,
  ));
}
