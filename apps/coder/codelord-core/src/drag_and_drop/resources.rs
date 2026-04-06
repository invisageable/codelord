use bevy_ecs::resource::Resource;

use std::any::Any;
use std::sync::Arc;

/// Axis constraint for drag operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DragAxis {
  /// Horizontal only.
  X,
  /// Vertical only.
  Y,
  /// Free movement.
  #[default]
  XY,
  /// No visual drag (mouse position only).
  None,
}

/// Current drag state.
#[derive(Resource, Default)]
pub struct DragState {
  /// Type-erased payload.
  payload: Option<Arc<dyn Any + Send + Sync>>,
  /// Axis constraint.
  pub axis: DragAxis,
  /// Drag start position.
  pub start_pos: Option<[f32; 2]>,
  /// Current drag position.
  pub current_pos: Option<[f32; 2]>,
  /// Source identifier (e.g., "titlebar", "explorer").
  pub source: Option<String>,
}

impl DragState {
  /// Set payload with axis constraint.
  pub fn set_payload<T: Any + Send + Sync>(
    &mut self,
    payload: T,
    axis: DragAxis,
    source: &str,
    start_pos: [f32; 2],
  ) {
    self.payload = Some(Arc::new(payload));
    self.axis = axis;
    self.source = Some(source.to_string());
    self.start_pos = Some(start_pos);
    self.current_pos = Some(start_pos);
  }

  /// Get payload if type matches.
  pub fn payload<T: Any + Send + Sync>(&self) -> Option<&T> {
    self.payload.as_ref()?.downcast_ref::<T>()
  }

  /// Take payload (clears state).
  pub fn take_payload<T: Any + Send + Sync + Clone>(&mut self) -> Option<T> {
    let result = self.payload.as_ref()?.downcast_ref::<T>().cloned();

    if result.is_some() {
      self.clear();
    }

    result
  }

  /// Check if payload type matches.
  pub fn has_payload<T: Any>(&self) -> bool {
    self
      .payload
      .as_ref()
      .map(|p| p.downcast_ref::<T>().is_some())
      .unwrap_or(false)
  }

  /// Check if any payload exists.
  pub fn is_dragging(&self) -> bool {
    self.payload.is_some()
  }

  /// Update current position (constrained by axis).
  pub fn update_pos(&mut self, mouse_pos: [f32; 2]) {
    if let Some(start) = self.start_pos {
      self.current_pos = Some(match self.axis {
        DragAxis::X => [mouse_pos[0], start[1]],
        DragAxis::Y => [start[0], mouse_pos[1]],
        DragAxis::XY => mouse_pos,
        DragAxis::None => start,
      });
    }
  }

  /// Get drag delta from start.
  pub fn delta(&self) -> [f32; 2] {
    match (self.start_pos, self.current_pos) {
      (Some(start), Some(current)) => {
        [current[0] - start[0], current[1] - start[1]]
      }
      _ => [0.0, 0.0],
    }
  }

  /// Clear drag state.
  pub fn clear(&mut self) {
    self.payload = None;
    self.axis = DragAxis::default();
    self.start_pos = None;
    self.current_pos = None;
    self.source = None;
  }
}
