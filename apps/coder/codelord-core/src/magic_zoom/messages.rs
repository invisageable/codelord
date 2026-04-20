use bevy_ecs::message::Message;

/// Command emitted by the input layer when the hotkey transitions between
/// pressed and released. Systems consume it to retarget the zoom curve.
#[derive(Message, Debug, Clone, Copy)]
pub struct MagicZoomCommand {
  pub engage: bool,
}
