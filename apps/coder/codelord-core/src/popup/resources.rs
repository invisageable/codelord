use bevy_ecs::entity::Entity;
use bevy_ecs::message::Message;
use bevy_ecs::resource::Resource;

/// Command to control popup visibility.
#[derive(Message, Debug, Clone)]
pub struct PopupCommand {
  pub action: PopupAction,
}

/// Actions that can be performed on popups.
#[derive(Debug, Clone)]
pub enum PopupAction {
  /// Show a popup at the given anchor rect [x, y, width, height].
  Show {
    entity: Entity,
    anchor_rect: [f32; 4],
  },
  /// Hide a specific popup.
  Hide(Entity),
  /// Toggle a popup at the given anchor rect.
  Toggle {
    entity: Entity,
    anchor_rect: [f32; 4],
  },
  /// Hide all popups.
  HideAll,
}

/// Resource tracking popup state.
#[derive(Resource, Debug, Default)]
pub struct PopupResource {
  /// Currently active popup entity.
  pub active_popup: Option<Entity>,
  /// Settings popup entity.
  pub settings_popup: Option<Entity>,
  /// Explorer context menu popup entity.
  pub explorer_context_popup: Option<Entity>,
  /// Tab context menu popup entity.
  pub tab_context_popup: Option<Entity>,
  /// SQLite export popup entity.
  pub sqlite_export_popup: Option<Entity>,
}

impl PopupResource {
  pub fn new() -> Self {
    Self::default()
  }
}
