//! Magic zoom systems.

use super::messages::MagicZoomCommand;
use super::resources::MagicZoomState;
use crate::animation::components::DeltaTime;

use bevy_ecs::message::MessageReader;
use bevy_ecs::system::{Res, ResMut};

/// Drains `MagicZoomCommand` messages to retarget the zoom, then advances
/// all eased scalars by frame dt.
pub fn update_magic_zoom_system(
  mut state: ResMut<MagicZoomState>,
  mut commands: MessageReader<MagicZoomCommand>,
  dt: Res<DeltaTime>,
) {
  for cmd in commands.read() {
    state.retarget_zoom(cmd.engage);
  }

  state.tick(dt.delta());
}
