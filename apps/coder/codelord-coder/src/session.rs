//! eframe-storage transport for session state.
//!
//! The actual `SessionState` DTO and its extract/restore ECS logic
//! live in [`codelord_core::session`]. This module only bridges to
//! eframe's persistence layer.

pub use codelord_core::session::{
  ExplorerSessionState, PanelState, SessionState, TabState, ThemeState,
};

/// Key used to store session state in eframe storage.
pub const SESSION_KEY: &str = "codelord_session_v1";

/// Clear the saved session from storage.
pub fn clear_session(storage: &mut dyn eframe::Storage) {
  storage.set_string(SESSION_KEY, String::new());

  log::info!("[Session] Cleared saved session");
}
