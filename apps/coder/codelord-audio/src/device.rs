//! Audio device management.
//!
//! Provides shared audio output device that is created once at startup and
//! shared between SFX and music player subsystems.

use crate::error::AudioError;

use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};

/// Shared audio output device.
///
/// Created once at startup, shared between SFX and music player.
/// The sink must be kept alive for audio to work.
pub struct AudioDevice {
  /// The device sink - must keep alive.
  sink: MixerDeviceSink,
}

impl AudioDevice {
  /// Create a new audio device using the default output.
  pub fn new() -> Result<Self, AudioError> {
    let sink = DeviceSinkBuilder::open_default_sink()
      .map_err(|e| AudioError::DeviceInit(e.to_string()))?;

    Ok(Self { sink })
  }

  /// Create a new player for audio playback.
  pub fn create_player(&self) -> Player {
    Player::connect_new(self.sink.mixer())
  }

  /// Get the stream mixer for direct playback.
  pub fn mixer(&self) -> &rodio::mixer::Mixer {
    self.sink.mixer()
  }

  /// Get the device sink reference.
  pub fn device_sink(&self) -> &MixerDeviceSink {
    &self.sink
  }
}
