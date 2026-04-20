//! Global loading indicator resource.
//!
//! Tracks loading tasks across the application for unified progress display.
//! Used by the titlebar to show a global progress indicator.

use crate::time::current_time_ms;

use bevy_ecs::prelude::Resource;

/// Categories of loading tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadingTask {
  /// PDF page rendering in background.
  PdfRender,
  /// Voice transcription processing.
  VoiceProcessing,
  /// Database query execution.
  SqliteQuery,
  /// File loading/parsing.
  FileLoad,
  /// Network request.
  Network,
  /// Compilation.
  Compilation,
}

/// Global loading state resource.
///
/// Tracks active loading tasks. The titlebar displays a progress indicator
/// when any task is active.
#[derive(Resource, Default)]
pub struct GlobalLoading {
  /// Bitflags of active loading tasks.
  active: u32,
  /// Timestamp when loading started (for animation).
  pub start_time: u64,
  /// Timestamp when loading completed (for success animation).
  pub completed_time: u64,
}

impl GlobalLoading {
  /// Start a loading task.
  pub fn start(&mut self, task: LoadingTask) {
    let was_loading = self.is_loading();
    self.active |= 1 << (task as u32);

    // Set start time when transitioning from idle to loading
    if !was_loading {
      self.start_time = current_time_ms();
      self.completed_time = 0;
    }
  }

  /// Finish a loading task.
  pub fn finish(&mut self, task: LoadingTask) {
    self.active &= !(1 << (task as u32));

    // Set completed time when all tasks finish
    if !self.is_loading() && self.completed_time == 0 {
      self.completed_time = current_time_ms();
    }
  }

  /// Check if any task is loading.
  pub fn is_loading(&self) -> bool {
    self.active != 0
  }

  /// Check if a specific task is active.
  pub fn is_task_active(&self, task: LoadingTask) -> bool {
    (self.active & (1 << (task as u32))) != 0
  }

  /// Check if recently completed (for success animation).
  pub fn is_completed(&self) -> bool {
    if self.completed_time == 0 {
      return false;
    }
    let elapsed = current_time_ms().saturating_sub(self.completed_time);
    elapsed < 500 // Show success for 500ms
  }

  /// Clear completed state after animation.
  pub fn clear_completed(&mut self) {
    self.completed_time = 0;
  }
}
