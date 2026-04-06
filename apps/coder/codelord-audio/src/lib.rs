//! IDE Audio System
//!
//! A high-performance audio system for the IDE with two distinct subsystems:
//!
//! - **Sound Effects (SFX)**: Instant UI feedback (<10ms latency), many
//!   simultaneous
//! - **Music Player**: Background music playback (~100ms OK), single track +
//!   crossfade
//!
//! ## Architecture
//!
//! Uses a dedicated audio thread with lock-free command dispatch (flume
//! channels). All blocking operations (file I/O, decoding) happen on the audio
//! thread, never blocking the main UI thread.
//!
//! ## Key Design Decisions (from codelord)
//!
//! - **Dedicated audio thread**: No runtime overhead, predictable timing
//! - **flume channels**: Lock-free, faster than `std::sync::mpsc`
//! - **LazyLock init**: One-time setup, globally accessible sender
//! - **Pre-cached SFX**: Zero runtime file I/O for sound effects
//! - **Atomic visualization**: UI reads without blocking audio
//! - **SFX cooldown**: Prevents audio spam (50ms debounce)

pub mod device;
pub mod error;
pub mod manager;
pub mod monitored_source;
pub mod player;
pub mod sfx;
pub mod source;
pub mod visualizer;

// Re-export main types.
pub use error::AudioError;
pub use player::{MusicPlayer, MusicPlayerSnapshot, PlaybackState};
pub use sfx::{SfxPlayer, SoundEffect};
pub use visualizer::{WAVEFORM_SAMPLES, WaveformVisualizer};

// Re-export manager API.
pub use manager::{
  get_music_snapshot, get_visualizer, init, music_pause, music_play,
  music_resume, music_seek, music_set_repeat, music_set_volume, music_stop,
  music_toggle, play_sfx, play_sfx_with_volume, set_sfx_enabled,
  set_sfx_volume, shutdown,
};
