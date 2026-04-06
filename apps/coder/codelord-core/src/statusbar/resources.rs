use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use eazy::interpolation::linear::lerp::lerp;
use eazy::{Curve, Easing};

/// Resource tracking statusbar button entities.
///
/// Buttons are stored as entity lists for left and right columns.
/// The statusbar renderer iterates these to display buttons.
#[derive(Resource, Default)]
pub struct StatusbarResource {
  /// Button entities for the left column (e.g., Explorer)
  pub left: Vec<Entity>,
  /// Button entities for the right column (e.g., Voice)
  pub right: Vec<Entity>,
}

impl StatusbarResource {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn add_left(&mut self, entity: Entity) {
    self.left.push(entity);
  }

  pub fn add_right(&mut self, entity: Entity) {
    self.right.push(entity);
  }
}

/// Resource for animated line/column display in statusbar.
///
/// Uses the generic animation system. Systems update this resource,
/// render functions only read the current values.
#[derive(Resource, Debug, Clone)]
pub struct LineColumnAnimation {
  /// Current displayed line (animated)
  pub line: usize,
  /// Current displayed column (animated)
  pub column: usize,
  /// Target line value
  target_line: f32,
  /// Target column value
  target_col: f32,
  /// Current interpolated line
  current_line: f32,
  /// Current interpolated column
  current_col: f32,
  /// Animation elapsed time
  elapsed: f32,
  /// Animation duration
  duration: f32,
  /// Start values for interpolation
  start_line: f32,
  start_col: f32,
  /// Whether animation is currently active
  pub is_active: bool,
  /// Easing function
  easing: Easing,
}

impl Default for LineColumnAnimation {
  fn default() -> Self {
    Self {
      line: 1,
      column: 1,
      target_line: 1.0,
      target_col: 1.0,
      current_line: 1.0,
      current_col: 1.0,
      elapsed: 0.0,
      duration: 0.3,
      start_line: 1.0,
      start_col: 1.0,
      is_active: false,
      easing: Easing::InOutCubic,
    }
  }
}

impl LineColumnAnimation {
  pub fn new() -> Self {
    Self::default()
  }

  /// Set new target values. Returns true if animation was started.
  pub fn set_target(&mut self, line: usize, col: usize) -> bool {
    let new_line = line as f32;
    let new_col = col as f32;

    let line_changed = (self.target_line - new_line).abs() > 0.001;
    let col_changed = (self.target_col - new_col).abs() > 0.001;

    if line_changed || col_changed {
      self.start_line = self.current_line;
      self.start_col = self.current_col;
      self.target_line = new_line;
      self.target_col = new_col;
      self.elapsed = 0.0;

      // Only return true if we weren't already animating
      let was_inactive = !self.is_active;
      self.is_active = true;

      return was_inactive;
    }

    false
  }

  /// Update animation with delta time. Returns true if animation completed.
  pub fn update(&mut self, dt: f32) -> bool {
    if !self.is_active {
      return false;
    }

    self.elapsed += dt;

    if self.elapsed >= self.duration {
      // Animation complete
      self.current_line = self.target_line;
      self.current_col = self.target_col;
      self.line = self.target_line.round() as usize;
      self.column = self.target_col.round() as usize;
      self.is_active = false;

      return true;
    }

    // Interpolate
    let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
    let eased = self.easing.y(t);

    self.current_line = lerp(eased, self.start_line, self.target_line);
    self.current_col = lerp(eased, self.start_col, self.target_col);
    self.line = self.current_line.round() as usize;
    self.column = self.current_col.round() as usize;

    false
  }
}
