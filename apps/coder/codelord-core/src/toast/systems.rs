//! Toast system - processes commands and updates animations.

use super::resources::{DismissToastCommand, ToastCommand, ToasterResource};

use crate::time;

use bevy_ecs::message::MessageReader;
use bevy_ecs::system::ResMut;

/// System to process toast commands and add new toasts.
pub fn process_toast_commands(
  mut toaster: ResMut<ToasterResource>,
  mut commands: MessageReader<ToastCommand>,
) {
  for cmd in commands.read() {
    toaster.push(cmd.message.clone(), cmd.status, cmd.actions.clone());
  }
}

/// System to process dismiss commands.
pub fn process_dismiss_commands(
  mut toaster: ResMut<ToasterResource>,
  mut commands: MessageReader<DismissToastCommand>,
) {
  for cmd in commands.read() {
    toaster.dismiss(cmd.0);
  }
}

/// System to update toast animations each frame.
pub fn update_toast_animations(mut toaster: ResMut<ToasterResource>) {
  toaster.update(time::current_time_ms());
}
