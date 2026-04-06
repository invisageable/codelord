//! Audio ECS resources.
//!
//! Provides Bevy ECS resources for audio state management in the IDE.

use crate::animation::height::HeightAnimation;

use codelord_audio::player::MusicPlayerSnapshot;
use codelord_audio::visualizer::WaveformVisualizer;

use bevy_ecs::prelude::Resource;

/// Music player UI state resource.
///
/// Tracks UI-specific state for the music player panel.
#[derive(Resource)]
pub struct MusicPlayerState {
  /// Whether the music player panel is visible.
  pub visible: bool,
  /// Whether the music player UI is playing.
  pub is_playing: bool,
  /// Current volume (0.0 - 1.0).
  pub volume: f32,
  /// Whether the volume is muted.
  pub is_muted: bool,
  /// Volume before muting (to restore when unmuting).
  pub volume_before_mute: f32,
  /// Whether repeat mode is enabled.
  pub is_repeat: bool,
  /// Whether the playlist panel is visible.
  pub playlist_visible: bool,
  /// Whether captions are visible.
  pub caption_visible: bool,
  /// Cached waveform visualizer (from audio thread).
  pub waveform: Option<WaveformVisualizer>,
  /// Cached music snapshot (from audio thread).
  pub snapshot: Option<MusicPlayerSnapshot>,
  /// Height animation for smooth panel transitions.
  pub height_animation: HeightAnimation,
}

impl Default for MusicPlayerState {
  fn default() -> Self {
    Self::new()
  }
}

impl MusicPlayerState {
  /// Create a new music player state with default values.
  pub fn new() -> Self {
    Self {
      visible: false,
      is_playing: false,
      volume: 1.0,
      is_muted: false,
      volume_before_mute: 1.0,
      is_repeat: false,
      playlist_visible: false,
      caption_visible: false,
      waveform: None,
      snapshot: None,
      height_animation: HeightAnimation::new(0.0),
    }
  }

  /// Update state from audio snapshot.
  pub fn update_from_snapshot(&mut self, snapshot: &MusicPlayerSnapshot) {
    self.is_playing =
      snapshot.state == codelord_audio::player::PlaybackState::Playing;
    self.is_repeat = snapshot.is_repeat;
    // Volume is managed locally, not synced from snapshot.
  }

  /// Toggle mute state.
  pub fn toggle_mute(&mut self) {
    if self.is_muted {
      // Unmute: restore previous volume.
      self.volume = self.volume_before_mute;
      self.is_muted = false;
    } else {
      // Mute: save current volume and set to 0.
      self.volume_before_mute = self.volume;
      self.volume = 0.0;
      self.is_muted = true;
    }
  }

  /// Toggle music player visibility.
  pub fn toggle_visibility(&mut self, current_time: f32) {
    self.visible = !self.visible;

    let target = if self.visible { 40.0 } else { 0.0 };

    self.height_animation.set_target(target, current_time);
  }

  /// Check if music player is visible.
  pub fn is_visible(&self) -> bool {
    self.visible
  }

  /// Toggle playback - same behavior as clicking the play button.
  pub fn toggle_playback(&mut self) {
    use codelord_audio::{music_pause, music_play, music_resume};

    self.is_playing = !self.is_playing;

    if self.is_playing {
      let is_paused = self
        .snapshot
        .as_ref()
        .is_some_and(|s| s.state == codelord_audio::PlaybackState::Paused);

      if is_paused {
        music_resume();
      } else {
        let track_path = std::env::current_dir()
          .unwrap_or_default()
          .join("apps/coder/codelord-assets/sound/freeze-corleone-desiigner-a-colors-show.mp3");
        music_play(track_path);
      }
    } else {
      music_pause();
    }
  }
}

/// Playlist entry for the music player.
#[derive(Debug, Clone)]
pub struct PlaylistEntry {
  /// Song title.
  pub title: String,
  /// Artist name.
  pub artist: String,
  /// Album name.
  pub album: String,
  /// Duration formatted as string (e.g., "3:45").
  pub time: String,
  /// Genre.
  pub genre: String,
  /// Play count.
  pub plays: u32,
  /// File path.
  pub path: std::path::PathBuf,
}

/// Playlist resource for the music player.
#[derive(Resource, Default)]
pub struct Playlist {
  /// List of playlist entries.
  pub entries: Vec<PlaylistEntry>,
  /// Currently selected index.
  pub current_index: Option<usize>,
}

impl Playlist {
  /// Create a new empty playlist.
  pub fn new() -> Self {
    Self {
      entries: Vec::new(),
      current_index: None,
    }
  }

  /// Add an entry to the playlist.
  pub fn add(&mut self, entry: PlaylistEntry) {
    self.entries.push(entry);
  }

  /// Get the current entry.
  pub fn current(&self) -> Option<&PlaylistEntry> {
    self.current_index.and_then(|i| self.entries.get(i))
  }

  /// Select the next track.
  pub fn select_next(&mut self) -> Option<&PlaylistEntry> {
    if self.entries.is_empty() {
      return None;
    }

    let next_index = match self.current_index {
      Some(i) => (i + 1) % self.entries.len(),
      None => 0,
    };

    self.current_index = Some(next_index);
    self.entries.get(next_index)
  }

  /// Select the previous track.
  pub fn previous(&mut self) -> Option<&PlaylistEntry> {
    if self.entries.is_empty() {
      return None;
    }

    let prev_index = match self.current_index {
      Some(i) => {
        if i == 0 {
          self.entries.len() - 1
        } else {
          i - 1
        }
      }
      None => 0,
    };

    self.current_index = Some(prev_index);
    self.entries.get(prev_index)
  }
}
