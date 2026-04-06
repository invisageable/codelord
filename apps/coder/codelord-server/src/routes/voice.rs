//! Voice command interpretation endpoints (event-driven).
//!
//! **Architecture Flow:**
//! ```text
//! POST /voice/interpret (fire-and-forget)
//!   ↓
//! Publish ServerEvent::Voice(Command)
//!   ↓
//! event_bus (broadcast channel)
//!   ↓
//! VoiceWorker processes with OpenAI
//!   ↓
//! Publish ServerEvent::Voice(Answer)
//!   ↓
//! RPC WebSocket broadcasts to clients
//!   ↓
//! Client receives answer via event stream
//! ```
//!
//! **Note:** This endpoint returns immediately (202 Accepted).
//! The actual VoiceAnswer arrives via WebSocket (`/rpc/connect`).

use crate::state::ServerState;

use codelord_protocol::event::ServerEvent;
use codelord_protocol::voice::dto::{InterpretRequest, InterpretResponse};
use codelord_protocol::voice::model::{Voice, VoiceCommand};

use axum::extract::State;
use axum::{Json, Router, routing};

use std::sync::Arc;

pub fn router(state: Arc<ServerState>) -> Router {
  Router::new()
    .route("/interpret", routing::post(interpret))
    .with_state(state)
}

/// POST /voice/interpret
///
/// Accepts a voice command text, publishes it to the event bus,
/// and returns immediately. The actual answer will be broadcast
/// via WebSocket to all connected clients.
async fn interpret(
  State(state): State<Arc<ServerState>>,
  Json(request): Json<InterpretRequest>,
) -> Json<InterpretResponse> {
  let command = VoiceCommand {
    text: request.text.clone(),
  };

  if let Err(e) = state
    .event_bus
    .send(ServerEvent::Voice(Voice::Command(command)))
  {
    tracing::error!("[Voice] Failed to publish command event: {e}");
    return Json(InterpretResponse {
      success: false,
      action: None,
      error: Some("Failed to process voice command".to_string()),
    });
  }

  Json(InterpretResponse {
    success: true,
    action: None,
    error: None,
  })
}
