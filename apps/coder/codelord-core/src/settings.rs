pub mod resources;
pub mod system_info;

/// Insert settings resources.
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(resources::SettingsResource::default());
}

/// Spawn the settings dropdown popup (About / Settings / Check for
/// Updates) and register its entity in [`PopupResource`].
pub fn spawn_popup(world: &mut crate::ecs::world::World) {
  use crate::popup::components::{MenuItem, Popup, PopupContent};
  use crate::popup::resources::PopupResource;

  let menu = PopupContent::Menu(vec![
    MenuItem::new("about", "About Codelord"),
    MenuItem::new("settings", "Settings").with_shortcut("Cmd+,"),
    MenuItem::new("check_updates", "Check for Updates"),
  ]);

  let entity = world.spawn(Popup::new(menu)).id();

  if let Some(mut popup_res) = world.get_resource_mut::<PopupResource>() {
    popup_res.settings_popup = Some(entity);
  }
}
