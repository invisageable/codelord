//! Voice control system for IDE.
//!
//! Provides speech-to-text and voice command interpretation
//! following the "Voice as an Action Dispatcher" philosophy.

mod dispatcher;
mod error;
mod input;
mod manager;
mod parser;
pub mod transcriber;
mod visualizer;

pub use dispatcher::VoiceDispatcher;
pub use error::{VoiceError, VoiceResult};
pub use manager::VoiceManager;
pub use parser::VoiceIntentParser;
pub use visualizer::{VisualizerStatus, VoiceVisualizerState};

/// Install a fresh [`VoiceVisualizerState`] resource and return a clone
/// for hand-off to [`VoiceManager`]. The clone keeps the audio-thread
/// side of the visualizer synced with the world resource.
pub fn install_visualizer(
  world: &mut bevy_ecs::world::World,
) -> VoiceVisualizerState {
  let state = VoiceVisualizerState::new();
  world.insert_resource(state.clone());
  state
}
