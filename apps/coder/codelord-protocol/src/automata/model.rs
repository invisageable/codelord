use serde::{Deserialize, Serialize};

/// A message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
  pub role: String,
  pub content: String,
}

/// A choice returned by the chat completion API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
  pub message: Message,
}
