pub mod components;
pub mod resources;
pub mod systems;

/// Insert voice-control resources + message queues.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::ecs::message::Messages;
  use crate::voice::resources::{
    VoiceActionEvent, VoiceModelState, VoiceResource, VoiceToggleCommand,
  };

  world.insert_resource(VoiceResource::default());
  world.insert_resource(VoiceModelState::default());
  world.init_resource::<Messages<VoiceToggleCommand>>();
  world.init_resource::<Messages<VoiceActionEvent>>();
}

/// Register voice systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule.add_systems((
    systems::voice_toggle_system,
    systems::voice_animation_system,
    systems::voice_action_system,
  ));
}
