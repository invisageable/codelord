//! Waveform visualization for audio.
//!
//! Provides a lock-free waveform visualizer that can be shared between the
//! audio thread (writer) and UI thread (reader) using atomic operations.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// Number of samples to store for waveform visualization.
pub const WAVEFORM_SAMPLES: usize = 128;

/// Shared waveform data that can be read by UI without blocking audio.
///
/// Uses a ring buffer with atomic index for lock-free updates.
/// The audio thread writes samples, UI thread reads for visualization.
#[derive(Clone)]
pub struct WaveformVisualizer {
  /// Audio sample buffer (circular buffer of raw PCM samples -1.0 to 1.0).
  /// Stored as u32 to allow atomic access (f32 bits reinterpreted).
  samples: Arc<[AtomicU32; WAVEFORM_SAMPLES]>,
  /// Current write position in the ring buffer.
  write_pos: Arc<AtomicU64>,
  /// Whether the visualizer is actively receiving samples.
  active: Arc<AtomicBool>,
}

impl WaveformVisualizer {
  /// Create a new waveform visualizer.
  pub fn new() -> Self {
    let samples: [AtomicU32; WAVEFORM_SAMPLES] =
      std::array::from_fn(|_| AtomicU32::new(0));

    Self {
      samples: Arc::new(samples),
      write_pos: Arc::new(AtomicU64::new(0)),
      active: Arc::new(AtomicBool::new(false)),
    }
  }

  /// Push a sample to the waveform buffer.
  ///
  /// Called from audio thread - must be non-blocking.
  pub fn push_sample(&self, amplitude: f32) {
    if !self.active.load(Ordering::Relaxed) {
      return;
    }

    // Store raw PCM sample (-1.0 to 1.0).
    // Convert f32 to u32 bits for atomic storage.
    let bits = amplitude.to_bits();

    // Get current write position.
    let pos = self.write_pos.load(Ordering::Relaxed) as usize;

    // Write to circular buffer.
    self.samples[pos % WAVEFORM_SAMPLES].store(bits, Ordering::Relaxed);

    // Advance write position.
    self
      .write_pos
      .store(((pos + 1) % WAVEFORM_SAMPLES) as u64, Ordering::Relaxed);
  }

  /// Get a snapshot of the current waveform samples.
  ///
  /// Called from UI thread - returns a copy of the current buffer.
  pub fn read_samples(&self) -> Vec<f32> {
    let mut result = Vec::with_capacity(WAVEFORM_SAMPLES);

    // Read samples in display order (oldest to newest).
    let write_pos = self.write_pos.load(Ordering::Relaxed) as usize;

    for i in 0..WAVEFORM_SAMPLES {
      let idx = (write_pos + i) % WAVEFORM_SAMPLES;
      let bits = self.samples[idx].load(Ordering::Relaxed);
      let amplitude = f32::from_bits(bits);

      result.push(amplitude);
    }

    result
  }

  /// Get normalized samples as f32 values in range [-1.0, 1.0].
  ///
  /// Alias for read_samples() for API compatibility.
  pub fn get_normalized_samples(&self) -> Vec<f32> {
    self.read_samples()
  }

  /// Set whether the visualizer is active.
  pub fn set_active(&self, active: bool) {
    self.active.store(active, Ordering::Relaxed);
  }

  /// Check if the visualizer is active.
  pub fn is_active(&self) -> bool {
    self.active.load(Ordering::Relaxed)
  }

  /// Clear all samples (reset to zero).
  pub fn clear(&self) {
    for sample in self.samples.iter() {
      sample.store(0, Ordering::Relaxed);
    }

    self.write_pos.store(0, Ordering::Relaxed);
  }
}

impl Default for WaveformVisualizer {
  fn default() -> Self {
    Self::new()
  }
}

/// Process audio samples and update visualizer.
///
/// This should be called periodically from the audio thread with raw PCM
/// samples. Pushes each raw sample directly for FFT analysis.
pub fn process_audio_chunk(visualizer: &WaveformVisualizer, samples: &[f32]) {
  if samples.is_empty() {
    return;
  }

  // Push each raw sample for FFT analysis.
  for &sample in samples {
    visualizer.push_sample(sample);
  }
}
