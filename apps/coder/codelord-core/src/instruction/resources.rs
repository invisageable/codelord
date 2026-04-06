//! Instruction resource for ECS-managed keyboard shortcut data.

use super::components::{Instruction, InstructionKey, InstructionSection};
use crate::icon::components::{Icon, Key};

use bevy_ecs::resource::Resource;

/// Resource holding instruction sections for empty editor state.
#[derive(Resource, Debug, Clone)]
pub struct InstructionsResource {
  /// The sections to display when no file is open.
  pub sections: Vec<InstructionSection>,
}

impl Default for InstructionsResource {
  fn default() -> Self {
    Self {
      sections: vec![
        InstructionSection::new(
          "GET STARTED",
          vec![
            Instruction::new(
              "New Untitled File",
              vec![
                InstructionKey::Icon(Icon::Key(Key::Cmd)),
                InstructionKey::Text("N"),
              ],
            ),
            Instruction::new(
              "Open File or Folder",
              vec![
                InstructionKey::Icon(Icon::Key(Key::Cmd)),
                InstructionKey::Text("O"),
              ],
            ),
            Instruction::new(
              "Show All Commands",
              vec![
                InstructionKey::Icon(Icon::Key(Key::Shift)),
                InstructionKey::Icon(Icon::Key(Key::Cmd)),
                InstructionKey::Text("P"),
              ],
            ),
          ],
        ),
        InstructionSection::new(
          "VOICE CONTROLS",
          vec![
            Instruction::new(
              "Toggle And Speak",
              vec![InstructionKey::Icon(Icon::Voice)],
            ),
            Instruction::new(
              "Hold And Speak",
              vec![
                InstructionKey::Icon(Icon::Key(Key::Shift)),
                InstructionKey::Icon(Icon::Key(Key::Cmd)),
                InstructionKey::Icon(Icon::Key(Key::Space)),
              ],
            ),
            Instruction::new(
              "Show All Voice Commands",
              vec![
                InstructionKey::Icon(Icon::Key(Key::Shift)),
                InstructionKey::Icon(Icon::Key(Key::Cmd)),
                InstructionKey::Text("V"),
              ],
            ),
          ],
        ),
      ],
    }
  }
}

impl InstructionsResource {
  /// Creates a new instructions resource with default sections.
  pub fn new() -> Self {
    Self::default()
  }
}
