use serde::{Deserialize, Serialize};

/// Voice service events (Command or Answer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Voice {
  Command(VoiceCommand),
  Answer(VoiceAnswer),
}

/// Voice command from user (transcribed speech).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
  pub text: String,
}

/// Voice answer from AI (interpreted action).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceAnswer {
  pub action: VoiceAction,
}

/// Action to execute from voice command.
/// This is the output from the AI command interpreter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceAction {
  pub action: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub payload: Option<String>,
}
