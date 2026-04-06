//! Codeshow - presenter slider for markdown-based presentations.

pub mod events;
pub mod resources;

pub use events::{
  LoadPresentationDirectory, LoadPresentationFile, NavigateSlide,
  SlideDirection,
};
pub use resources::{
  CodeshowState, PendingPresentationDirectory, PendingPresentationFile,
  SlideTransition,
};
