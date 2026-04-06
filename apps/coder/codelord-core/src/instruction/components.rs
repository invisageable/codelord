//! Instruction component types for keyboard shortcuts display.

use crate::icon::components::Icon;

/// Instruction key - either an icon or text.
#[derive(Debug, Clone)]
pub enum InstructionKey {
  /// Icon key (Cmd, Shift, etc).
  Icon(Icon),
  /// Text key (letter or word).
  Text(&'static str),
}

/// Single instruction entry with description and keyboard shortcut.
#[derive(Debug, Clone)]
pub struct Instruction {
  pub description: &'static str,
  pub keys: Vec<InstructionKey>,
}

impl Instruction {
  /// Creates a new instruction.
  pub fn new(description: &'static str, keys: Vec<InstructionKey>) -> Self {
    Self { description, keys }
  }
}

/// A section of instructions with a title.
#[derive(Debug, Clone)]
pub struct InstructionSection {
  pub title: &'static str,
  pub instructions: Vec<Instruction>,
}

impl InstructionSection {
  /// Creates a new instruction section.
  pub fn new(title: &'static str, instructions: Vec<Instruction>) -> Self {
    Self {
      title,
      instructions,
    }
  }
}
