//! Voice visualizer component for the voice control system.

use crate::components::waveform;

use codelord_core::ecs::world::World;
use codelord_core::voice::resources::{VisualizerStatus, VoiceResource};

use eframe::egui;

/// Render the voice visualizer in the titlebar.
///
/// This shows different visualizations based on the voice control state:
///
/// - [`VisualizerStatus::Listening`] — blue waveform with real-time audio
///   amplitude.
/// - [`VisualizerStatus::Processing`] — animated progress bar while
///   transcribing.
/// - [`VisualizerStatus::Speaking`] — gold frequency bars (for future TTS).
pub fn show(ui: &mut egui::Ui, world: &World) {
  if let Some(voice) = world.get_resource::<VoiceResource>() {
    match voice.visualizer_status {
      VisualizerStatus::Listening => waveform::show_voice(ui, &voice.waveform),
      VisualizerStatus::Processing => {}
      VisualizerStatus::Speaking => {}
      VisualizerStatus::Success => {}
      VisualizerStatus::Error => {}
      VisualizerStatus::Idle => {}
    }
  }
}
