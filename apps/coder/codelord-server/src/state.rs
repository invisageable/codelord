use crate::automata::openai::OpenAIClient;

use codelord_protocol::event::ServerEvent;

use secrecy::SecretString;
use tokio::sync::Mutex;
use tokio::sync::broadcast::Sender;

/// Global server state shared across all routes and workers.
pub struct ServerState {
  /// Event bus for internal message broadcasting.
  pub event_bus: Sender<ServerEvent>,
  /// OpenAI client for AI completions.
  pub openai_client: OpenAIClient,
  /// Current HTML file path for preview.
  pub current_html_file: Mutex<String>,
  /// Rendered HTML from playground UI compilation.
  pub preview_html: Mutex<String>,
}

impl ServerState {
  /// Create a new [`ServerState`] instance.
  pub fn new() -> Self {
    let api_key = std::env::var("OPENAI_API_KEY")
      .expect("OPENAI_API_KEY environment variable must be set");

    Self {
      event_bus: Sender::new(100),
      openai_client: OpenAIClient::new(SecretString::from(api_key)),
      current_html_file: Mutex::new(String::new()),
      preview_html: Mutex::new(String::new()),
    }
  }
}

impl Default for ServerState {
  fn default() -> Self {
    Self::new()
  }
}
