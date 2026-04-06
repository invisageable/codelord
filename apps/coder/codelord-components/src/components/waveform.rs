//! Real-time audio frequency spectrum visualization component
//!
//! Displays a frequency spectrum visualizer using FFT analysis.
//! Bars are mirrored around center: bass in middle, treble on edges.

use codelord_audio::visualizer::WAVEFORM_SAMPLES;

use eframe::egui;
use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

/// Lime green color for music player waveform.
pub const COLOR_MUSIC: egui::Color32 = egui::Color32::from_rgb(204, 253, 62);

/// Blue color for voice input waveform.
pub const COLOR_VOICE: egui::Color32 = egui::Color32::from_rgb(100, 150, 255);

/// Render music player waveform (lime green).
pub fn show(ui: &mut egui::Ui, samples: &[f32]) {
  show_with_color(ui, samples, COLOR_MUSIC);
}

/// Render voice input waveform (blue).
pub fn show_voice(ui: &mut egui::Ui, samples: &[f32]) {
  show_with_color(ui, samples, COLOR_VOICE);
}

/// Render waveform with custom color.
pub fn show_with_color(
  ui: &mut egui::Ui,
  samples: &[f32],
  color: egui::Color32,
) {
  let size = egui::vec2(120.0, 24.0);
  let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
  let painter = ui.painter();

  // Background
  painter.rect_filled(rect, 0.0, egui::Color32::TRANSPARENT);

  // Draw center line coordinates.
  let center_y = rect.center().y;
  let center_x = rect.center().x;
  let num_bars = 32;
  let bar_width = 1.0;
  let half_width = rect.width() / 2.0;
  let bar_spacing = half_width / num_bars as f32;
  let max_height = rect.height() / 2.0 - 2.0;
  let min_bar_height = 0.05;

  if samples.len() < WAVEFORM_SAMPLES {
    // Not enough samples yet - show idle state (line of dots).
    for i in 0..num_bars {
      let x_offset = i as f32 * bar_spacing + bar_spacing / 2.0;

      // Right side.
      painter.line_segment(
        [
          egui::pos2(center_x + x_offset, center_y - min_bar_height),
          egui::pos2(center_x + x_offset, center_y + min_bar_height),
        ],
        egui::Stroke::new(bar_width, color),
      );

      // Left side (mirrored).
      painter.line_segment(
        [
          egui::pos2(center_x - x_offset, center_y - min_bar_height),
          egui::pos2(center_x - x_offset, center_y + min_bar_height),
        ],
        egui::Stroke::new(bar_width, color),
      );
    }

    return;
  }

  // ===== FFT PROCESSING =====

  // 1. Apply Hann window and prepare FFT input
  let mut planner = FftPlanner::new();
  let fft = planner.plan_fft_forward(WAVEFORM_SAMPLES);
  let mut buffer: Vec<Complex<f32>> = Vec::with_capacity(WAVEFORM_SAMPLES);

  for (i, sample) in samples.iter().enumerate().take(WAVEFORM_SAMPLES) {
    // Hann window reduces FFT artifacts
    let hann = 0.5
      * (1.0
        - (2.0 * std::f32::consts::PI * i as f32
          / (WAVEFORM_SAMPLES - 1) as f32)
          .cos());

    buffer.push(Complex::new(sample * hann, 0.0));
  }

  // 2. Perform FFT
  fft.process(&mut buffer);

  // 3. Extract frequency bins (only first half - Nyquist)
  let visual_bins = &buffer[0..WAVEFORM_SAMPLES / 2];

  // 4. Group bins into bars for visualization
  let num_bars = 32;

  let bins_per_bar =
    (visual_bins.len() as f32 / num_bars as f32).floor() as usize;

  let mut frequency_bands: Vec<f32> = Vec::with_capacity(num_bars);

  for i in 0..num_bars {
    let start = i * bins_per_bar;
    let end = ((i + 1) * bins_per_bar).min(visual_bins.len());

    let mut magnitude_sum = 0.0;

    for bin in &visual_bins[start..end] {
      magnitude_sum += bin.norm(); // Magnitude of complex number
    }

    let avg_magnitude = magnitude_sum / (end - start) as f32;

    // Logarithmic scaling for better visualization
    let scaled = (avg_magnitude * 50.0).log10().max(0.0) * 0.5;

    frequency_bands.push(scaled.clamp(0.0, 1.0));
  }

  // Draw mirrored frequency spectrum.
  for (i, amplitude) in frequency_bands.iter().enumerate().take(num_bars) {
    let bar_height = (amplitude * max_height).max(min_bar_height);

    // x_offset from center
    let x_offset = i as f32 * bar_spacing + bar_spacing / 2.0;

    // Draw bar: Right side.
    painter.line_segment(
      [
        egui::pos2(center_x + x_offset, center_y - bar_height),
        egui::pos2(center_x + x_offset, center_y + bar_height),
      ],
      egui::Stroke::new(bar_width, color),
    );

    // Left side (mirrored).
    painter.line_segment(
      [
        egui::pos2(center_x - x_offset, center_y - bar_height),
        egui::pos2(center_x - x_offset, center_y + bar_height),
      ],
      egui::Stroke::new(bar_width, color),
    );
  }

  // Animation tracking is handled by ECS ContinuousAnimations system
}
