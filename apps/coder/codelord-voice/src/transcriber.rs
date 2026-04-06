//! Whisper transcription engine.

use crate::error::{VoiceError, VoiceResult};

use whisper_rs::{
  FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters,
};

use std::path::PathBuf;

/// URL to download the Whisper base model.
pub const MODEL_URL: &str =
  "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";

/// Expected size of the model file in bytes (~148 MB).
pub const MODEL_SIZE: u64 = 147_951_465;

/// Whisper transcription engine.
pub struct VoiceTranscriber {
  context: WhisperContext,
}

impl VoiceTranscriber {
  /// Creates a new transcriber with the given model path.
  pub fn new(model_path: PathBuf) -> VoiceResult<Self> {
    log::info!("Loading Whisper model from: {}", model_path.display());

    let context = WhisperContext::new_with_params(
      model_path.to_str().ok_or_else(|| {
        VoiceError::ModelLoadError("Invalid model path".to_string())
      })?,
      WhisperContextParameters::default(),
    )
    .map_err(|e| VoiceError::ModelLoadError(e.to_string()))?;

    log::info!("Whisper model loaded successfully");

    Ok(Self { context })
  }

  /// Transcribe audio samples to text.
  pub fn transcribe(&mut self, audio: &[f32]) -> VoiceResult<String> {
    let audio_16khz = Self::resample_to_16khz(audio);

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    params.set_language(Some("en"));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    let mut state = self
      .context
      .create_state()
      .map_err(|e| VoiceError::TranscriptionError(e.to_string()))?;

    state
      .full(params, &audio_16khz)
      .map_err(|e| VoiceError::TranscriptionError(e.to_string()))?;

    let num_segments = state.full_n_segments();
    let mut result = String::new();

    for i in 0..num_segments {
      state
        .get_segment(i)
        .map(|segment| segment.to_str().ok().map(|text| result.push_str(text)));
    }

    let result = result.trim().to_string();

    log::info!("Transcription result: '{result}'");

    Ok(result)
  }

  fn resample_to_16khz(audio: &[f32]) -> Vec<f32> {
    const DOWNSAMPLE_FACTOR: usize = 3;

    audio.iter().step_by(DOWNSAMPLE_FACTOR).copied().collect()
  }
}

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

/// Checks if the model file exists.
pub fn model_exists() -> bool {
  model_path().exists()
}

/// Gets the models directory path.
pub fn models_dir() -> PathBuf {
  dirs::home_dir()
    .map(|home| home.join(".config").join("codelord").join("models"))
    .unwrap_or_else(|| PathBuf::from("."))
}
