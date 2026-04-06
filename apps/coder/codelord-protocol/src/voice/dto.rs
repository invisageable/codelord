use serde::{Deserialize, Serialize};

use super::model::VoiceAction;

/// Request to interpret a voice command.
#[derive(Debug, Serialize, Deserialize)]
pub struct InterpretRequest {
  pub text: String,
}

/// Response from interpret request (acknowledgement only).
/// Actual answer comes via WebSocket.
#[derive(Debug, Serialize, Deserialize)]
pub struct InterpretResponse {
  pub success: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub action: Option<VoiceAction>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}
