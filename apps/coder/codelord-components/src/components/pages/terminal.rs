//! Terminal page - integrated terminal emulator.

pub mod bridge;
pub mod cursor_animation;
pub mod view;

pub use bridge::AlacrittyBridge;
pub use cursor_animation::{AnimatedCursor, CursorAnimationSettings};
pub use view::{show, show_content};
