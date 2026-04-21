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

/// Spawn the tab right-click context menu popup and register its
/// entity in [`crate::popup::resources::PopupResource`].
pub fn spawn_context_popup(world: &mut crate::ecs::world::World) {
  use crate::popup::components::{
    MenuItem, Popup, PopupContent, PopupPosition,
  };
  use crate::popup::resources::PopupResource;

  let menu = PopupContent::Menu(vec![
    MenuItem::new("close_tab", "Close"),
    MenuItem::new("close_others", "Close Others"),
    MenuItem::new("close_to_right", "Close to the Right").with_separator(),
    MenuItem::new("close_all", "Close All"),
  ]);

  let entity = world
    .spawn(Popup::new(menu).with_position(PopupPosition::Cursor))
    .id();

  if let Some(mut popup_res) = world.get_resource_mut::<PopupResource>() {
    popup_res.tab_context_popup = Some(entity);
  }
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
