//! Music player engine.
//!
//! Provides background music playback with:
//! - Play, pause, resume, stop controls
//! - Volume control
//! - Position tracking via atomics (UI can read without blocking)
//! - Waveform visualization
//! - Repeat mode
//!
//! Follows console music player architecture (PlayStation/Xbox approach):
//! - Dedicated audio thread (never blocks main UI thread)
//! - Streaming architecture (rodio streams on-demand)
//! - Command-based control (fire-and-forget via lock-free queue)

use crate::error::AudioError;
use crate::monitored_source::MonitoredSource;
use crate::visualizer::WaveformVisualizer;

use rodio::{Decoder, MixerDeviceSink, Player as RodioPlayer, Source};

use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Music player state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
  /// No track loaded.
  Stopped,
  /// Track is playing.
  Playing,
  /// Track is paused (can be resumed).
  Paused,
}

/// Music player engine.
///
/// Handles playback of a single track with controls.
/// Lives exclusively on the audio thread.
pub struct MusicPlayer {
  /// Current playback player (rodio handle).
  current_player: Option<RodioPlayer>,
  /// Path to current track.
  current_track: Option<PathBuf>,
  /// Playback state.
  state: PlaybackState,
  /// Music volume (0.0 - 1.0).
  volume: f32,
  /// Current playback position in microseconds (atomic for UI reads).
  position: Arc<AtomicU64>,
  /// Waveform visualizer (shared with UI thread).
  visualizer: WaveformVisualizer,
  /// Start time of playback (for position tracking).
  playback_start: Option<std::time::Instant>,
  /// Track duration in microseconds.
  duration_us: Option<u64>,
  /// Whether repeat mode is enabled.
  is_repeat: bool,
}

impl MusicPlayer {
  /// Create a new music player.
  pub fn new() -> Self {
    Self {
      current_player: None,
      current_track: None,
      state: PlaybackState::Stopped,
      volume: 1.0,
      position: Arc::new(AtomicU64::new(0)),
      visualizer: WaveformVisualizer::new(),
      playback_start: None,
      duration_us: None,
      is_repeat: false,
    }
  }

  /// Get a clone of the visualizer (for sharing with UI thread).
  pub fn visualizer(&self) -> WaveformVisualizer {
    self.visualizer.clone()
  }

  /// Get current playback state.
  pub fn state(&self) -> PlaybackState {
    self.state
  }

  /// Get current volume.
  pub fn volume(&self) -> f32 {
    self.volume
  }

  /// Get current track path.
  pub fn current_track(&self) -> Option<&PathBuf> {
    self.current_track.as_ref()
  }

  /// Get atomic position handle (for UI thread reads).
  pub fn position_handle(&self) -> Arc<AtomicU64> {
    Arc::clone(&self.position)
  }

  /// Get track duration in microseconds.
  pub fn duration_us(&self) -> Option<u64> {
    self.duration_us
  }

  /// Play a music track.
  pub fn play(
    &mut self,
    track_path: PathBuf,
    device_sink: &MixerDeviceSink,
  ) -> Result<(), AudioError> {
    // Stop current playback if any.
    self.stop();

    // Open the music file.
    let file = File::open(&track_path)?;

    // Decode the audio using try_from for better seeking support.
    let source = Decoder::try_from(file)?;

    // Extract duration from source (if available).
    let duration = source.total_duration();
    self.duration_us = duration.map(|d| d.as_micros() as u64);

    // Wrap with MonitoredSource to capture samples for visualization.
    let monitored = MonitoredSource::new(source, self.visualizer.clone());

    // Enable visualizer.
    self.visualizer.set_active(true);

    // Create a new player.
    let player = RodioPlayer::connect_new(device_sink.mixer());

    player.set_volume(self.volume);
    player.append(monitored);

    // Store state.
    self.current_player = Some(player);
    self.current_track = Some(track_path);
    self.state = PlaybackState::Playing;
    self.position.store(0, Ordering::Relaxed);
    self.playback_start = Some(std::time::Instant::now());

    log::info!("Music playback started: {:?}", self.current_track);

    Ok(())
  }

  /// Pause playback.
  pub fn pause(&mut self) {
    if !self.is_playing() {
      return;
    }

    if let Some(player) = &self.current_player {
      // Store current position before pausing.
      if let Some(start_time) = self.playback_start {
        let elapsed = start_time.elapsed();

        self
          .position
          .store(elapsed.as_micros() as u64, Ordering::Relaxed);
      }

      player.pause();

      self.state = PlaybackState::Paused;

      log::debug!(
        "Music paused at position: {:?}",
        Duration::from_micros(self.position.load(Ordering::Relaxed))
      );
    }
  }

  /// Resume playback.
  pub fn resume(&mut self) {
    if !self.is_paused() {
      return;
    }

    if let Some(player) = &self.current_player {
      // Adjust playback_start to account for time paused.
      let current_position =
        Duration::from_micros(self.position.load(Ordering::Relaxed));

      self.playback_start = Some(std::time::Instant::now() - current_position);

      player.play();

      self.state = PlaybackState::Playing;

      log::debug!("Music resumed from position: {current_position:?}");
    }
  }

