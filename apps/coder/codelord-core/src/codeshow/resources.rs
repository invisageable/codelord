//! Codeshow presentation resources.

use bevy_ecs::resource::Resource;
use rustc_hash::FxHashMap;

use std::path::PathBuf;

/// Resource for codeshow presentation state.
#[derive(Resource, Default)]
pub struct CodeshowState {
  /// Current slide index (0-based).
  pub current: usize,
  /// Total number of slides.
  pub total: usize,
  /// Parsed slides (markdown content per slide).
  pub slides: Vec<String>,
  /// Presenter notes per slide (slide index -> note content).
  pub notes: FxHashMap<usize, String>,
  /// Source file path (single file mode).
  pub source_path: Option<PathBuf>,
  /// Source directory path (multi-file mode).
  pub source_dir: Option<PathBuf>,
  /// Slide transition animation progress (0.0 to 1.0).
  pub transition_progress: f32,
  /// Direction of current transition.
  pub transition_direction: Option<SlideTransition>,
}

/// Pending file dialog receiver (non-blocking).
#[derive(Resource)]
pub struct PendingPresentationFile(pub flume::Receiver<Option<PathBuf>>);

/// Pending folder dialog receiver (non-blocking).
#[derive(Resource)]
pub struct PendingPresentationDirectory(pub flume::Receiver<Option<PathBuf>>);

#[derive(Clone, Copy, PartialEq)]
pub enum SlideTransition {
  Next,
  Previous,
}

impl CodeshowState {
  /// Load presentation from a single markdown file (slides separated by `---`).
  pub fn load_file(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
    let content = std::fs::read_to_string(&path)?;
    self.slides = content
      .split("\n---\n")
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
      .collect();
    self.total = self.slides.len();
    self.current = 0;
    self.source_path = Some(path);
    self.source_dir = None;
    self.transition_progress = 1.0;
    self.transition_direction = None;
    Ok(())
  }

  /// Load presentation from a directory (one markdown file per slide).
  /// Files are sorted alphabetically (e.g., 01-intro.md, 02-agenda.md, ...).
  pub fn load_directory(&mut self, dir: PathBuf) -> Result<(), std::io::Error> {
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
      .filter_map(|e| e.ok())
      .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
      .collect();

    entries.sort_by_key(|e| e.path());

    self.slides = entries
      .into_iter()
      .filter_map(|e| std::fs::read_to_string(e.path()).ok())
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
      .collect();

    self.total = self.slides.len();
    self.current = 0;
    self.source_path = None;
    self.source_dir = Some(dir);
    self.transition_progress = 1.0;
    self.transition_direction = None;
    Ok(())
  }

  /// Navigate to next slide.
  pub fn next(&mut self) {
    if self.current + 1 < self.total {
      self.current += 1;
      self.transition_progress = 0.0;
      self.transition_direction = Some(SlideTransition::Next);
    }
  }

  /// Navigate to previous slide.
  pub fn previous(&mut self) {
    if self.current > 0 {
      self.current -= 1;
      self.transition_progress = 0.0;
      self.transition_direction = Some(SlideTransition::Previous);
    }
  }

  /// Go to first slide.
  pub fn first(&mut self) {
    if self.current != 0 {
      self.current = 0;
      self.transition_progress = 0.0;
      self.transition_direction = Some(SlideTransition::Previous);
    }
  }

  /// Go to last slide.
  pub fn last(&mut self) {
    if self.current != self.total.saturating_sub(1) {
      self.current = self.total.saturating_sub(1);
      self.transition_progress = 0.0;
      self.transition_direction = Some(SlideTransition::Next);
    }
  }

  /// Go to specific slide by index.
  pub fn goto(&mut self, index: usize) {
    if index < self.total && index != self.current {
      let direction = if index > self.current {
        SlideTransition::Next
      } else {
        SlideTransition::Previous
      };
      self.current = index;
      self.transition_progress = 0.0;
      self.transition_direction = Some(direction);
    }
  }

  /// Get current slide content.
  pub fn current_slide(&self) -> Option<&str> {
    self.slides.get(self.current).map(|s| s.as_str())
  }

  /// Get current slide notes.
  pub fn current_notes(&self) -> Option<&str> {
    self.notes.get(&self.current).map(|s| s.as_str())
  }

  /// Set note for a specific slide.
  pub fn set_note(&mut self, index: usize, note: String) {
    if note.is_empty() {
      self.notes.remove(&index);
    } else {
      self.notes.insert(index, note);
    }
  }

  /// Get note for a specific slide.
  pub fn get_note(&self, index: usize) -> Option<&str> {
    self.notes.get(&index).map(|s| s.as_str())
  }

  /// Update transition animation.
  pub fn update_transition(&mut self, delta: f32) {
    if self.transition_direction.is_some() {
      self.transition_progress =
        (self.transition_progress + delta * 4.0).min(1.0);
      if self.transition_progress >= 1.0 {
        self.transition_direction = None;
      }
    }
  }

  /// Check if transition is animating.
  pub fn is_animating(&self) -> bool {
    self.transition_direction.is_some()
  }

  /// Check if presentation is loaded.
  pub fn is_loaded(&self) -> bool {
    !self.slides.is_empty()
  }
}
