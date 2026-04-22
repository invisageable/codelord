//! Codeshow - presenter slider for markdown-based presentations.

pub mod events;
pub mod resources;

pub use events::{
  LoadPresentationDirectory, LoadPresentationFile, NavigateSlide,
  SlideDirection,
};
pub use resources::{
  CodeshowState, PendingPresentationDirectory, PendingPresentationFile,
  SlideTransition,
};

/// Insert codeshow (presenter) state resource.
pub fn install(world: &mut crate::ecs::world::World) {
  world.insert_resource(CodeshowState::default());
}

/// Per-frame codeshow work that needs exclusive world access (so it
/// can't live in a system): drain pending file/directory dialog
/// results, process `NavigateSlide` messages, and advance the slide
/// transition animation by `delta` seconds.
pub fn poll_pending(world: &mut crate::ecs::world::World, delta: f32) {
  use crate::animation::resources::ContinuousAnimations;
  use crate::ecs::entity::Entity;

  // Pending file dialog.
  let file_result = world
    .get_resource::<PendingPresentationFile>()
    .and_then(|pending| pending.0.try_recv().ok());

  if let Some(result) = file_result {
    if let Some(path) = result
      && let Some(mut state) = world.get_resource_mut::<CodeshowState>()
    {
      let path_str = path.display().to_string();

      if let Err(e) = state.load_file(path) {
        log::error!("[Codeshow] Failed to load presentation file: {e}");
      } else {
        log::info!("[Codeshow] Loaded presentation: {path_str}");
      }
    }

    world.remove_resource::<PendingPresentationFile>();
  }

  // Pending directory dialog.
  let dir_result = world
    .get_resource::<PendingPresentationDirectory>()
    .and_then(|pending| pending.0.try_recv().ok());

  if let Some(result) = dir_result {
    if let Some(path) = result
      && let Some(mut state) = world.get_resource_mut::<CodeshowState>()
    {
      let path_str = path.display().to_string();

      if let Err(e) = state.load_directory(path) {
        log::error!("[Codeshow] Failed to load presentation dir: {e}");
      } else {
        log::info!("[Codeshow] Loaded presentation dir: {path_str}");
      }
    }

    world.remove_resource::<PendingPresentationDirectory>();
  }

  // NavigateSlide messages.
  let nav_messages: Vec<_> = world
    .query_filtered::<(Entity, &NavigateSlide), ()>()
    .iter(world)
    .map(|(e, msg)| (e, msg.direction))
    .collect();

  for (entity, direction) in nav_messages {
    if let Some(mut state) = world.get_resource_mut::<CodeshowState>() {
      match direction {
        SlideDirection::Next => state.next(),
        SlideDirection::Previous => state.previous(),
        SlideDirection::First => state.first(),
        SlideDirection::Last => state.last(),
      }
    }

    world.despawn(entity);
  }

  // Transition animation tick.
  if let Some(mut state) = world.get_resource_mut::<CodeshowState>()
    && state.is_animating()
  {
    state.update_transition(delta);

    if let Some(mut cont) = world.get_resource_mut::<ContinuousAnimations>() {
      cont.set_presenter_active();
    }
  }
}
