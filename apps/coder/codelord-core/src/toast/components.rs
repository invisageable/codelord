use std::sync::atomic::{AtomicU64, Ordering};

/// Unique toast identifier (monotonic counter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

impl ToastId {
  /// Creates a new unique toast ID.
  pub fn new() -> Self {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    Self(COUNTER.fetch_add(1, Ordering::Relaxed))
  }

  /// Returns the inner u64 value (for egui ID generation).
  pub const fn as_u64(&self) -> u64 {
    self.0
  }
}

impl Default for ToastId {
  fn default() -> Self {
    Self::new()
  }
}

/// Toast status level (determines icon and color).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastStatus {
  #[default]
  Info,
  Success,
  Warning,
  Error,
}

impl ToastStatus {
  /// Returns the RGB color for this status.
  pub const fn color_rgb(&self) -> (u8, u8, u8) {
    match self {
      Self::Info => (241, 196, 15),
      Self::Success => (6, 208, 1),
      Self::Warning => (255, 165, 0),
      Self::Error => (220, 53, 69),
    }
  }
}

/// Animation state for a toast.
#[derive(Debug, Clone, Copy)]
pub struct ToastAnimation {
  /// Horizontal offset (0.0 = visible, toast_width = off-screen).
  pub x_offset: f32,
  /// Opacity (0.0 to 1.0).
  pub opacity: f32,
  /// Current Y position.
  pub y_position: f32,
  /// Target Y position (for slide animation).
  pub target_y: f32,
}

/// Action button for interactive toasts.
#[derive(Debug, Clone)]
pub struct ToastAction {
  pub id: String,
  pub label: String,
  pub primary: bool,
  pub stripe: bool,
}

impl ToastAction {
  pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
    Self {
      id: id.into(),
      label: label.into(),
      primary: false,
      stripe: false,
    }
  }

  pub fn primary(mut self) -> Self {
    self.primary = true;
    self
  }

  pub fn stripe(mut self) -> Self {
    self.stripe = true;
    self
  }
}

/// A single toast notification.
#[derive(Debug, Clone)]
pub struct Toast {
  pub id: ToastId,
  pub message: String,
  pub status: ToastStatus,
  pub created_at: u64,
  pub animation: ToastAnimation,
  /// Optional action buttons (makes toast persistent until dismissed).
  pub actions: Vec<ToastAction>,
}
