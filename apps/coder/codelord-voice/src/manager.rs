//! Voice manager — orchestrates the voice control system.

use crate::dispatcher::VoiceDispatcher;
use crate::error::VoiceResult;
use crate::input::VoiceInput;
use crate::transcriber;
use crate::transcriber::VoiceTranscriber;
use crate::visualizer::{VisualizerStatus, VoiceVisualizerState};

use codelord_protocol::voice::model::VoiceAction;
use codelord_sdk::Sdk;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Voice manager — orchestrates voice control.
pub struct VoiceManager {
  voice_input: VoiceInput,
  voice_dispatcher: VoiceDispatcher,
  transcriber: Option<VoiceTranscriber>,
  pub visualizer_state: VoiceVisualizerState,
  is_listening: Arc<AtomicBool>,
  transcriber_return_rx: flume::Receiver<VoiceTranscriber>,
  transcriber_return_tx: flume::Sender<VoiceTranscriber>,
}

impl VoiceManager {
  /// Creates a new voice manager.
  ///
  /// Takes a shared `VoiceVisualizerState` that will be updated during voice
  /// operations. This allows the UI to read state even if VoiceManager
  /// initialization fails later.
  pub fn new(
    action_sender: flume::Sender<VoiceAction>,
    model_path: Option<PathBuf>,
    runtime_handle: tokio::runtime::Handle,
    sdk: Arc<Sdk>,
    visualizer_state: VoiceVisualizerState,
  ) -> VoiceResult<Self> {
    let transcriber = model_path
      .or_else(|| {
        let path = transcriber::model_path();
        if path.exists() { Some(path) } else { None }
      })
      .and_then(|path| {
        VoiceTranscriber::new(path)
          .map_err(|e| {
            log::warn!("Failed to load Whisper model: {e}");
            e
          })
          .ok()
      });

    if transcriber.is_none() {
      log::warn!("Voice control disabled: Whisper model not found");
      log::info!(
        "To enable voice control, download the model to: {}",
        transcriber::model_path().display()
      );
    }

    let (transcriber_return_tx, transcriber_return_rx) = flume::unbounded();

    Ok(Self {
      voice_input: VoiceInput::new(visualizer_state.clone())?,
      voice_dispatcher: VoiceDispatcher::new(
        action_sender,
        runtime_handle,
        visualizer_state.clone(),
        sdk,
      ),
      transcriber,
      visualizer_state: visualizer_state.clone(),
      is_listening: Arc::new(AtomicBool::new(false)),
      transcriber_return_rx,
      transcriber_return_tx,
    })
  }

  /// Check if voice control is available.
  pub fn is_available(&self) -> bool {
    self.transcriber.is_some()
  }

  /// Load the transcriber if not already loaded.
  /// Call this after the model has been downloaded.
  pub fn load_transcriber(&mut self) -> bool {
    if self.transcriber.is_some() {
      return true;
    }

    let path = transcriber::model_path();
    if !path.exists() {
      log::warn!("[Voice] Cannot load transcriber: model not found");
      return false;
    }

    match VoiceTranscriber::new(path) {
      Ok(t) => {
        log::info!("[Voice] Transcriber loaded successfully");
        self.transcriber = Some(t);
        true
      }
      Err(e) => {
        log::error!("[Voice] Failed to load transcriber: {e}");
        false
      }
    }
  }

  /// Start listening for voice input.
  ///
  /// Always starts audio recording (for visualizer waveform), even if
  /// transcription isn't available.
  pub fn start_listening(&mut self) -> VoiceResult<()> {
    self.is_listening.store(true, Ordering::SeqCst);

    self
      .visualizer_state
      .set_status(VisualizerStatus::Listening);

    self.voice_input.start_recording()?;

    if !self.is_available() {
      log::warn!("Voice control: Recording but transcription unavailable");
    } else {
      log::info!("Voice control: Started listening");
    }
    Ok(())
  }

  /// Stop listening and process the audio.
  pub fn stop_listening(&mut self) {
    self.is_listening.store(false, Ordering::SeqCst);

    let audio = match self.voice_input.stop_recording() {
      Ok(audio) => audio,
      Err(e) => {
        log::error!("Failed to stop recording: {e}");
        self.visualizer_state.set_status(VisualizerStatus::Error);

        return;
      }
    };

    // If no transcriber, just return to idle
    if !self.is_available() {
      log::warn!("Voice control: No transcriber, returning to idle");
      self.visualizer_state.set_status(VisualizerStatus::Idle);

      return;
    }

    self
      .visualizer_state
      .set_status(VisualizerStatus::Processing);

    log::info!("Voice control: Stopped listening, processing audio...");

    let mut transcriber = match self.transcriber.take() {
      Some(t) => t,
      None => {
        log::error!("No transcriber available");
        self.visualizer_state.set_status(VisualizerStatus::Error);

        return;
      }
    };

    let dispatcher = self.voice_dispatcher.clone();
    let visualizer_state = self.visualizer_state.clone();
    let return_tx = self.transcriber_return_tx.clone();

    std::thread::spawn(move || {
      match transcriber.transcribe(&audio) {
        Ok(text) => {
          log::info!("Transcribed: '{text}'");
          dispatcher.dispatch_voice_text(&text);
        }
        Err(e) => {
          log::error!("Transcription failed: {e}");
          visualizer_state.set_status(VisualizerStatus::Error);
          std::thread::sleep(Duration::from_millis(300));
          visualizer_state.set_status(VisualizerStatus::Idle);
        }
      }

      if let Err(e) = return_tx.send(transcriber) {
        log::error!("Failed to return transcriber: {e}");
      }
    });
  }

  /// Check if currently listening.
  pub fn is_listening(&self) -> bool {
    self.is_listening.load(Ordering::SeqCst)
  }

  /// Try to restore the transcriber from background thread.
  pub fn try_restore_transcriber(&mut self) {
    if let Ok(transcriber) = self.transcriber_return_rx.try_recv() {
      log::debug!("Transcriber restored from background thread");

      self.transcriber = Some(transcriber);
    }
  }

  /// Get visualizer status.
  pub fn get_status(&self) -> VisualizerStatus {
    self.visualizer_state.get_status()
  }

  /// Get input waveform data for rendering.
  pub fn get_waveform(&self) -> Vec<f32> {
    self.visualizer_state.get_input_waveform()
  }
}

impl Drop for VoiceManager {
  fn drop(&mut self) {
    self.is_listening.store(false, Ordering::SeqCst);
  }
}
