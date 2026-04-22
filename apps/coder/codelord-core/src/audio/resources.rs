//! Audio ECS resources.
//!
//! Provides Bevy ECS resources for audio state management in the IDE.

use crate::animation::height::HeightAnimation;
use crate::ecs::resource::Resource;

use codelord_audio::player::{MusicPlayerSnapshot, PlaybackState};
use codelord_audio::sfx::SoundEffect;
use codelord_audio::visualizer::WaveformVisualizer;

use std::path::PathBuf;
use std::time::Duration;

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

  /// Sync UI state from the engine snapshot.
  ///
  /// Volume is intentionally not synced — the UI slider is
  /// authoritative, and local changes are pushed to the engine via
  /// [`AudioDispatcher::music_set_volume`].
  pub fn update_from_snapshot(&mut self, snapshot: &MusicPlayerSnapshot) {
    self.is_playing = snapshot.state == PlaybackState::Playing;
    self.is_repeat = snapshot.is_repeat;
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

  /// Flip music playback on/off. Same entry point as clicking the play
  /// button in the UI.
  ///
  /// If a track is already loaded: toggle play/pause via the engine.
  /// If nothing is loaded but the playlist has entries: start the
  /// currently-selected entry (or the first one). If the playlist is
  /// also empty: no-op. Matches macOS Music / Spotify defaults.
  ///
  /// UI state flips immediately for snappy feedback — the engine state
  /// syncs on the next snapshot tick.
  pub fn toggle(&mut self, audio: &AudioDispatcher, playlist: &Playlist) {
    // `self.snapshot` is Some on every frame (show_time_display caches
    // the engine's current state unconditionally). The real "track
    // loaded" signal is `track_name.is_some()`.
    let has_loaded_track = self
      .snapshot
      .as_ref()
      .is_some_and(|s| s.track_name.is_some());

    if has_loaded_track {
      audio.music_toggle();

      self.is_playing = !self.is_playing;

      return;
    }

    let path = playlist
      .current()
      .or_else(|| playlist.entries.first())
      .map(|entry| entry.path.clone());

    match path {
      Some(path) => {
        audio.music_play(path);

        self.is_playing = true;
      }
      None => {
        log::debug!("music toggle: no track loaded and playlist is empty");
      }
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
#[derive(Resource, Default, Debug, Clone)]
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

/// ECS handle for the global audio subsystem.
///
/// Wraps `codelord_audio`'s `LazyLock<Sender<AudioCommand>>` under ECS
/// discipline. Callers take `Res<AudioDispatcher>` (or fetch it from the
/// world) instead of importing `codelord_audio::music_play` directly —
/// this surfaces the audio dependency in the schedule and keeps the
/// "flying function" bypass out of the rest of the codebase.
///
/// The struct is zero-sized. Each method delegates to
/// `codelord_audio::send_command`. Query methods (`music_snapshot`,
/// `music_visualizer`) preserve the upstream blocking-with-100ms-timeout
/// semantics.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct AudioDispatcher;

impl AudioDispatcher {
  /// Play a one-shot sound effect. Non-blocking; pre-decoded samples
  /// dispatched to the audio thread. Rapid same-effect repeats are
  /// rate-limited to 50 ms by the SFX player to prevent audio spam.
  pub fn sfx_play(&self, effect: SoundEffect) {
    codelord_audio::play_sfx(effect);
  }

  /// Play a one-shot sound effect at a custom volume. The `volume`
  /// argument (0.0 – 1.0, clamped) multiplies the global SFX volume.
  /// Same cooldown rules as [`sfx_play`].
  pub fn sfx_play_with_volume(&self, effect: SoundEffect, volume: f32) {
    codelord_audio::play_sfx_with_volume(effect, volume);
  }

  /// Set the global SFX volume multiplier (0.0 – 1.0, clamped).
  pub fn sfx_set_volume(&self, volume: f32) {
    codelord_audio::set_sfx_volume(volume);
  }

  /// Enable or disable all SFX playback. When disabled, [`sfx_play`]
  /// and [`sfx_play_with_volume`] become no-ops.
  pub fn sfx_set_enabled(&self, enabled: bool) {
    codelord_audio::set_sfx_enabled(enabled);
  }

  /// Load and start playing a music track. Replaces any currently
  /// loaded track; decoding happens on the audio thread so ~100 ms
  /// startup latency is acceptable.
  pub fn music_play(&self, path: PathBuf) {
    codelord_audio::music_play(path);
  }

  /// Suspend playback. Position and loaded track are preserved —
  /// continue with [`music_resume`] or flip with [`music_toggle`].
  pub fn music_pause(&self) {
    codelord_audio::music_pause();
  }

  /// Continue playback from the paused position. No-op if nothing is
  /// paused.
  pub fn music_resume(&self) {
    codelord_audio::music_resume();
  }

  /// Flip playback state: playing ↔ paused. No-op when the engine has
  /// no track loaded (the audio thread logs a warning in that case).
  pub fn music_toggle(&self) {
    codelord_audio::music_toggle();
  }

  /// Set the music volume (0.0 – 1.0, clamped). Applies immediately to
  /// the currently loaded track.
  pub fn music_set_volume(&self, volume: f32) {
    codelord_audio::music_set_volume(volume);
  }

  /// Seek to an absolute position within the current track.
  pub fn music_seek(&self, position: Duration) {
    codelord_audio::music_seek(position);
  }

  /// Enable or disable repeat-on-end for the current track.
  pub fn music_set_repeat(&self, repeat: bool) {
    codelord_audio::music_set_repeat(repeat);
  }

  /// Snapshot the music player: playback state, loaded track name,
  /// position, duration, volume, repeat flag.
  ///
  /// **Blocking** — round-trips through the audio thread on a one-shot
  /// channel with a 100 ms timeout. Returns `None` on timeout.
  pub fn music_snapshot(&self) -> Option<MusicPlayerSnapshot> {
    codelord_audio::get_music_snapshot()
  }

  /// Fetch the latest waveform samples for UI visualization.
  ///
  /// **Blocking** — round-trips through the audio thread on a one-shot
  /// channel with a 100 ms timeout. Returns `None` on timeout.
  pub fn music_visualizer(&self) -> Option<WaveformVisualizer> {
    codelord_audio::get_visualizer()
  }

  /// Stop the music and terminate the dedicated audio thread.
  /// Intended for app shutdown — see `eframe::App::on_exit`.
  pub fn shutdown(&self) {
    codelord_audio::shutdown();
  }
}
