//! Codeshow presentation events.

use bevy_ecs::component::Component;
use bevy_ecs::message::Message;

use std::path::PathBuf;

/// Message to load a presentation from a single file.
#[derive(Message, Debug, Clone)]
pub struct LoadPresentationFile {
  pub path: PathBuf,
}

/// Message to load a presentation from a directory.
#[derive(Message, Debug, Clone)]
pub struct LoadPresentationDirectory {
  pub path: PathBuf,
}

/// Component to navigate slides (spawned as entity, despawned after handling).
#[derive(Component, Debug, Clone, Copy)]
pub struct NavigateSlide {
  pub direction: SlideDirection,
}

#[derive(Debug, Clone, Copy)]
pub enum SlideDirection {
  Next,
  Previous,
  First,
  Last,
}
