//! Audio system integration for the IDE.
//!
//! This module re-exports the `codelord_audio` crate and provides ECS resources
//! for audio management.
//!
//! ## Usage
//!
//! ```ignore
//! // Play a sound effect
//! codelord_core::audio::play_sfx(SoundEffect::Click);
//!
//! // Play music
//! codelord_core::audio::music_play(path);
//! ```

pub mod resources;

// Re-export codelord_audio types for convenience.
pub use codelord_audio::{
  // Types.
  MusicPlayer,
  MusicPlayerSnapshot,
  PlaybackState,
  SfxPlayer,
  SoundEffect,
  WaveformVisualizer,
  // Manager functions.
  get_music_snapshot,
  get_visualizer,
  init,
  music_pause,
  music_play,
  music_resume,
  music_seek,
  music_set_repeat,
  music_set_volume,
  music_stop,
  music_toggle,
  play_sfx,
  play_sfx_with_volume,
  set_sfx_enabled,
  set_sfx_volume,
  shutdown,
};
