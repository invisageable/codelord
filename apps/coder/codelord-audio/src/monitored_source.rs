//! Monitored audio source wrapper.
//!
//! Wraps a rodio Source to capture samples for waveform visualization
//! while passing them through to the audio output unchanged.

use crate::visualizer::WaveformVisualizer;

use rodio::Source;
use rodio::{ChannelCount, SampleRate};

use std::time::Duration;

/// A source wrapper that monitors samples for visualization.
///
/// Captures every Nth sample to avoid overwhelming the visualizer
/// while maintaining accurate waveform representation.
pub struct MonitoredSource<S>
where
  S: Source<Item = f32>,
{
  /// The underlying audio source.
  inner: S,
  /// Waveform visualizer to send samples to.
  visualizer: WaveformVisualizer,
  /// Counter for sample decimation.
  sample_counter: usize,
  /// Decimation factor - capture every Nth sample.
  /// At 44.1kHz stereo, capturing every 32 samples gives ~1378 samples/sec.
  decimation: usize,
}

impl<S> MonitoredSource<S>
where
  S: Source<Item = f32>,
{
  /// Create a new monitored source.
  ///
  /// The decimation factor controls how many samples to skip between captures.
  /// A factor of 32 at 44.1kHz stereo gives good visualization without
  /// overhead.
  pub fn new(source: S, visualizer: WaveformVisualizer) -> Self {
    Self {
      inner: source,
      visualizer,
      sample_counter: 0,
      decimation: 32,
    }
  }

  /// Create with custom decimation factor.
  pub fn with_decimation(
    source: S,
    visualizer: WaveformVisualizer,
    decimation: usize,
  ) -> Self {
    Self {
      inner: source,
      visualizer,
      sample_counter: 0,
      decimation: decimation.max(1),
    }
  }
}

impl<S> Iterator for MonitoredSource<S>
where
  S: Source<Item = f32>,
{
  type Item = f32;

  fn next(&mut self) -> Option<Self::Item> {
    let sample = self.inner.next()?;

    // Capture every Nth sample for visualization.
    self.sample_counter += 1;

    if self.sample_counter >= self.decimation {
      self.sample_counter = 0;
      self.visualizer.push_sample(sample);
    }

    Some(sample)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    self.inner.size_hint()
  }
}

impl<S> Source for MonitoredSource<S>
where
  S: Source<Item = f32>,
{
  fn current_span_len(&self) -> Option<usize> {
    self.inner.current_span_len()
  }

  fn channels(&self) -> ChannelCount {
    self.inner.channels()
  }

  fn sample_rate(&self) -> SampleRate {
    self.inner.sample_rate()
  }

  fn total_duration(&self) -> Option<Duration> {
    self.inner.total_duration()
  }

  fn try_seek(
    &mut self,
    pos: Duration,
  ) -> Result<(), rodio::source::SeekError> {
    self.inner.try_seek(pos)
  }
}
