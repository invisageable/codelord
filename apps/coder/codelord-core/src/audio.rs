pub mod resources;

/// Insert audio resources and spawn the dedicated audio thread.
///
/// Audio has no ECS systems of its own — all engine work runs on a
/// separate thread owned by `codelord_audio`. The ECS side is a
/// zero-sized [`resources::AudioDispatcher`] handle plus the music
/// player / playlist UI state.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::audio::resources::{AudioDispatcher, MusicPlayerState, Playlist};

  world.insert_resource(AudioDispatcher);
  world.insert_resource(MusicPlayerState::new());
  world.insert_resource(Playlist::new());

  if let Err(e) = codelord_audio::init() {
    log::error!("Failed to initialize audio system: {e}");
  }
}
