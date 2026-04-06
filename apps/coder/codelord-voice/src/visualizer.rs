//! Voice visualizer state for UI feedback.
//!
//! Shared state between the audio thread and the UI thread
//! for real-time visual feedback.

use bevy_ecs::resource::Resource;

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Shared state between audio thread and UI thread.
#[derive(Clone, Default, Resource)]
pub struct VoiceVisualizerState {
  /// Current visualizer status.
  pub status: Arc<Mutex<VisualizerStatus>>,
  /// Small buffer of recent audio amplitude values for waveform.
  /// Circular buffer capped at 256 values.
  pub input_waveform_data: Arc<Mutex<Vec<f32>>>,
  /// Output amplitude (from TTS if applicable).
  pub output_amplitude: Arc<AtomicU32>,
  /// Timestamp when Processing state started (ms since epoch).
  pub processing_start_time: Arc<AtomicU64>,
}

impl VoiceVisualizerState {
  /// Creates a new visualizer state.
  pub fn new() -> Self {
    Self {
      status: Arc::new(Mutex::new(VisualizerStatus::Idle)),
      input_waveform_data: Arc::new(Mutex::new(Vec::with_capacity(256))),
      output_amplitude: Arc::new(AtomicU32::new(0)),
      processing_start_time: Arc::new(AtomicU64::new(0)),
    }
  }

  /// Get current output amplitude (thread-safe).
  #[inline(always)]
  pub fn get_output_amplitude(&self) -> f32 {
    f32::from_bits(self.output_amplitude.load(Ordering::Relaxed))
  }

  /// Set current output amplitude (thread-safe).
  #[inline(always)]
  pub fn set_output_amplitude(&self, amplitude: f32) {
    self
      .output_amplitude
      .store(amplitude.to_bits(), Ordering::Relaxed);
  }

  /// Get current status.
  pub fn get_status(&self) -> VisualizerStatus {
    *self.status.lock().unwrap()
  }

  /// Set current status.
  pub fn set_status(&self, new_status: VisualizerStatus) {
    if new_status == VisualizerStatus::Processing {
      let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

      self.processing_start_time.store(now, Ordering::Relaxed);
    }

    *self.status.lock().unwrap() = new_status;
  }

  /// Get timestamp when Processing state started.
  pub fn get_processing_start_time(&self) -> u64 {
    self.processing_start_time.load(Ordering::Relaxed)
  }

  /// Push raw audio samples to input waveform buffer (for FFT).
  pub fn push_input_samples(&self, samples: &[f32]) {
    let mut waveform = self.input_waveform_data.lock().unwrap();

    waveform.extend_from_slice(samples);

    const FFT_SIZE: usize = 512;

    if waveform.len() > FFT_SIZE {
      let excess = waveform.len() - FFT_SIZE;

      waveform.drain(0..excess);
    }
  }

  /// Clear input waveform buffer.
  pub fn clear_input_waveform(&self) {
    self.input_waveform_data.lock().unwrap().clear();
  }

  /// Get input waveform data for rendering.
  pub fn get_input_waveform(&self) -> Vec<f32> {
    self.input_waveform_data.lock().unwrap().clone()
  }
}

/// Current state of the voice visualizer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum VisualizerStatus {
  /// Not recording or playing.
  #[default]
  Idle,
  /// User speaking (recording).
  Listening,
  /// Transcribing/thinking.
  Processing,
  /// AI responding.
  Speaking,
  /// Brief success feedback.
  Success,
  /// Brief error feedback.
  Error,
}
