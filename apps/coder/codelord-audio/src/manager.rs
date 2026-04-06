//! Global audio manager.
//!
//! Provides a thread-safe global audio system that can be accessed from
//! anywhere in the application. Uses an async command channel pattern for
//! true non-blocking audio operations.
//!
//! ## Architecture
//!
//! - **LazyLock sender**: Global command sender initialized once
//! - **Dedicated thread**: Audio engine lives on separate thread
//! - **Lock-free channels**: flume for command dispatch
//! - **No mutex**: Audio state only accessed from audio thread

use crate::device::AudioDevice;
use crate::player::{MusicPlayer, MusicPlayerSnapshot};
use crate::sfx::{SfxPlayer, SoundEffect};
use crate::visualizer::WaveformVisualizer;

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::Duration;

/// Commands sent from main thread to audio thread.
#[derive(Debug)]
pub enum AudioCommand {
  // SFX Commands.
  PlaySfx(SoundEffect),
  PlaySfxWithVolume(SoundEffect, f32),
  SetSfxVolume(f32),
  SetSfxEnabled(bool),

  // Music Commands.
  MusicPlay(PathBuf),
  MusicPause,
  MusicResume,
  MusicStop,
  MusicToggle,
  MusicSetVolume(f32),
  MusicSeek(Duration),
  MusicSetRepeat(bool),

  // Query Commands (with response channel).
  GetVisualizer(flume::Sender<WaveformVisualizer>),
  GetMusicSnapshot(flume::Sender<MusicPlayerSnapshot>),

  // System Commands.
  Shutdown,
}

/// Audio engine that lives on the dedicated audio thread.
struct AudioEngine {
  /// Audio device - owns the output stream.
  device: Rc<AudioDevice>,
  /// Sound effects player.
  sfx: SfxPlayer,
  /// Music player.
  music: MusicPlayer,
}

impl AudioEngine {
  /// Create a new audio engine.
  fn new() -> Result<Self, Box<dyn std::error::Error>> {
    let device = Rc::new(AudioDevice::new()?);

    Ok(Self {
      device: Rc::clone(&device),
      sfx: SfxPlayer::new(device),
      music: MusicPlayer::new(),
    })
  }

  /// Load cursor sounds from assets directory.
  fn load_sounds(&mut self, assets_path: &std::path::Path) {
    // Load cursor sounds if they exist.
    let sound_dir = assets_path.join("sfx");

    if let Ok(bytes) = std::fs::read(sound_dir.join("click.wav")) {
      self.sfx.register(SoundEffect::Click, &bytes);
    }
    if let Ok(bytes) = std::fs::read(sound_dir.join("hover.wav")) {
      self.sfx.register(SoundEffect::Hover, &bytes);
    }
    if let Ok(bytes) = std::fs::read(sound_dir.join("select.wav")) {
      self.sfx.register(SoundEffect::Select, &bytes);
    }
    if let Ok(bytes) = std::fs::read(sound_dir.join("save.wav")) {
      self.sfx.register(SoundEffect::Save, &bytes);
    }
    if let Ok(bytes) = std::fs::read(sound_dir.join("error.wav")) {
      self.sfx.register(SoundEffect::Error, &bytes);
    }
    if let Ok(bytes) = std::fs::read(sound_dir.join("success.wav")) {
      self.sfx.register(SoundEffect::Success, &bytes);
    }
  }

  /// Process a single audio command.
  fn process_command(&mut self, command: AudioCommand) {
    match command {
      // SFX.
      AudioCommand::PlaySfx(effect) => self.sfx.play(effect),
      AudioCommand::PlaySfxWithVolume(effect, vol) => {
        self.sfx.play_with_volume(effect, vol)
      }
      AudioCommand::SetSfxVolume(vol) => self.sfx.set_volume(vol),
      AudioCommand::SetSfxEnabled(enabled) => self.sfx.set_enabled(enabled),

      // Music.
      AudioCommand::MusicPlay(path) => {
        log::info!("MusicPlay command received for: {path:?}");
        match self.music.play(path, self.device.device_sink()) {
          Ok(()) => log::info!("Music playback started successfully"),
          Err(e) => log::error!("Failed to play music: {e}"),
        }
      }
      AudioCommand::MusicPause => self.music.pause(),
      AudioCommand::MusicResume => self.music.resume(),
      AudioCommand::MusicStop => self.music.stop(),
      AudioCommand::MusicToggle => self.music.toggle(),
      AudioCommand::MusicSetVolume(vol) => self.music.set_volume(vol),
      AudioCommand::MusicSeek(pos) => {
        if let Err(e) = self.music.seek(pos) {
          log::error!("Failed to seek: {e}");
        }
      }
      AudioCommand::MusicSetRepeat(repeat) => self.music.set_repeat(repeat),

      // Queries.
      AudioCommand::GetVisualizer(tx) => {
        let _ = tx.send(self.music.visualizer());
      }
      AudioCommand::GetMusicSnapshot(tx) => {
        let snapshot = MusicPlayerSnapshot::from_player(&self.music);
        let _ = tx.send(snapshot);
      }

      // System.
      AudioCommand::Shutdown => {
        log::info!("Audio engine shutting down");
        self.music.stop();
      }
    }
  }

