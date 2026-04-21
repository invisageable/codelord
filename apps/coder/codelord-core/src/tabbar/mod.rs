pub mod components;
pub mod resources;
pub mod systems;

pub use components::{
  EditorTab, PlaygroundTab, SonarAnimation, Tab, TabMarker,
};
pub use resources::{
  PanelSnapshot, TabContextTarget, TabOrderCounter, UnsavedChangesDialog,
  UnsavedChangesResponse, ZoomSource, ZoomState, ZoomTransition,
};

/// Insert tabbar resources.
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(TabOrderCounter::default());
  world.insert_resource(ZoomState::default());
  world.insert_resource(TabContextTarget::default());
  world.insert_resource(UnsavedChangesDialog::default());
}

/// Register tabbar systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::close_editor_tab_system,
    systems::close_terminal_tab_system,
    systems::close_playground_tab_system,
    systems::close_all_editor_tabs_system,
    systems::close_other_editor_tabs_system,
    systems::close_tabs_to_right_editor_system,
    systems::navigate_prev_editor_tab_system,
    systems::navigate_next_editor_tab_system,
    systems::navigate_prev_terminal_tab_system,
    systems::navigate_next_terminal_tab_system,
    systems::zoom_toggle_system,
    systems::zoom_animation_system,
  ));
}
