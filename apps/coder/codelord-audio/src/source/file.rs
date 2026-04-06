//! Local file audio source.
//!
//! Provides audio playback from local files with metadata extraction.

use crate::error::AudioError;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::Hint;

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Track metadata extracted from audio file.
#[derive(Debug, Clone, Default)]
pub struct TrackMetadata {
  /// Track title.
  pub title: Option<String>,
  /// Artist name.
  pub artist: Option<String>,
  /// Album name.
  pub album: Option<String>,
  /// Track duration.
  pub duration: Option<Duration>,
}

/// Local file audio source.
pub struct FileSource {
  /// Path to the audio file.
  path: PathBuf,
  /// Extracted metadata.
  metadata: TrackMetadata,
}

impl FileSource {
  /// Create a new file source.
  ///
  /// Extracts metadata from the file (blocking operation).
  pub fn new(path: PathBuf) -> Result<Self, AudioError> {
    let metadata = extract_metadata(&path);

    Ok(Self { path, metadata })
  }

  /// Get the file path.
  pub fn path(&self) -> &Path {
    &self.path
  }

  /// Get track metadata.
  pub fn metadata(&self) -> &TrackMetadata {
    &self.metadata
  }

  /// Get duration if known.
  pub fn duration(&self) -> Option<Duration> {
    self.metadata.duration
  }
}

/// Extract metadata from an audio file.
///
/// Uses symphonia for proper tag extraction (ID3v2, Vorbis, FLAC, etc.).
/// Falls back to filename if no metadata is found.
fn extract_metadata(path: &Path) -> TrackMetadata {
  let fallback_title =
    path.file_stem().and_then(|s| s.to_str()).map(String::from);

  let Ok(file) = File::open(path) else {
    return TrackMetadata {
      title: fallback_title,
      artist: None,
      album: None,
      duration: None,
    };
  };

  let mss = MediaSourceStream::new(Box::new(file), Default::default());

  let mut hint = Hint::new();

  if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
    hint.with_extension(ext);
  }

  let format_opts = FormatOptions::default();
  let metadata_opts = MetadataOptions::default();

  let Ok(mut probed) = symphonia::default::get_probe().format(
    &hint,
    mss,
    &format_opts,
    &metadata_opts,
  ) else {
    return TrackMetadata {
      title: fallback_title,
      artist: None,
      album: None,
      duration: None,
    };
  };

  let mut title = None;
  let mut artist = None;
  let mut album = None;
  let mut duration = None;

  // Extract duration from track info.
  if let Some(track) = probed.format.default_track()
    && let Some(n_frames) = track.codec_params.n_frames
    && let Some(sample_rate) = track.codec_params.sample_rate
  {
    let secs = n_frames as f64 / sample_rate as f64;
    duration = Some(Duration::from_secs_f64(secs));
  }

  // Extract metadata from probe result.
  if let Some(metadata) = probed.metadata.get()
    && let Some(rev) = metadata.current()
  {
    for tag in rev.tags() {
      match tag.std_key {
        Some(StandardTagKey::TrackTitle) => {
          title = Some(tag.value.to_string());
        }
        Some(StandardTagKey::Artist) => {
          artist = Some(tag.value.to_string());
        }
        Some(StandardTagKey::Album) => {
          album = Some(tag.value.to_string());
        }
        _ => {}
      }
    }
  }

  TrackMetadata {
    title: title.or(fallback_title),
    artist,
    album,
    duration,
  }
}
