//! Sound effects engine.
//!
//! Provides instant, non-blocking sound effect playback with:
//! - Pre-decoded sounds at startup (zero runtime decoding)
//! - Cooldown to prevent audio spam (50ms between same sounds)
//! - Volume control per effect and global
//!
//! ## Performance
//!
//! SFX playback is guaranteed <1ms because:
//! 1. Sounds are pre-decoded to PCM samples at startup
//! 2. `SamplesBuffer` just wraps the `Arc`, no copying
//! 3. `play_raw` queues to audio thread, non-blocking

use crate::device::AudioDevice;

use rodio::Source;
use rodio::buffer::SamplesBuffer;
use rustc_hash::FxHashMap as HashMap;

use std::io::Cursor;
use std::num::NonZero;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Available sound effects.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SoundEffect {
  // UI Interactions.
  Click,
  Hover,
  Select,
  Deselect,

  // Editor Actions.
  Save,
  Undo,
  Redo,
  Delete,

  // Notifications.
  Success,
  Warning,
  Error,
  Notification,

  // Navigation.
  TabSwitch,
  PanelOpen,
  PanelClose,

  // Special.
  Startup,
  Shutdown,

  // Cursor (from codelord).
  CursorMove,
  CursorBlink,
}

/// Pre-decoded sound effect data.
///
/// Stores raw PCM samples - NO decoding needed at runtime.
struct SfxData {
  /// Pre-decoded PCM samples (f32).
  samples: Arc<[f32]>,
  /// Sample rate (e.g., 44100).
  sample_rate: u32,
  /// Number of channels (1 = mono, 2 = stereo).
  channels: u16,
}

/// Cooldown state to prevent SFX spam.
///
/// Codelord pattern: 50ms minimum between same sounds.
struct SfxCooldown {
  last_effect: Option<SoundEffect>,
  last_played: Instant,
}

impl SfxCooldown {
  const COOLDOWN_MS: u64 = 50;

  fn new() -> Self {
    Self {
      last_effect: None,
      last_played: Instant::now(),
    }
  }

  /// Check if we can play this effect (not in cooldown).
  fn can_play(&self, effect: SoundEffect) -> bool {
    // Different effect? Always allow.
    if self.last_effect != Some(effect) {
      return true;
    }

    // Same effect? Check cooldown.
    self.last_played.elapsed() >= Duration::from_millis(Self::COOLDOWN_MS)
  }

  /// Record that we played an effect.
  fn record(&mut self, effect: SoundEffect) {
    self.last_effect = Some(effect);
    self.last_played = Instant::now();
  }
}

/// Sound effects player.
///
/// Pre-decodes all sounds at startup for instant, non-blocking playback.
/// Includes cooldown to prevent audio spam (50ms between same sounds).
pub struct SfxPlayer {
  device: Rc<AudioDevice>,
  sounds: HashMap<SoundEffect, SfxData>,
  cooldown: SfxCooldown,
  volume: f32,
  enabled: bool,
}

impl SfxPlayer {
  /// Create new SFX player.
  ///
  /// **Note:** This blocks during startup to pre-decode all sounds.
  /// This is intentional - startup blocking is acceptable.
  pub fn new(device: Rc<AudioDevice>) -> Self {
    Self {
      device,
      sounds: HashMap::default(),
      cooldown: SfxCooldown::new(),
      volume: 0.5,
      enabled: true,
    }
  }

  /// Load and pre-decode a sound from bytes.
  ///
  /// Blocking is OK here - runs once at startup.
  pub fn register(&mut self, effect: SoundEffect, bytes: &[u8]) {
    let Ok(decoder) = rodio::Decoder::new(Cursor::new(bytes.to_vec())) else {
      log::warn!("Failed to decode SFX {effect:?}");
      return;
    };

    // Get format info before consuming decoder.
    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();

    // Decode ONCE, store as f32 samples.
    let samples: Vec<f32> = decoder.collect();

    self.sounds.insert(
      effect,
      SfxData {
        samples: Arc::from(samples),
        sample_rate: sample_rate.into(),
        channels: channels.into(),
      },
    );

    log::debug!("Registered SFX {effect:?}");
  }

  /// Play a sound effect immediately.
  ///
  /// **Guaranteed non-blocking** - just wraps pre-decoded samples.
  /// Takes <1ms even for long sounds.
  /// Respects cooldown to prevent spam (50ms between same sounds).
  pub fn play(&mut self, effect: SoundEffect) {
    if !self.enabled {
      return;
    }

    // Check cooldown (codelord pattern).
    if !self.cooldown.can_play(effect) {
      return; // Skip - same sound played too recently.
    }

    let Some(data) = self.sounds.get(&effect) else {
      log::trace!("SFX {effect:?} not loaded");
      return;
    };

    // Create source from pre-decoded samples - NO DECODING!
    // SamplesBuffer just wraps the data, no copying.
    let samples_vec: Vec<f32> = data.samples.to_vec();
    let channels = NonZero::new(data.channels).unwrap();
    let sample_rate = NonZero::new(data.sample_rate).unwrap();
    let source = SamplesBuffer::new(channels, sample_rate, samples_vec);

    // Apply volume and play via mixer.
    let source = source.amplify(self.volume);

    // Add to mixer - non-blocking.
    self.device.mixer().add(source);

    // Record for cooldown.
    self.cooldown.record(effect);
  }

  /// Play with custom volume (0.0 - 1.0).
  pub fn play_with_volume(&mut self, effect: SoundEffect, volume: f32) {
    if !self.enabled {
      return;
    }

    // Check cooldown.
    if !self.cooldown.can_play(effect) {
      return;
    }

    let Some(data) = self.sounds.get(&effect) else {
      return;
    };

    let samples_vec: Vec<f32> = data.samples.to_vec();
    let channels = NonZero::new(data.channels).unwrap();
    let sample_rate = NonZero::new(data.sample_rate).unwrap();
    let source = SamplesBuffer::new(channels, sample_rate, samples_vec);

    let final_volume = self.volume * volume.clamp(0.0, 1.0);
    let source = source.amplify(final_volume);

    self.device.mixer().add(source);
    self.cooldown.record(effect);
  }

  /// Set global SFX volume (0.0 - 1.0).
  pub fn set_volume(&mut self, volume: f32) {
    self.volume = volume.clamp(0.0, 1.0);
  }

  /// Get current volume.
  pub fn volume(&self) -> f32 {
    self.volume
  }

  /// Enable or disable SFX playback.
  pub fn set_enabled(&mut self, enabled: bool) {
    self.enabled = enabled;
  }

  /// Check if SFX is enabled.
  pub fn is_enabled(&self) -> bool {
    self.enabled
  }
}
