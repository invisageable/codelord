use crate::compilation::CompilationEvent;
use crate::voice::model::Voice;

use serde::{Deserialize, Serialize};

/// Server events broadcast to all connected clients via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerEvent {
  Voice(Voice),
  Compilation(CompilationEvent),
}
