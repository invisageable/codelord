//! Background worker for processing voice commands (event-driven).
//!
//! **Architecture Flow:**
//! ```text
//! event_bus.subscribe()
//!   ↓
//! Receive ServerEvent::Voice(Command)
//!   ↓
//! Call OpenAI API (interpret command)
//!   ↓
//! Parse response to VoiceAction
//!   ↓
//! Publish ServerEvent::Voice(Answer)
//!   ↓
//! event_bus broadcasts to all subscribers
//! ```
//!
//! **Note:** This worker runs as a background task spawned in main.rs.
//! It processes commands asynchronously without blocking the HTTP server.

use crate::state::ServerState;

use codelord_protocol::automata::dto::ChatCompletionRequest;
use codelord_protocol::automata::model::Message;
use codelord_protocol::event::ServerEvent;
use codelord_protocol::voice::model::{Voice, VoiceAction, VoiceAnswer};

use std::sync::Arc;

/// Background worker that processes voice command events.
///
/// Subscribes to ServerEvent::Voice(Command) events, calls OpenAI
/// to interpret the command, and publishes VoiceAnswer events back.
pub async fn run(state: Arc<ServerState>) {
  tracing::info!("[VoiceWorker] Starting voice command processor");

  let mut event_rx = state.event_bus.subscribe();
  let system_prompt = include_str!("../../prompts/commands.md");

  loop {
    match event_rx.recv().await {
      Ok(ServerEvent::Voice(Voice::Command(command))) => {
        tracing::debug!("[VoiceWorker] Processing command: {}", command.text);

        // Call OpenAI to interpret the command.
        let request = ChatCompletionRequest {
          model: "gpt-4o-mini".into(),
          messages: vec![
            Message {
              role: "system".into(),
              content: system_prompt.trim().into(),
            },
            Message {
              role: "user".into(),
              content: command.text.clone(),
            },
          ],
          temperature: 1.0,
          max_tokens: 100,
        };

        let action = match state.openai_client.complete(request).await {
          Ok(response) => {
            let response_text = &response.choices[0].message.content;
            let cleaned = clean_response(response_text);

            match sonic_rs::from_str::<VoiceAction>(&cleaned) {
              Ok(action) => action,
              Err(e) => {
                tracing::error!(
                  "[VoiceWorker] Failed to parse action: {e}, response: {cleaned}"
                );
                VoiceAction {
                  action: "Unknown".into(),
                  payload: None,
                }
              }
            }
          }
          Err(e) => {
            tracing::error!("[VoiceWorker] OpenAI error: {e}");
            VoiceAction {
              action: "Unknown".into(),
              payload: None,
            }
          }
        };

        let answer = VoiceAnswer { action };

        if let Err(e) = state
          .event_bus
          .send(ServerEvent::Voice(Voice::Answer(answer)))
        {
          tracing::error!("[VoiceWorker] Failed to publish answer: {e}");
        }
      }
      Ok(_) => {} // Ignore other events.
      Err(e) => {
        tracing::error!("[VoiceWorker] Event bus error: {e}");
        break;
      }
    }
  }
}

/// Cleans the AI response by removing markdown code blocks and whitespace.
///
/// Handles any markdown code fence format: ```lang or just ```.
/// Works regardless of language specifier (json, rust, etc.).
fn clean_response(response: &str) -> String {
  let trimmed = response.trim();

  if trimmed.starts_with("```") && trimmed.ends_with("```") {
    let first_newline = trimmed.find('\n').unwrap_or(trimmed.len());
    let last_newline = trimmed.rfind('\n').unwrap_or(0);

    if first_newline < last_newline {
      trimmed[first_newline + 1..last_newline].trim().to_string()
    } else {
      trimmed.to_string()
    }
  } else {
    trimmed.to_string()
  }
}
