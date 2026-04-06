//! Playground resources.

use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;

use crate::previews::WebViewRect;

/// Resource holding playground metric entities.
#[derive(Resource, Default)]
pub struct PlaygroundMetrics {
  /// Output metric entity (bytes).
  pub output: Option<Entity>,
  /// Time metric entity (ms).
  pub time: Option<Entity>,
}

/// Feedback state for playground execution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FeedbackState {
  #[default]
  Ready,
  Running,
  Success,
}

/// Resource holding playground feedback state.
#[derive(Resource, Default)]
pub struct PlaygroundFeedback {
  pub state: FeedbackState,
}

/// Active output view in playground (Icon::Terminal vs Icon::Browser).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputViewKind {
  #[default]
  CompilerOutput,
  Webview,
}

/// Templating render target (web vs native).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TemplatingTarget {
  /// Render to webview using zo-runtime-web.
  #[default]
  Web,
  /// Render to native egui using zo-runtime-native.
  Native,
}

/// Resource holding playground compilation output.
#[derive(Resource, Default)]
pub struct PlaygroundOutput {
  /// Currently active view.
  pub active_view: OutputViewKind,
  /// Templating render target (web or native).
  pub templating_target: TemplatingTarget,
  /// Compilation state.
  pub compilation: CompilationState,
}

/// Compilation state with outputs per stage.
#[derive(Debug, Clone, Default)]
pub struct CompilationState {
  /// Is compilation in progress?
  pub is_compiling: bool,
  /// Token count (for metric display).
  pub token_count: usize,
  /// Node count (for metric display).
  pub node_count: usize,
  /// Instruction count (for metric display).
  pub insn_count: usize,
  /// Assembly byte count (for metric display).
  pub asm_bytes: usize,
  /// UI command count (for metric display).
  pub ui_count: usize,
  /// Elapsed time in milliseconds (for metric display).
  pub elapsed_time: f64,
  /// Tokens output (JSON).
  pub tokens: Option<String>,
  /// Tree output (JSON).
  pub tree: Option<String>,
  /// SIR output (JSON).
  pub sir: Option<String>,
  /// Assembly output (text).
  pub asm: Option<String>,
  /// UI commands output (JSON).
  pub ui: Option<String>,
}

/// Resource for hovered token span in playground.
///
/// When hovering a row in the tokens table, this stores the span
/// so the editor can highlight the corresponding lexeme.
#[derive(Resource, Default, Debug, Clone)]
pub struct PlaygroundHoveredSpan {
  /// Hovered span (start, end) in bytes.
  pub span: Option<(usize, usize)>,
}

/// Default preview URL for playground webview.
pub const PLAYGROUND_PREVIEW_URL: &str =
  "http://127.0.0.1:1337/preview/playground";

/// Resource for tracking playground webview state.
///
/// The actual WebView is stored outside ECS because wry::WebView is
/// !Send+!Sync.
#[derive(Resource, Default)]
pub struct PlaygroundWebviewState {
  /// Whether the webview is currently enabled/visible.
  pub enabled: bool,
  /// The rect where the WebView should be rendered.
  pub webview_rect: Option<WebViewRect>,
  /// Flag to indicate the WebView should reload (content changed).
  pub needs_reload: bool,
}
