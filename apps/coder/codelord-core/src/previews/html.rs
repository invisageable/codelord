//! HTML preview state resource.
//!
//! NOTE: The actual WebView (wry) is stored in the app struct, not in ECS,
//! because wry::WebView is not Send/Sync. This resource only tracks state.

use bevy_ecs::prelude::Resource;

use std::path::PathBuf;

/// Default preview URL for the local server.
pub const DEFAULT_PREVIEW_URL: &str = "http://127.0.0.1:1337/preview";

/// Resource for tracking HTML preview state (enabled flag only).
///
/// The actual WebView is stored outside ECS because wry::WebView is
/// !Send+!Sync.
#[derive(Resource, Default)]
pub struct HtmlPreviewState {
  /// Whether the preview is currently enabled/visible.
  pub enabled: bool,
  /// The rect where the WebView should be rendered (set by panel_right).
  pub webview_rect: Option<WebViewRect>,
  /// The file path currently being previewed.
  pub current_file: Option<PathBuf>,
  /// Flag to indicate the WebView should reload (file changed).
  pub needs_reload: bool,
}

/// Rectangle for WebView positioning (egui rect data without egui dependency).
#[derive(Clone, Copy, Debug, Default)]
pub struct WebViewRect {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
}
