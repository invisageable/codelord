use super::components::{BottomPanelView, LeftPanelView, RightPanelView};

use bevy_ecs::message::Message;
use bevy_ecs::resource::Resource;

/// Command to request panel visibility changes.
///
/// UI components send this message to request panel toggles.
/// The `panel_command_system` handles it and updates panel resources.
#[derive(Message, Debug, Clone, Copy)]
pub struct PanelCommand {
  pub action: PanelAction,
}

/// Actions that can be performed on panels.
#[derive(Debug, Clone, Copy)]
pub enum PanelAction {
  /// Toggle left panel visibility.
  ToggleLeft,
  /// Toggle right panel visibility.
  ToggleRight,
  /// Toggle bottom panel visibility.
  ToggleBottom,
}

/// Resource managing right panel state.
#[derive(Resource, Debug, Default)]
pub struct RightPanelResource {
  pub active_view: RightPanelView,
  pub is_visible: bool,
}

impl RightPanelResource {
  pub fn toggle(&mut self) {
    self.is_visible = !self.is_visible;
  }
}

/// Resource managing bottom panel state.
#[derive(Resource, Debug, Default)]
pub struct BottomPanelResource {
  pub active_view: BottomPanelView,
  pub is_visible: bool,
}

impl BottomPanelResource {
  pub fn toggle(&mut self) {
    self.is_visible = !self.is_visible;
  }
}

/// Resource managing left panel state.
#[derive(Resource, Debug, Default)]
pub struct LeftPanelResource {
  pub active_view: LeftPanelView,
  pub is_visible: bool,
}

impl LeftPanelResource {
  pub fn toggle(&mut self) {
    self.is_visible = !self.is_visible;
  }
}
