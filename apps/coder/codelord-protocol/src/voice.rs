pub mod dto;
pub mod model;

/// Shared voice-visualizer status, produced by `codelord-voice` and
/// consumed by `codelord-core` (and downstream UI). Lives here so the
/// two sibling crates don't each define their own near-identical copy.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum VisualizerStatus {
  /// Not recording or playing.
  #[default]
  Idle,
  /// User speaking (recording).
  Listening,
  /// Transcribing/thinking.
  Processing,
  /// AI responding.
  Speaking,
  /// Brief success feedback.
  Success,
  /// Brief error feedback.
  Error,
}