  /// Update music player state (check for track completion, etc).
  fn update(&mut self) {
    self.music.update(self.device.device_sink());
  }
}

/// Global audio command sender - the only shared state.
static AUDIO_COMMAND_SENDER: LazyLock<flume::Sender<AudioCommand>> =
  LazyLock::new(|| {
    let (tx, rx) = flume::unbounded();

    std::thread::Builder::new()
      .name("codelord-audio".into())
      .spawn(move || {
        match AudioEngine::new() {
          Ok(mut engine) => {
            // Load sounds from assets.
            let assets_path =
              std::path::PathBuf::from("apps/coder/codelord-assets");

            engine.load_sounds(&assets_path);

            log::info!("Audio system initialized successfully");

            // Main loop - process commands with periodic updates.
            loop {
              // Try to receive a command with timeout for periodic updates.
              match rx.recv_timeout(Duration::from_millis(16)) {
                Ok(AudioCommand::Shutdown) => {
                  engine.process_command(AudioCommand::Shutdown);

                  break;
                }
                Ok(command) => {
                  engine.process_command(command);
                }
                Err(flume::RecvTimeoutError::Timeout) => {
                  // No command, just update.
                }
                Err(flume::RecvTimeoutError::Disconnected) => {
                  log::info!("Audio command channel disconnected");

                  break;
                }
              }

              // Periodic update for music player state.
              engine.update();
            }

            log::info!("Audio thread shutting down");
          }
          Err(error) => {
            log::error!("Failed to initialize audio engine: {error}");
          }
        }
      })
      .expect("Failed to spawn audio thread");

    tx
  });

/// Initialize the global audio system.
///
/// Force the lazy initialization by accessing the sender.
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
  let _ = &*AUDIO_COMMAND_SENDER;

  Ok(())
}

/// Send an audio command to the global audio system.
///
/// Non-blocking - just sends a message and returns.
pub fn send_command(command: AudioCommand) {
  AUDIO_COMMAND_SENDER.send(command).ok();
}

/// Shutdown the audio system.
pub fn shutdown() {
  send_command(AudioCommand::Shutdown);
}

// ============================================================================
// SFX API
// ============================================================================

/// Play a sound effect.
pub fn play_sfx(effect: SoundEffect) {
  send_command(AudioCommand::PlaySfx(effect));
}

/// Play a sound effect with specific volume.
pub fn play_sfx_with_volume(effect: SoundEffect, volume: f32) {
  send_command(AudioCommand::PlaySfxWithVolume(effect, volume));
}

/// Set the SFX volume.
pub fn set_sfx_volume(volume: f32) {
  send_command(AudioCommand::SetSfxVolume(volume));
}

/// Enable or disable SFX.
pub fn set_sfx_enabled(enabled: bool) {
  send_command(AudioCommand::SetSfxEnabled(enabled));
}

// ============================================================================
// Music API
// ============================================================================

/// Play a music track.
pub fn music_play(path: PathBuf) {
  send_command(AudioCommand::MusicPlay(path));
}

/// Pause music playback.
pub fn music_pause() {
  send_command(AudioCommand::MusicPause);
}

/// Resume music playback.
pub fn music_resume() {
  send_command(AudioCommand::MusicResume);
}

/// Stop music playback.
pub fn music_stop() {
  send_command(AudioCommand::MusicStop);
}

/// Toggle music play/pause.
pub fn music_toggle() {
  send_command(AudioCommand::MusicToggle);
}

/// Set music volume (0.0 - 1.0).
pub fn music_set_volume(volume: f32) {
  send_command(AudioCommand::MusicSetVolume(volume));
}

/// Seek to a specific position in the track.
pub fn music_seek(position: Duration) {
  send_command(AudioCommand::MusicSeek(position));
}

/// Set repeat mode.
pub fn music_set_repeat(repeat: bool) {
  send_command(AudioCommand::MusicSetRepeat(repeat));
}

/// Get the waveform visualizer (blocking call with timeout).
pub fn get_visualizer() -> Option<WaveformVisualizer> {
  let (tx, rx) = flume::bounded(1);

  send_command(AudioCommand::GetVisualizer(tx));
  rx.recv_timeout(Duration::from_millis(100)).ok()
}

/// Get the music player snapshot (blocking call with timeout).
pub fn get_music_snapshot() -> Option<MusicPlayerSnapshot> {
  let (tx, rx) = flume::bounded(1);

  send_command(AudioCommand::GetMusicSnapshot(tx));
  rx.recv_timeout(Duration::from_millis(100)).ok()
}
