pub mod resources;
pub mod systems;

pub use resources::{
  CompilationState, FeedbackState, OutputViewKind, PLAYGROUND_PREVIEW_URL,
  PlaygroundFeedback, PlaygroundHoveredSpan, PlaygroundMetrics,
  PlaygroundOutput, PlaygroundWebviewState, TemplatingTarget,
};
pub use systems::{activate_playground_tab_system, new_playground_tab_system};

/// Insert playground-mode resources.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::playground::resources::{
    PlaygroundFeedback, PlaygroundHoveredSpan, PlaygroundMetrics,
    PlaygroundOutput, PlaygroundWebviewState,
  };

  world.insert_resource(PlaygroundMetrics::default());
  world.insert_resource(PlaygroundFeedback::default());
  world.insert_resource(PlaygroundOutput::default());
  world.insert_resource(PlaygroundHoveredSpan::default());
  world.insert_resource(PlaygroundWebviewState::default());
}

/// Register playground systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule
    .add_systems((new_playground_tab_system, activate_playground_tab_system));
}
