//! Converts transcribed voice text into executable actions (event-driven).
//!
//! **Architecture Flow:**
//! ```text
//! Dispatcher Setup (in new()):
//!   SDK.connect_events()
//!     ↓
//!   Spawn event listener task
//!     ↓
//!   Listen for ServerEvent::Voice(Answer)
//!     ↓
//!   Send VoiceAction via channel
//!
//! Voice Command Flow (in dispatch_voice_text()):
//!   User speaks → transcribed text
//!     ↓
//!   SDK.publish_voice_command() (fire-and-forget)
//!     ↓ (returns immediately)
//!   [Answer arrives later via event stream]
//!
//! Fallback Flow (if server unavailable):
//!   User speaks → transcribed text
//!     ↓
//!   Regex parser (local, synchronous)
//!     ↓
//!   Action executed immediately
//! ```

use crate::parser::VoiceIntentParser;
use crate::visualizer::{VisualizerStatus, VoiceVisualizerState};

use codelord_protocol::event::ServerEvent;
use codelord_protocol::voice::model::{Voice, VoiceAction};
use codelord_sdk::Sdk;

use std::sync::Arc;
use std::time::Duration;

/// Routes voice commands to AI interpretation or regex fallback.
pub struct VoiceDispatcher {
  action_sender: flume::Sender<VoiceAction>,
  sdk: Option<Arc<Sdk>>,
  intent_parser: VoiceIntentParser,
  runtime_handle: tokio::runtime::Handle,
  visualizer_state: VoiceVisualizerState,
}

impl VoiceDispatcher {
  /// Creates dispatcher and connects to event stream.
  pub fn new(
    action_sender: flume::Sender<VoiceAction>,
    runtime_handle: tokio::runtime::Handle,
    visualizer_state: VoiceVisualizerState,
    sdk: Arc<Sdk>,
  ) -> Self {
    let sdk_clone = Arc::clone(&sdk);

    let handle =
      runtime_handle.spawn(async move { sdk_clone.is_available().await });

    let sdk_available = futures::executor::block_on(handle).unwrap_or(false);

    let sdk = if sdk_available {
      log::info!("[Voice] Server detected - using AI command interpretation");

      let sdk_clone = Arc::clone(&sdk);
      let action_sender_clone = action_sender.clone();
      let visualizer_clone = visualizer_state.clone();

      runtime_handle.spawn(async move {
        match sdk_clone.connect_events().await {
          Ok(event_rx) => {
            log::info!("[Voice] Connected to event stream");

            while let Ok(event) = event_rx.recv_async().await {
              if let ServerEvent::Voice(Voice::Answer(answer)) = event {
                log::debug!("[Voice] Received answer: {:?}", answer.action);

                // Check if action is Unknown - treat as error
                if answer.action.action == "Unknown" {
                  log::warn!("[Voice] Server returned Unknown action");
                  visualizer_clone.set_status(VisualizerStatus::Error);
                  tokio::time::sleep(Duration::from_millis(300)).await;
                  visualizer_clone.set_status(VisualizerStatus::Idle);
                } else {
                  visualizer_clone.set_status(VisualizerStatus::Success);
                  let _ = action_sender_clone.send_async(answer.action).await;
                  tokio::time::sleep(Duration::from_millis(300)).await;
                  visualizer_clone.set_status(VisualizerStatus::Idle);
                }
              }
            }

            log::warn!("[Voice] Event stream closed");
          }
          Err(e) => {
            log::error!("[Voice] Failed to connect to events: {e}");
          }
        }
      });

      Some(sdk)
    } else {
      log::warn!("[Voice] Server not available - falling back to regex parser");
      None
    };

    Self {
      action_sender,
      sdk,
      intent_parser: VoiceIntentParser::new(),
      runtime_handle,
      visualizer_state,
    }
  }

  /// Sends voice text for async interpretation (non-blocking).
  ///
  /// Publishes command to server if available, otherwise uses regex parser.
  pub fn dispatch_voice_text(&self, text: &str) {
    let text = text.to_string();
    let action_sender = self.action_sender.clone();
    let sdk = self.sdk.clone();
    let intent_parser = self.intent_parser.clone();
    let visualizer_state = self.visualizer_state.clone();

    self.runtime_handle.spawn(async move {
      // Publish to AI server if available (answer arrives via WebSocket).
      let server_handled = match sdk {
        Some(sdk) => match sdk.publish_voice_command(&text).await {
          Ok(_) => {
            log::info!("[Voice] Published command: '{text}'");
            true
          }
          Err(error) => {
            log::warn!("Server error ({error}), falling back to regex");
            false
          }
        },
        None => false,
      };

      if server_handled {
        return;
      }

      // Fallback to regex parser if server unavailable or error.
      match intent_parser.parse(&text) {
        Some(action) => {
          log::info!("[Voice] Regex matched: '{text}' -> {action:?}");

          let _ = action_sender.send_async(action).await;

          visualizer_state.set_status(VisualizerStatus::Success);
          tokio::time::sleep(Duration::from_millis(300)).await;
          visualizer_state.set_status(VisualizerStatus::Idle);
        }
        None => {
          log::warn!("[Voice] Command not recognized: '{text}'");

          visualizer_state.set_status(VisualizerStatus::Error);
          tokio::time::sleep(Duration::from_millis(300)).await;
          visualizer_state.set_status(VisualizerStatus::Idle);
        }
      }
    });
  }

  /// Check if server is available.
  pub fn is_server_available(&self) -> bool {
    self.sdk.is_some()
  }
}

impl Clone for VoiceDispatcher {
  fn clone(&self) -> Self {
    Self {
      action_sender: self.action_sender.clone(),
      sdk: self.sdk.clone(),
      intent_parser: VoiceIntentParser::new(),
      runtime_handle: self.runtime_handle.clone(),
      visualizer_state: self.visualizer_state.clone(),
    }
  }
}
