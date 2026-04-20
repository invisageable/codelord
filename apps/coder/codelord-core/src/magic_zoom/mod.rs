//! Screen-Studio-style magic zoom.
//!
//! Hold a hotkey: content smoothly zooms toward the cursor. Release: eases
//! back. Applied as an egui `TSTransform` on a dedicated layer — no
//! per-frame allocations, no font re-rasterization (v1 accepts text blur
//! during motion).

pub mod messages;
pub mod resources;
pub mod systems;

pub use messages::MagicZoomCommand;
pub use resources::MagicZoomState;
pub use systems::update_magic_zoom_system;
