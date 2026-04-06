//! Async file dialog utilities.
//!
//! Provides non-blocking file picker and save dialogs using tokio's
//! blocking thread pool.

use crate::runtime::RuntimeHandle;

use std::path::PathBuf;

/// Opens a file picker dialog asynchronously.
///
/// Uses tokio's blocking thread pool for the native dialog.
/// Returns a receiver that will contain the selected file path.
pub fn pick_file(
  handle: &RuntimeHandle,
  filters: &[(&str, &[&str])],
) -> flume::Receiver<Option<PathBuf>> {
  let (tx, rx) = flume::unbounded();

  let filters: Vec<(String, Vec<String>)> = filters
    .iter()
    .map(|(name, exts)| {
      (
        name.to_string(),
        exts.iter().map(|s| s.to_string()).collect(),
      )
    })
    .collect();

  handle.spawn_blocking(move || {
    let mut dialog = rfd::FileDialog::new();
    for (name, exts) in &filters {
      let ext_refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
      dialog = dialog.add_filter(name, &ext_refs);
    }
    if tx.send(dialog.pick_file()).is_err() {
      log::debug!("[Dialog] pick_file receiver dropped before completion");
    }
  });

  rx
}

/// Opens a folder picker dialog asynchronously.
///
/// Uses tokio's blocking thread pool for the native dialog.
/// Returns a receiver that will contain the selected folder path.
pub fn pick_folder(handle: &RuntimeHandle) -> flume::Receiver<Option<PathBuf>> {
  let (tx, rx) = flume::unbounded();

  handle.spawn_blocking(move || {
    if tx.send(rfd::FileDialog::new().pick_folder()).is_err() {
      log::debug!("[Dialog] pick_folder receiver dropped before completion");
    }
  });

  rx
}

/// Opens a save file dialog asynchronously.
///
/// Uses tokio's blocking thread pool for the native dialog.
/// Returns a receiver that will contain the selected save path.
pub fn save_file(
  handle: &RuntimeHandle,
  default_name: &str,
  filters: &[(&str, &[&str])],
) -> flume::Receiver<Option<PathBuf>> {
  let (tx, rx) = flume::unbounded();

  let default_name = default_name.to_string();
  let filters: Vec<(String, Vec<String>)> = filters
    .iter()
    .map(|(name, exts)| {
      (
        name.to_string(),
        exts.iter().map(|s| s.to_string()).collect(),
      )
    })
    .collect();

  handle.spawn_blocking(move || {
    let mut dialog = rfd::FileDialog::new().set_file_name(&default_name);
    for (name, exts) in &filters {
      let ext_refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
      dialog = dialog.add_filter(name, &ext_refs);
    }
    if tx.send(dialog.save_file()).is_err() {
      log::debug!("[Dialog] save_file receiver dropped before completion");
    }
  });

  rx
}
