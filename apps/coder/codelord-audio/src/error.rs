use std::error::Error;
use std::fmt;

/// Audio system errors.
#[derive(Debug)]
pub enum AudioError {
  /// Failed to initialize audio device.
  DeviceInit(String),
  /// Failed to create audio sink.
  SinkCreate(String),
  /// Failed to decode audio file.
  Decode(String),
  /// Failed to open audio file.
  FileOpen(std::io::Error),
  /// Failed to play audio.
  Playback(String),
  /// Seek operation failed.
  Seek(String),
  /// Audio format not supported.
  UnsupportedFormat(String),
}

impl fmt::Display for AudioError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::DeviceInit(msg) => {
        write!(f, "failed to initialize audio device: {msg}")
      }
      Self::SinkCreate(msg) => write!(f, "failed to create audio sink: {msg}"),
      Self::Decode(msg) => write!(f, "failed to decode audio: {msg}"),
      Self::FileOpen(err) => write!(f, "failed to open audio file: {err}"),
      Self::Playback(msg) => write!(f, "failed to play audio: {msg}"),
      Self::Seek(msg) => write!(f, "seek failed: {msg}"),
      Self::UnsupportedFormat(msg) => {
        write!(f, "unsupported audio format: {msg}")
      }
    }
  }
}

impl Error for AudioError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      Self::FileOpen(err) => Some(err),
      _ => None,
    }
  }
}

impl From<std::io::Error> for AudioError {
  fn from(err: std::io::Error) -> Self {
    AudioError::FileOpen(err)
  }
}

impl From<rodio::DeviceSinkError> for AudioError {
  fn from(err: rodio::DeviceSinkError) -> Self {
    AudioError::DeviceInit(err.to_string())
  }
}

impl From<rodio::PlayError> for AudioError {
  fn from(err: rodio::PlayError) -> Self {
    AudioError::SinkCreate(err.to_string())
  }
}

impl From<rodio::decoder::DecoderError> for AudioError {
  fn from(err: rodio::decoder::DecoderError) -> Self {
    AudioError::Decode(err.to_string())
  }
}
