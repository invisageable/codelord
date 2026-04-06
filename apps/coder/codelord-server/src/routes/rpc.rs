//! RPC WebSocket endpoint for event streaming.
//!
//! Clients connect via WebSocket to receive all ServerEvents
//! broadcast on the event bus.

use crate::state::ServerState;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::{Router, routing};

use std::sync::Arc;

pub fn router(state: Arc<ServerState>) -> Router {
  Router::new()
    .route("/connect", routing::any(handle_websocket))
    .with_state(state)
}

/// Handle WebSocket upgrade for RPC connection.
async fn handle_websocket(
  ws: WebSocketUpgrade,
  State(state): State<Arc<ServerState>>,
) -> Response {
  ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection.
///
/// Subscribes to the event bus and forwards all events
/// to the connected client as JSON.
async fn handle_socket(mut socket: WebSocket, state: Arc<ServerState>) {
  tracing::info!("[RPC] Client connected");

  // Subscribe to server events.
  let mut event_rx = state.event_bus.subscribe();

  loop {
    tokio::select! {
      // Receive message from client.
      msg = socket.recv() => {
        match msg {
          Some(Ok(Message::Text(text))) => {
            tracing::debug!("[RPC] Received: {text}");
            // Echo back for now (can be extended for RPC calls).
            if socket.send(Message::Text(format!("Echo: {text}").into())).await.is_err() {
              break;
            }
          }
          Some(Ok(Message::Close(_))) | None => {
            tracing::info!("[RPC] Client disconnected");
            break;
          }
          Some(Ok(_)) => {} // Ignore other message types.
          Some(Err(e)) => {
            tracing::error!("[RPC] WebSocket error: {e}");
            break;
          }
        }
      }

      // Broadcast server events to client.
      event = event_rx.recv() => {
        match event {
          Ok(event) => {
            let json = sonic_rs::to_string(&event).unwrap();
            if socket.send(Message::Text(json.into())).await.is_err() {
              break;
            }
          }
          Err(_) => break,
        }
      }
    }
  }

  tracing::info!("[RPC] Connection closed");
}
