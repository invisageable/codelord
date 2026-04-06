use super::components::XmbAction;
use super::resources::{XmbCommand, XmbResource};
use crate::page::components::Page;
use crate::page::resources::PageSwitchCommand;
use crate::theme::resources::{ThemeAction, ThemeCommand};

use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;

/// System to process XMB navigation commands.
pub fn xmb_command_system(
  mut commands: MessageReader<XmbCommand>,
  mut xmb: ResMut<XmbResource>,
) {
  for command in commands.read() {
    let action = xmb.handle_navigation(command.navigation);
    if action.is_some() {
      xmb.pending_action = action;
    }
  }
}

/// System to process pending XMB actions.
pub fn xmb_action_system(world: &mut World) {
  let pending_action = world
    .get_resource_mut::<XmbResource>()
    .and_then(|mut xmb| xmb.pending_action.take());

  if let Some(action) = pending_action {
    match action {
      XmbAction::SwitchToPage(page) => {
        world.write_message(PageSwitchCommand { page });
      }
      XmbAction::OpenSettings => {
        world.write_message(PageSwitchCommand {
          page: Page::Settings,
        });
      }
      XmbAction::OpenThemeSelector => {
        world.write_message(ThemeCommand {
          action: ThemeAction::Toggle,
        });
      }
      XmbAction::OpenFolder => {
        // Switch to editor page (folder dialog handled at component level)
        world.write_message(PageSwitchCommand { page: Page::Editor });
      }
      XmbAction::NewFile => {
        // Switch to editor for new file
        world.write_message(PageSwitchCommand { page: Page::Editor });
      }
      XmbAction::OpenFile => {
        // TODO: Open file dialog
        world.write_message(PageSwitchCommand { page: Page::Editor });
      }
      XmbAction::OpenRecentFile(_path) => {
        // TODO: Open the specific file
        world.write_message(PageSwitchCommand { page: Page::Editor });
      }
      XmbAction::Exit => {
        // TODO: Handle exit
      }
    }
  }
}
