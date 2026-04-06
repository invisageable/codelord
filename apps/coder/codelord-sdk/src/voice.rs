//! Voice model download and management.
//!
//! Handles downloading the Whisper model for voice transcription.

use futures::StreamExt;

use std::io::Write;
use std::path::PathBuf;

/// URL to download the Whisper base model.
pub const MODEL_URL: &str =
  "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";

/// Expected size of the model file in bytes (~148 MB).
pub const MODEL_SIZE: u64 = 147_951_465;

/// Gets the default model path.
pub fn model_path() -> PathBuf {
  dirs::home_dir()
    .map(|home| {
      home
        .join(".config")
        .join("codelord")
        .join("models")
        .join("ggml-base.bin")
    })
    .unwrap_or_else(|| PathBuf::from("ggml-base.bin"))
}

/// Gets the models directory path.
pub fn models_dir() -> PathBuf {
  dirs::home_dir()
    .map(|home| home.join(".config").join("codelord").join("models"))
    .unwrap_or_else(|| PathBuf::from("."))
}

/// Checks if the model file exists.
pub fn model_exists() -> bool {
  model_path().exists()
}

/// Download progress update.
#[derive(Debug, Clone, Copy)]
pub struct DownloadProgress {
  /// Bytes downloaded so far.
  pub downloaded: u64,
  /// Total bytes to download.
  pub total: u64,
  /// Progress as fraction (0.0 to 1.0).
  pub fraction: f32,
}

/// Downloads the Whisper model to the default location.
///
/// Sends progress updates via the provided channel.
/// Returns the path where the model was saved.
pub async fn download_model(
  progress_tx: flume::Sender<DownloadProgress>,
) -> Result<PathBuf, String> {
  let path = model_path();

  // Create directory if needed
  let dir = models_dir();
  std::fs::create_dir_all(&dir)
    .map_err(|e| format!("Failed to create models directory: {e}"))?;

  log::info!("Downloading Whisper model to: {}", path.display());
  log::info!("URL: {MODEL_URL}");

  // Download with progress
  let client = reqwest::Client::new();
  let response = client
    .get(MODEL_URL)
    .send()
    .await
    .map_err(|e| format!("Failed to start download: {e}"))?;

  if !response.status().is_success() {
    return Err(format!("Download failed: HTTP {}", response.status()));
  }

  let total = response.content_length().unwrap_or(MODEL_SIZE);

  // Create temp file first, rename on success
  let temp_path = path.with_extension("bin.tmp");
  let mut file = std::fs::File::create(&temp_path)
    .map_err(|e| format!("Failed to create file: {e}"))?;

  let mut downloaded: u64 = 0;
  let mut stream = response.bytes_stream();

  while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|e| format!("Download error: {e}"))?;

    file
      .write_all(&chunk)
      .map_err(|e| format!("Write error: {e}"))?;

    downloaded += chunk.len() as u64;
    let fraction = downloaded as f32 / total as f32;

    let _ = progress_tx.send(DownloadProgress {
      downloaded,
      total,
      fraction,
    });
  }

  // Flush and close file
  file.flush().map_err(|e| format!("Flush error: {e}"))?;
  drop(file);

  // Rename temp file to final path
  std::fs::rename(&temp_path, &path)
    .map_err(|e| format!("Failed to finalize download: {e}"))?;

  log::info!("Whisper model download complete: {}", path.display());

  Ok(path)
}

/// Spawns model download in background, returns immediately.
///
/// Progress and completion are communicated via the returned channel.
pub fn spawn_download() -> flume::Receiver<DownloadResult> {
  log::info!("[Voice] spawn_download called");

  let (result_tx, result_rx) = flume::unbounded();

  std::thread::spawn(move || {
    log::info!("[Voice] Download thread started");

    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("Failed to create tokio runtime");

    let (progress_tx, progress_rx) = flume::unbounded();

    // Forward progress updates
    let result_tx_progress = result_tx.clone();
    std::thread::spawn(move || {
      while let Ok(progress) = progress_rx.recv() {
        let _ = result_tx_progress.send(DownloadResult::Progress(progress));
      }
    });

    // Run async download
    let result = runtime.block_on(download_model(progress_tx));

    match result {
      Ok(path) => {
        let _ = result_tx.send(DownloadResult::Complete(path));
      }
      Err(e) => {
        let _ = result_tx.send(DownloadResult::Error(e));
      }
    }
  });

  result_rx
}

/// Result from download operation.
#[derive(Debug, Clone)]
pub enum DownloadResult {
  /// Progress update during download.
  Progress(DownloadProgress),
  /// Download completed successfully.
  Complete(PathBuf),
  /// Download failed with error.
  Error(String),
}
