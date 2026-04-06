//! Terminal ECS components.

use bevy_ecs::component::Component;

use crate::ecs::world::World;
use crate::tabbar::{TabMarker, ZoomSource};

/// Marker component for terminal tabs.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerminalTab;

impl TabMarker for TerminalTab {
  fn spawn_new_tab_event(world: &mut World) {
    world.spawn(crate::events::NewTerminalTabRequest);
  }

  fn zoom_source() -> ZoomSource {
    ZoomSource::Terminal
  }
}

/// Individual terminal cell with character, colors, and styling flags.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCell {
  pub character: char,
  pub fg_color: u32,
  pub bg_color: u32,
  pub flags: u8,
}

impl Default for TerminalCell {
  fn default() -> Self {
    Self {
      character: ' ',
      fg_color: 0xFFFFFFFF, // White
      bg_color: 0x00000000, // Transparent
      flags: 0,
    }
  }
}

/// Cell styling flags.
pub const CELL_FLAG_BOLD: u8 = 0b00000001;
pub const CELL_FLAG_ITALIC: u8 = 0b00000010;
pub const CELL_FLAG_UNDERLINE: u8 = 0b00000100;
pub const CELL_FLAG_STRIKETHROUGH: u8 = 0b00001000;
pub const CELL_FLAG_DIM: u8 = 0b00010000;
pub const CELL_FLAG_REVERSE: u8 = 0b00100000;

/// Pack RGBA into u32.
pub fn pack_color(r: u8, g: u8, b: u8, a: u8) -> u32 {
  ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Unpack u32 to RGBA.
pub fn unpack_color(color: u32) -> (u8, u8, u8, u8) {
  let a = ((color >> 24) & 0xFF) as u8;
  let r = ((color >> 16) & 0xFF) as u8;
  let g = ((color >> 8) & 0xFF) as u8;
  let b = (color & 0xFF) as u8;

  (r, g, b, a)
}

/// Terminal grid - 2D array of cells.
/// Component attached to terminal tab entities.
#[derive(Component, Clone)]
pub struct TerminalGrid {
  pub cells: Vec<TerminalCell>,
  pub width: u16,
  pub height: u16,
}

impl Default for TerminalGrid {
  fn default() -> Self {
    Self::new(80, 24)
  }
}

impl TerminalGrid {
  pub fn new(width: u16, height: u16) -> Self {
    let capacity = (width as usize) * (height as usize);
    Self {
      cells: vec![TerminalCell::default(); capacity],
      width,
      height,
    }
  }

  pub fn resize(&mut self, width: u16, height: u16) {
    let new_capacity = (width as usize) * (height as usize);
    self.cells.resize(new_capacity, TerminalCell::default());
    self.width = width;
    self.height = height;
  }

  pub fn get_cell(&self, row: u16, col: u16) -> Option<&TerminalCell> {
    if row >= self.height || col >= self.width {
      return None;
    }

    let index = (row as usize) * (self.width as usize) + (col as usize);

    self.cells.get(index)
  }

  pub fn set_cell(&mut self, row: u16, col: u16, cell: TerminalCell) {
    if row >= self.height || col >= self.width {
      return;
    }

    let index = (row as usize) * (self.width as usize) + (col as usize);

    if let Some(c) = self.cells.get_mut(index) {
      *c = cell;
    }
  }
}

/// Terminal cursor state.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerminalCursor {
  pub row: u16,
  pub col: u16,
  pub visible: bool,
}

impl TerminalCursor {
  pub fn new() -> Self {
    Self {
      row: 0,
      col: 0,
      visible: true,
    }
  }
}

/// Terminal text selection.
#[derive(Component, Debug, Clone, Copy)]
pub struct TerminalSelection {
  pub start_row: u16,
  pub start_col: u16,
  pub end_row: u16,
  pub end_col: u16,
}

/// Terminal scroll state.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerminalScroll {
  pub offset: f32,
  pub total_lines: usize,
  pub display_offset: usize,
}
