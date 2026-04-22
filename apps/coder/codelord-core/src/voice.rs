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

/// Mark voice as contributing to continuous animations when the
/// visualizer is not idle (waveform, progress bar, etc.).
pub fn tick_continuous_animation(world: &mut crate::ecs::world::World) {
  use crate::animation::resources::ContinuousAnimations;
  use crate::voice::resources::{VisualizerStatus, VoiceResource};

  let animating = world
    .get_resource::<VoiceResource>()
    .map(|v| !matches!(v.visualizer_status, VisualizerStatus::Idle))
    .unwrap_or(false);

  if animating
    && let Some(mut anim) = world.get_resource_mut::<ContinuousAnimations>()
  {
    anim.set_voice_active();
  }
}

/// Drain the voice-model-download toast flag and, if set, emit a
/// `ToastCommand::info` with a Download action. Called once per frame
/// from the app shell.
pub fn check_model_toast(world: &mut crate::ecs::world::World) {
  use crate::toast::components::ToastAction;
  use crate::toast::resources::ToastCommand;
  use crate::voice::resources::VoiceModelState;

  let should_show = world
    .get_resource::<VoiceModelState>()
    .map(|s| s.show_download_toast)
    .unwrap_or(false);

  if !should_show {
    return;
  }

  if let Some(mut model_state) = world.get_resource_mut::<VoiceModelState>() {
    model_state.dismiss_toast();
  }

  world.write_message(
    ToastCommand::info("Voice model required (~148 MB)").with_actions(vec![
      ToastAction::new("voice_download", "Download").stripe(),
    ]),
  );
}
