//! Toaster resource and message types.

use super::components::{
  Toast, ToastAction, ToastAnimation, ToastId, ToastStatus,
};
use crate::time::current_time_ms;

use bevy_ecs::message::Message;
use bevy_ecs::resource::Resource;
use eazy::{Curve, Easing};

use std::collections::VecDeque;

/// Command to show a toast notification.
/// Send this message from anywhere in the app to display a toast.
#[derive(Message, Debug, Clone)]
pub struct ToastCommand {
  pub message: String,
  pub status: ToastStatus,
  pub actions: Vec<ToastAction>,
}

impl ToastCommand {
  pub fn info(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      status: ToastStatus::Info,
      actions: Vec::new(),
    }
  }

  pub fn success(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      status: ToastStatus::Success,
      actions: Vec::new(),
    }
  }

  pub fn warning(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      status: ToastStatus::Warning,
      actions: Vec::new(),
    }
  }

  pub fn error(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      status: ToastStatus::Error,
      actions: Vec::new(),
    }
  }

  /// Add action buttons to the toast (makes it persistent).
  pub fn with_actions(mut self, actions: Vec<ToastAction>) -> Self {
    self.actions = actions;
    self
  }
}

/// Command to dismiss a toast by ID.
#[derive(Message, Debug, Clone, Copy)]
pub struct DismissToastCommand(pub ToastId);

/// Event fired when user clicks an action button on a toast.
#[derive(Message, Debug, Clone)]
pub struct ToastActionEvent {
  pub toast_id: ToastId,
  pub action_id: String,
}

/// Resource managing all active toasts.
#[derive(Resource)]
pub struct ToasterResource {
  toasts: VecDeque<Toast>,
  // Layout constants
  pub toast_width: f32,
  pub toast_height: f32,
  pub toast_margin: f32,
  pub start_y: f32,
  // Animation timing
  entry_duration: f32,
  display_duration: f32,
  exit_duration: f32,
}

impl Default for ToasterResource {
  fn default() -> Self {
    Self {
      toasts: VecDeque::with_capacity(16),
      toast_width: 350.0,
      toast_height: 50.0,
      toast_margin: 8.0,
      start_y: 48.0,
      entry_duration: 0.5,
      display_duration: 3.0,
      exit_duration: 0.3,
    }
  }
}

impl ToasterResource {
  /// Add a new toast at the top, shifting others down.
  pub fn push(
    &mut self,
    message: String,
    status: ToastStatus,
    actions: Vec<ToastAction>,
  ) {
    let slot_height = self.toast_height + self.toast_margin;

    // Shift existing toasts down
    for (index, toast) in self.toasts.iter_mut().enumerate() {
      toast.animation.target_y =
        self.start_y + ((index + 1) as f32) * slot_height;
    }

    // Create new toast at top
    let toast = Toast {
      id: ToastId::new(),
      message,
      status,
      created_at: current_time_ms(),
      animation: ToastAnimation {
        x_offset: self.toast_width,
        opacity: 0.0,
        y_position: self.start_y,
        target_y: self.start_y,
      },
      actions,
    };

    self.toasts.push_front(toast);
  }

  /// Update all toast animations.
  pub fn update(&mut self, current_time: u64) {
    if self.toasts.is_empty() {
      return;
    }

    let mut expired_ids = Vec::new();

    for toast in self.toasts.iter_mut() {
      let age_secs = (current_time - toast.created_at) as f32 / 1000.0;
      let has_actions = !toast.actions.is_empty();

      // Smooth Y interpolation
      let y_diff = toast.animation.target_y - toast.animation.y_position;
      if y_diff.abs() > 0.1 {
        toast.animation.y_position += y_diff * 0.15;
      } else {
        toast.animation.y_position = toast.animation.target_y;
      }

      // Phase 1: Entry
      if age_secs < self.entry_duration {
        let t = age_secs / self.entry_duration;
        let eased = Easing::OutElastic.y(t);
        toast.animation.x_offset = self.toast_width * (1.0 - eased);
        toast.animation.opacity = t;
      }
      // Phase 2: Display (persistent if has actions)
      else if has_actions
        || age_secs < self.entry_duration + self.display_duration
      {
        toast.animation.x_offset = 0.0;
        toast.animation.opacity = 1.0;
      }
      // Phase 3: Exit (only for toasts without actions)
      else if age_secs
        < self.entry_duration + self.display_duration + self.exit_duration
      {
        let exit_t = (age_secs - self.entry_duration - self.display_duration)
          / self.exit_duration;
        let eased = Easing::InQuadratic.y(exit_t);
        toast.animation.x_offset = self.toast_width * eased;
        toast.animation.opacity = 1.0 - exit_t;
      }
      // Phase 4: Expired (only for toasts without actions)
      else {
        expired_ids.push(toast.id);
      }
    }

    self.toasts.retain(|t| !expired_ids.contains(&t.id));
  }

  /// Remove a toast by ID (user dismissed).
  pub fn dismiss(&mut self, id: ToastId) {
    self.toasts.retain(|t| t.id != id);
  }

  /// Iterator over active toasts.
  pub fn iter(&self) -> impl Iterator<Item = &Toast> {
    self.toasts.iter()
  }

  /// Check if empty.
  pub fn is_empty(&self) -> bool {
    self.toasts.is_empty()
  }
}
