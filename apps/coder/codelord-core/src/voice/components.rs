//! Voice control components.

/// Current voice control state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceState {
  /// Not active - waiting for user to trigger voice mode.
  #[default]
  Idle,
  /// Listening for voice input (recording).
  Listening,
  /// Processing voice command (transcription + interpretation).
  Processing,
  /// Executing the interpreted action.
  Executing,
}

impl VoiceState {
  /// Returns true if voice is currently active (not idle).
  pub fn is_active(&self) -> bool {
    !matches!(self, Self::Idle)
  }
}

/// Animation state for voice indicator.
#[derive(Debug, Clone, Copy, Default)]
pub struct VoiceAnimation {
  /// Pulse animation progress (0.0 to 1.0).
  pub pulse: f32,
  /// Opacity (0.0 to 1.0).
  pub opacity: f32,
  /// Scale factor for visual feedback.
  pub scale: f32,
}
