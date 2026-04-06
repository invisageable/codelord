//! Compilation protocol types for playground.

use serde::{Deserialize, Serialize};

/// Request to compile source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileRequest {
  /// Source code to compile.
  pub source: String,
  /// Target platform (e.g., "arm64-apple-darwin", "wasm32-unknown-unknown").
  pub target: String,
  /// Stage to compile up to (inclusive).
  pub stage: Stage,
}

/// Compilation events streamed via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompilationEvent {
  /// Compilation started.
  Started,
  /// Stage completed with output.
  Stage {
    stage: Stage,
    data: String,
    /// Elapsed time for this stage in milliseconds.
    elapsed_time: f64,
  },
  /// Compilation error.
  Error {
    message: String,
    span: Option<(u32, u32)>,
  },
  /// Compilation finished.
  Done { success: bool },
}

/// Compiler stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
  Tokens,
  Tree,
  Sir,
  /// Assembly output (Programming mode).
  Asm,
  /// UI commands rendered to HTML (Templating mode).
  Ui,
}
