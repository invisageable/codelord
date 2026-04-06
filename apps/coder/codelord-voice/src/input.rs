//! Voice input — audio capture and buffering.
//!
//! Captures audio from the microphone using cpal.

use crate::error::{VoiceError, VoiceResult};
use crate::visualizer::VoiceVisualizerState;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Voice input system.
///
/// Captures audio from the microphone and buffers it for transcription.
pub struct VoiceInput {
  audio_buffer: Arc<Mutex<Vec<f32>>>,
  is_recording: Arc<AtomicBool>,
  #[allow(dead_code)]
  stream: cpal::Stream,
  visualizer_state: VoiceVisualizerState,
}

impl VoiceInput {
  /// Creates a new voice input system.
  pub fn new(visualizer_state: VoiceVisualizerState) -> VoiceResult<Self> {
    let audio_buffer = Arc::new(Mutex::new(Vec::new()));
    let is_recording = Arc::new(AtomicBool::new(false));

    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| {
      VoiceError::AudioDeviceError("No input device available".to_string())
    })?;

    let config = device
      .default_input_config()
      .map_err(|e| VoiceError::AudioDeviceError(e.to_string()))?;

    log::info!(
      "Initializing audio capture: {:?} @ {} Hz",
      config.sample_format(),
      config.sample_rate()
    );

    let audio_buffer_clone = Arc::clone(&audio_buffer);
    let is_recording_clone = Arc::clone(&is_recording);
    let visualizer_state_clone = visualizer_state.clone();

    let stream = match config.sample_format() {
      cpal::SampleFormat::F32 => Self::build_stream::<f32>(
        &device,
        &config.into(),
        audio_buffer_clone,
        is_recording_clone,
        visualizer_state_clone,
      ),
      cpal::SampleFormat::I16 => Self::build_stream::<i16>(
        &device,
        &config.into(),
        audio_buffer_clone,
        is_recording_clone,
        visualizer_state_clone,
      ),
      cpal::SampleFormat::U16 => Self::build_stream::<u16>(
        &device,
        &config.into(),
        audio_buffer_clone,
        is_recording_clone,
        visualizer_state_clone,
      ),
      _ => {
        return Err(VoiceError::AudioDeviceError(
          "Unsupported sample format".to_string(),
        ));
      }
    }?;

    stream
      .play()
      .map_err(|e| VoiceError::StreamError(e.to_string()))?;

    Ok(Self {
      audio_buffer,
      is_recording,
      stream,
      visualizer_state,
    })
  }

  /// Start recording audio from the microphone.
  pub fn start_recording(&mut self) -> VoiceResult<()> {
    self.is_recording.store(true, Ordering::SeqCst);
    self.lock_audio_buffer().clear();
    self.visualizer_state.clear_input_waveform();

    log::info!("Started recording");

    Ok(())
  }

  /// Stop recording and return the captured audio buffer.
  pub fn stop_recording(&mut self) -> VoiceResult<Vec<f32>> {
    self.is_recording.store(false, Ordering::SeqCst);

    let buffer = self.lock_audio_buffer().clone();

    log::info!("Captured {} audio samples", buffer.len());

    Ok(buffer)
  }

  /// Locks the audio buffer, recovering from poison if necessary.
  fn lock_audio_buffer(&self) -> std::sync::MutexGuard<'_, Vec<f32>> {
    self.audio_buffer.lock().unwrap_or_else(|poisoned| {
      log::warn!("Audio buffer mutex was poisoned, recovering");
      poisoned.into_inner()
    })
  }

  fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    visualizer_state: VoiceVisualizerState,
  ) -> VoiceResult<cpal::Stream>
  where
    T: cpal::Sample<Float = f32> + cpal::SizedSample,
  {
    let stream = device
      .build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
          if !is_recording.load(Ordering::SeqCst) {
            return;
          }

          // Lock with poison recovery - audio must continue even after panics
          let mut buffer = audio_buffer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

          let mut float_samples = Vec::with_capacity(data.len());

          for &sample in data.iter() {
            let sample_f32 = sample.to_float_sample();

            buffer.push(sample_f32);
            float_samples.push(sample_f32);
          }

          visualizer_state.push_input_samples(&float_samples);
        },
        |err| log::error!("Audio stream error: {err}"),
        None,
      )
      .map_err(|e| VoiceError::StreamError(e.to_string()))?;

    Ok(stream)
  }
}

impl Drop for VoiceInput {
  fn drop(&mut self) {
    self.is_recording.store(false, Ordering::SeqCst);
  }
}
