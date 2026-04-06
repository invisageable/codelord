use serde::{Deserialize, Serialize};

use crate::automata::model::{Choice, Message};

/// Request to complete a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
  pub model: String,
  pub messages: Vec<Message>,
  pub temperature: f32,
  pub max_tokens: u32,
}

/// Response from a chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
  pub choices: Vec<Choice>,
}
