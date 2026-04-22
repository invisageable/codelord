//! SDK for communicating with codelord-server (event-driven).
//!
//! Automatically spawns and manages the server process, providing async
//! methods for voice command interpretation and event streaming via WebSocket.
//!
//! **Event-Driven Voice Architecture:**
//! ```text
//! Client Side:
//!   1. connect_events() → WebSocket to /rpc/connect
//!   2. publish_voice_command() → HTTP POST /voice/interpret (fire-and-forget)
//!   3. Receive ServerEvent::Voice(Answer) via WebSocket
//!
//! Flow:
//!   publish_voice_command("toggle terminal")
//!     ↓ (HTTP POST)
//!   Server publishes Voice::Command event
//!     ↓ (event bus)
//!   Worker processes with OpenAI
//!     ↓ (event bus)
//!   Server publishes Voice::Answer event
//!     ↓ (WebSocket broadcast)
//!   SDK event_rx receives ServerEvent::Voice(Answer)
//!     ↓ (channel)
//!   Client handles VoiceAction
//! ```
//!
//! **Usage:**
//! ```rust,no_run
//! let sdk = Arc::new(Sdk::new(runtime.handle().clone()));
//!
//! // Connect to event stream
//! let events = sdk.connect_events().await?;
//!
//! // Publish voice command (non-blocking)
//! sdk.publish_voice_command("toggle terminal").await?;
//!
//! // Handle events (flume receiver)
//! while let Ok(event) = events.recv_async().await {
//!   if let ServerEvent::Voice(Voice::Answer(answer)) = event {
//!     // Process answer.action
//!   }
//! }
//! ```

use codelord_protocol::event::ServerEvent;
use codelord_protocol::voice::dto::InterpretRequest;

use flume::Receiver;
use futures::{SinkExt, StreamExt};
use reqwest::Client;
use tokio::runtime::Handle;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Client for the codelord-server with automatic process management.
///
/// Wraps HTTP communication and owns the spawned server process.
/// Use `Arc<Sdk>` to share across components.
pub struct Sdk {
  base_url: String,
  http_client: Client,
  server_process: Option<Child>,
  runtime: Handle,
}

impl Sdk {
  /// Creates SDK and spawns server if not already running.
  ///
  /// Reuses existing server on port 1337 if available, otherwise spawns
  /// a new process and waits up to 2 seconds for it to become ready.
  pub fn new(runtime: Handle) -> Self {
    let http_client = Client::builder()
      .connect_timeout(Duration::from_secs(1))
      .timeout(Duration::from_secs(2))
      .build()
      .expect("Failed to create HTTP client");

    let base_url = std::env::var("codelord_SERVER_URL")
      .unwrap_or_else(|_| "http://127.0.0.1:1337".to_string());

    let server_process =
      if Self::blocking_health_check(&base_url, &http_client, &runtime) {
        log::info!("[SDK] Server already running on port 1337");

        None
      } else {
        match Self::spawn_server() {
          Some(child) => {
            log::info!("[SDK] Server process spawned, waiting for ready...");

            if Self::wait_for_ready(
              &base_url,
              &http_client,
              &runtime,
              Duration::from_secs(2),
            ) {
              log::info!("[SDK] Server started successfully");

              Some(child)
            } else {
              log::warn!("[SDK] Server spawn timeout, killing process");

              None
            }
          }
          None => {
            log::warn!(
              "[SDK] Could not spawn server process - functionality limited"
            );

            None
          }
        }
      };

    Self {
      base_url,
      http_client,
      server_process,
      runtime,
    }
  }

  /// Attempts to spawn codelord-server from common locations.
  ///
  /// Searches target/debug, target/release, and PATH.
  fn spawn_server() -> Option<Child> {
    let exe_path = std::env::current_exe().ok()?;
    let exe_dir = exe_path.parent()?;
    let mut binary_paths = vec![exe_dir.join("codelord-server")];

    // Search opposite build profile (debug ↔ release)
    if let Some(target_dir) = exe_dir.parent() {
      if exe_dir.ends_with("debug") {
        binary_paths.push(target_dir.join("release").join("codelord-server"));
      } else if exe_dir.ends_with("release") {
        binary_paths.push(target_dir.join("debug").join("codelord-server"));
      }
    }

    binary_paths.push(PathBuf::from("codelord-server"));

    for binary in &binary_paths {
      log::debug!("[SDK] Trying to spawn server from: {}", binary.display());

      match Command::new(binary)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
      {
        Ok(child) => {
          log::info!("[SDK] Spawned server from: {}", binary.display());

          return Some(child);
        }
        Err(error) => {
          log::debug!(
            "[SDK] Failed to spawn from {}: {error}",
            binary.display()
          );

          continue;
        }
      }
    }

    log::error!("[SDK] Could not find codelord-server binary");
    log::error!("[SDK] Tried paths: {binary_paths:?}");

    None
  }

  /// Health check avoiding nested `block_on` calls.
  fn blocking_health_check(
    base_url: &str,
    client: &Client,
    runtime: &Handle,
  ) -> bool {
    let base_url = base_url.to_string();
    let client = client.clone();

    let handle = runtime.spawn(async move {
      match client
        .get(format!("{base_url}/health"))
        .timeout(Duration::from_secs(1))
        .send()
        .await
      {
        Ok(response) => match response.text().await {
          Ok(text) => text == "OK",
          Err(_) => false,
        },
        Err(_) => false,
      }
    });

    futures::executor::block_on(handle).unwrap_or(false)
  }

