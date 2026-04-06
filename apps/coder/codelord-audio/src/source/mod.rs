//! Audio source abstractions.
//!
//! Provides traits and implementations for different audio sources:
//! - Local files (MVP)
//! - HTTP streams (future)
//! - Streaming services (future): SoundCloud, Spotify, YouTube

pub mod file;

pub use file::FileSource;

use std::path::PathBuf;

/// Source identifier for queue/history.
#[derive(Debug, Clone)]
pub enum SourceId {
  /// Local file path.
  File(PathBuf),
  /// URL for HTTP streams.
  Url(String),
  /// SoundCloud track.
  SoundCloud { track_id: String },
  /// Spotify track.
  Spotify { track_id: String },
  /// YouTube video.
  YouTube { video_id: String },
}

impl std::fmt::Display for SourceId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::File(path) => write!(f, "file://{}", path.display()),
      Self::Url(url) => write!(f, "{url}"),
      Self::SoundCloud { track_id } => write!(f, "soundcloud:{track_id}"),
      Self::Spotify { track_id } => write!(f, "spotify:{track_id}"),
      Self::YouTube { video_id } => write!(f, "youtube:{video_id}"),
    }
  }
}
