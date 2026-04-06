//! Error types for the voice control system.

use std::fmt;

/// Result type for voice operations.
pub type VoiceResult<T> = Result<T, VoiceError>;

/// Error types for voice operations.
#[derive(Debug)]
pub enum VoiceError {
  /// Audio device initialization failed.
  AudioDeviceError(String),
  /// Whisper model loading failed.
  ModelLoadError(String),
  /// Transcription failed.
  TranscriptionError(String),
  /// Command not recognized.
  UnrecognizedCommand(String),
  /// Action channel closed.
  ChannelClosed,
  /// Audio stream error.
  StreamError(String),
  /// Server communication error.
  ServerError(String),
}

impl fmt::Display for VoiceError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::AudioDeviceError(msg) => write!(f, "Audio device error: {msg}"),
      Self::ModelLoadError(msg) => write!(f, "Model load error: {msg}"),
      Self::TranscriptionError(msg) => write!(f, "Transcription error: {msg}"),
      Self::UnrecognizedCommand(text) => {
        write!(f, "Unrecognized command: '{text}'")
      }
      Self::ChannelClosed => write!(f, "Action channel closed"),
      Self::StreamError(msg) => write!(f, "Audio stream error: {msg}"),
      Self::ServerError(msg) => write!(f, "Server error: {msg}"),
    }
  }
}

impl std::error::Error for VoiceError {}