  /// Polls server health with exponential backoff until ready or timeout.
  fn wait_for_ready(
    base_url: &str,
    client: &Client,
    runtime: &Handle,
    timeout: Duration,
  ) -> bool {
    let start = Instant::now();
    let mut attempts = 0;

    while start.elapsed() < timeout {
      attempts += 1;

      if Self::blocking_health_check(base_url, client, runtime) {
        log::info!("[SDK] Server ready after {attempts} attempts");

        return true;
      }

      let wait = match attempts {
        1..=3 => Duration::from_millis(25),
        4..=6 => Duration::from_millis(50),
        7..=10 => Duration::from_millis(100),
        _ => Duration::from_millis(200),
      };

      std::thread::sleep(wait);
    }

    log::warn!("[SDK] Server health check timeout after {timeout:?}");

    false
  }

  /// Publishes a voice command to the server for asynchronous processing.
  ///
  /// The command is sent via HTTP and returns immediately (fire-and-forget).
  /// The actual VoiceAnswer will arrive via the event stream (WebSocket).
  /// Subscribe to events using `connect_events()` to receive answers.
  pub async fn publish_voice_command(&self, text: &str) -> Result<(), String> {
    let request = InterpretRequest {
      text: text.to_string(),
    };

    let response = self
      .http_client
      .post(format!("{}/voice/interpret", self.base_url))
      .json(&request)
      .send()
      .await
      .map_err(|error| format!("Failed to connect: {error}"))?;

    if response.status().is_success() {
      Ok(())
    } else {
      let status = response.status();

      let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());

      Err(format!("Server error {status}: {error_text}"))
    }
  }

  /// Connects to the server's event stream via WebSocket.
  ///
  /// Returns a channel receiver that yields ServerEvent messages.
  /// The WebSocket connection runs in a background task.
  pub async fn connect_events(&self) -> Result<Receiver<ServerEvent>, String> {
    let ws_url = self.base_url.replace("http://", "ws://");
    let url = format!("{ws_url}/rpc/connect");

    let (ws_stream, _) = connect_async(&url)
      .await
      .map_err(|e| format!("WebSocket connection failed: {e}"))?;

    let (mut write, mut read) = ws_stream.split();
    let (tx, rx) = flume::unbounded();

    // Spawn background task to handle WebSocket messages.
    tokio::spawn(async move {
      while let Some(msg) = read.next().await {
        match msg {
          Ok(Message::Text(text)) => {
            match sonic_rs::from_str::<ServerEvent>(&text) {
              Ok(event) => {
                if tx.send_async(event).await.is_err() {
                  log::debug!("[SDK] Event channel closed, stopping");

                  break;
                }
              }
              Err(e) => {
                log::warn!("[SDK] Failed to parse event: {e}");
              }
            }
          }
          Ok(Message::Close(_)) => {
            log::info!("[SDK] WebSocket closed by server");

            break;
          }
          Err(e) => {
            log::error!("[SDK] WebSocket error: {e}");

            break;
          }
          _ => {} // Ignore ping/pong/binary
        }
      }

      let _ = write.close().await;
    });

    Ok(rx)
  }

  /// Checks if server responds to health endpoint.
  pub async fn is_available(&self) -> bool {
    match self
      .http_client
      .get(format!("{}/health", self.base_url))
      .send()
      .await
    {
      Ok(response) => match response.text().await {
        Ok(text) => text == "OK",
        Err(_) => false,
      },
      Err(_) => false,
    }
  }

  /// Sends HTML preview file path to the server.
  ///
  /// The server will update its state to serve this file at /preview.
  pub fn send_html_preview_file(&self, file_path: String) {
    let http_client = self.http_client.clone();
    let base_url = self.base_url.clone();

    log::debug!("[SDK] Sending HTML preview file: {file_path}");

    self.runtime.spawn(async move {
      let url = format!("{base_url}/preview/set");

      log::debug!("[SDK] POST to {url}");

      match http_client
        .post(&url)
        .json(&sonic_rs::json!({ "file_path": file_path }))
        .send()
        .await
      {
        Ok(response) => {
          log::debug!("[SDK] Preview set response: {}", response.status());
        }
        Err(e) => {
          log::error!("[SDK] Failed to set preview file: {e}");
        }
      }
    });
  }

  /// Sends a compilation request to the server.
  ///
  /// The server will compile the source and stream events via WebSocket.
  /// `stage` picks the last stage to run (inclusive) — see
  /// [`codelord_protocol::compilation::Stage`].
  pub fn compile(
    &self,
    source: String,
    target: String,
    stage: codelord_protocol::compilation::Stage,
  ) {
    let http_client = self.http_client.clone();
    let base_url = self.base_url.clone();

    log::debug!("[SDK] Sending compile request (stage: {stage:?})");

    self.runtime.spawn(async move {
      let url = format!("{base_url}/playground/compile");

      let body = codelord_protocol::compilation::CompileRequest {
        source,
        target,
        stage,
      };

      match http_client.post(&url).json(&body).send().await {
        Ok(response) => {
          log::debug!("[SDK] Compile response: {}", response.status());
        }
        Err(e) => {
          log::error!("[SDK] Failed to send compile request: {e}");
        }
      }
    });
  }

  /// Terminates the spawned server process.
  pub fn shutdown(&mut self) {
    if let Some(mut child) = self.server_process.take() {
      log::info!("[SDK] Shutting down server...");

      if let Err(e) = child.kill() {
        log::error!("[SDK] Failed to send kill signal: {e}");

        return;
      }

      // Wait for the process to actually terminate.
      match child.wait() {
        Ok(status) => {
          log::info!("[SDK] Server exited with status: {status}");
        }
        Err(e) => {
          log::error!("[SDK] Error waiting for server to exit: {e}");
        }
      }
    }
  }
}

impl Drop for Sdk {
  fn drop(&mut self) {
    self.shutdown();
  }
}
