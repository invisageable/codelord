//! Voice control resources and message types.

use super::components::{VoiceAnimation, VoiceState};

use crate::time::current_time_ms;

use bevy_ecs::message::Message;
use bevy_ecs::resource::Resource;

/// Command to toggle voice control mode.
/// Send this message to start/stop voice recording.
#[derive(Message, Debug, Clone, Copy)]
pub struct VoiceToggleCommand;

/// Event indicating voice action was interpreted.
/// Dispatched from the VoiceManager when a command is recognized.
#[derive(Message, Debug, Clone)]
pub struct VoiceActionEvent {
  pub action: String,
  pub payload: Option<String>,
}

pub use codelord_protocol::voice::VisualizerStatus;

/// Resource managing voice control UI state.
/// The actual VoiceManager is stored separately and polled by the coder.
#[derive(Resource)]
pub struct VoiceResource {
  /// Current state (synced from VoiceManager).
  pub state: VoiceState,
  /// Visualizer status for UI rendering (synced from VoiceVisualizerState).
  pub visualizer_status: VisualizerStatus,
  /// Animation state for visual feedback.
  pub animation: VoiceAnimation,
  /// Last action executed (for feedback).
  pub last_action: Option<String>,
  /// Error message if something failed.
  pub error: Option<String>,
  /// Time when current state started (ms since epoch).
  pub state_started_at: u64,
  /// Input waveform data for visualizer.
  pub waveform: Vec<f32>,
  /// Timestamp when Processing state started (ms since epoch).
  pub processing_start_time: u64,
  /// Whether voice control is available (model loaded).
  pub is_available: bool,
}

impl Default for VoiceResource {
  fn default() -> Self {
    Self {
      state: VoiceState::Idle,
      visualizer_status: VisualizerStatus::Idle,
      animation: VoiceAnimation::default(),
      last_action: None,
      error: None,
      state_started_at: current_time_ms(),
      waveform: Vec::new(),
      processing_start_time: 0,
      is_available: false,
    }
  }
}

impl VoiceResource {
  /// Transition to a new state.
  pub fn set_state(&mut self, state: VoiceState) {
    self.state = state;
    self.state_started_at = current_time_ms();
    self.error = None;
  }

  /// Set error and return to idle.
  pub fn set_error(&mut self, error: impl Into<String>) {
    self.error = Some(error.into());
    self.state = VoiceState::Idle;
    self.state_started_at = current_time_ms();
  }

  /// Set visualizer status (synced from VoiceVisualizerState).
  pub fn set_visualizer_status(&mut self, status: VisualizerStatus) {
    if status == VisualizerStatus::Processing
      && self.visualizer_status != VisualizerStatus::Processing
    {
      self.processing_start_time = current_time_ms();
    }
    self.visualizer_status = status;
  }

  /// Update animation based on current state and delta time.
  pub fn update_animation(&mut self, dt: f32) {
    match self.state {
      VoiceState::Idle => {
        self.animation.opacity = (self.animation.opacity - dt * 3.0).max(0.0);
        self.animation.scale = (self.animation.scale - dt * 3.0).max(1.0);
        self.animation.pulse = 0.0;
      }
      VoiceState::Listening => {
        self.animation.opacity = 1.0;
        self.animation.pulse = (self.animation.pulse + dt * 2.0) % 1.0;
        let pulse_scale =
          1.0 + 0.1 * (self.animation.pulse * std::f32::consts::TAU).sin();
        self.animation.scale = pulse_scale;
      }
      VoiceState::Processing => {
        self.animation.opacity = 0.8;
        self.animation.pulse = (self.animation.pulse + dt * 4.0) % 1.0;
        self.animation.scale = 1.0;
      }
      VoiceState::Executing => {
        self.animation.opacity = 1.0;
        self.animation.scale = 1.2;
        self.animation.pulse = 0.0;
      }
    }
  }

  /// Check if we should show voice indicator overlay.
  pub fn should_show_indicator(&self) -> bool {
    self.state.is_active() || self.animation.opacity > 0.01
  }
}

// ============================================================================
// Voice Model State
// ============================================================================

/// Status of the Whisper model for voice transcription.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ModelStatus {
  /// Model status not yet checked.
  #[default]
  Unknown,
  /// Model file not found, needs download.
  Missing,
  /// Model is being downloaded.
  Downloading,
  /// Model is ready to use.
  Ready,
  /// Download or load failed.
  Error,
}

/// Resource tracking voice model download state.
#[derive(Resource, Default)]
pub struct VoiceModelState {
  /// Current model status.
  pub status: ModelStatus,
  /// Download progress (0.0 to 1.0).
  pub download_progress: f32,
  /// Error message if download/load failed.
  pub error: Option<String>,
  /// Whether to show the download toast notification.
  pub show_download_toast: bool,
}

impl VoiceModelState {
  /// Check if model is ready for use.
  pub fn is_ready(&self) -> bool {
    self.status == ModelStatus::Ready
  }

  /// Check if download is in progress.
  pub fn is_downloading(&self) -> bool {
    self.status == ModelStatus::Downloading
  }

  /// Start download.
  pub fn start_download(&mut self) {
    self.status = ModelStatus::Downloading;
    self.download_progress = 0.0;
    self.error = None;
    self.show_download_toast = false;
  }

  /// Update download progress.
  pub fn set_progress(&mut self, progress: f32) {
    self.download_progress = progress.clamp(0.0, 1.0);
  }

  /// Mark download complete.
  pub fn set_ready(&mut self) {
    self.status = ModelStatus::Ready;
    self.download_progress = 1.0;
    self.error = None;
    self.show_download_toast = false;
  }

  /// Set error state.
  pub fn set_error(&mut self, error: impl Into<String>) {
    self.status = ModelStatus::Error;
    self.error = Some(error.into());
    self.show_download_toast = false;
  }

  /// Show download prompt toast.
  pub fn prompt_download(&mut self) {
    self.show_download_toast = true;
  }

  /// Dismiss the download toast.
  pub fn dismiss_toast(&mut self) {
    self.show_download_toast = false;
  }
}