  /// Stop playback.
  pub fn stop(&mut self) {
    if let Some(player) = self.current_player.take() {
      player.stop();
    }

    // Disable visualizer and clear samples.
    self.visualizer.set_active(false);
    self.visualizer.clear();

    self.current_track = None;
    self.state = PlaybackState::Stopped;
    self.position.store(0, Ordering::Relaxed);
    self.playback_start = None;

    log::debug!("Music stopped");
  }

  /// Toggle play/pause.
  pub fn toggle(&mut self) {
    match self.state {
      PlaybackState::Playing => self.pause(),
      PlaybackState::Paused => self.resume(),
      PlaybackState::Stopped => {
        log::warn!("Cannot toggle playback from stopped state");
      }
    }
  }

  /// Set music volume (0.0 - 1.0).
  pub fn set_volume(&mut self, volume: f32) {
    self.volume = volume.clamp(0.0, 1.0);

    if let Some(player) = &self.current_player {
      player.set_volume(self.volume);
    }

    log::debug!("Music volume set to: {:.2}", self.volume);
  }

  /// Seek to a specific position in the track.
  pub fn seek(&mut self, position: Duration) -> Result<(), AudioError> {
    if let Some(player) = &self.current_player {
      match player.try_seek(position) {
        Ok(()) => {
          self.playback_start = Some(std::time::Instant::now() - position);

          self
            .position
            .store(position.as_micros() as u64, Ordering::Relaxed);

          log::info!("Seek to position: {position:?}");

          Ok(())
        }
        Err(e) => {
          log::warn!("Seek failed: {e:?}");

          Err(AudioError::Seek(e.to_string()))
        }
      }
    } else {
      log::warn!("Cannot seek: no track loaded");

      Ok(())
    }
  }

  /// Set repeat mode.
  pub fn set_repeat(&mut self, repeat: bool) {
    self.is_repeat = repeat;

    log::debug!("Repeat mode set to: {repeat}");
  }

  /// Get repeat mode.
  pub fn is_repeat(&self) -> bool {
    self.is_repeat
  }

  /// Check if a track is currently loaded.
  pub fn is_loaded(&self) -> bool {
    self.current_track.is_some()
  }

  /// Check if currently playing.
  pub fn is_playing(&self) -> bool {
    self.state == PlaybackState::Playing
  }

  /// Check if currently paused.
  pub fn is_paused(&self) -> bool {
    self.state == PlaybackState::Paused
  }

  /// Update playback position (called periodically from audio thread).
  pub fn update(&mut self, device_sink: &MixerDeviceSink) {
    if !self.is_playing() {
      return;
    }

    // Update position based on elapsed time.
    if let Some(start_time) = self.playback_start {
      self
        .position
        .store(start_time.elapsed().as_micros() as u64, Ordering::Relaxed);
    }

    // Check if playback finished naturally.
    let finished = self.current_player.as_ref().is_some_and(|p| p.empty());

    if !finished {
      return;
    }

    log::info!("Music track finished");

    // Try to repeat, otherwise stop.
    if self.is_repeat
      && let Some(track_path) = self.current_track.clone()
    {
      log::info!("Repeating track: {track_path:?}");

      if let Err(e) = self.play(track_path, device_sink) {
        log::error!("Failed to repeat track: {e:?}");
        self.stop();
      }

      return;
    }

    self.stop();
  }
}

impl Default for MusicPlayer {
  fn default() -> Self {
    Self::new()
  }
}

/// Music player state snapshot (for sending to UI thread).
#[derive(Debug, Clone)]
pub struct MusicPlayerSnapshot {
  /// Current playback state.
  pub state: PlaybackState,
  /// Current volume.
  pub volume: f32,
  /// Current track name (if any).
  pub track_name: Option<String>,
  /// Current position in microseconds.
  pub position_us: u64,
  /// Track duration in microseconds (if known).
  pub duration_us: Option<u64>,
  /// Whether repeat mode is enabled.
  pub is_repeat: bool,
}

impl MusicPlayerSnapshot {
  /// Create a snapshot from the music player.
  pub fn from_player(player: &MusicPlayer) -> Self {
    let track_name = player.current_track().map(|path| {
      path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown")
        .to_string()
    });

    Self {
      state: player.state(),
      volume: player.volume(),
      track_name,
      position_us: player.position_handle().load(Ordering::Relaxed),
      duration_us: player.duration_us(),
      is_repeat: player.is_repeat(),
    }
  }

  /// Get position as Duration.
  pub fn position(&self) -> Duration {
    Duration::from_micros(self.position_us)
  }

  /// Get duration as Duration (if known).
  pub fn duration(&self) -> Option<Duration> {
    self.duration_us.map(Duration::from_micros)
  }
}
