//! 1 px separator line that doubles as a voice / global-loading
//! progress indicator.
//!
//! Pure egui + ECS reads: pulls state from [`VoiceResource`] and
//! [`GlobalLoading`], paints a colored bar on top of the base
//! separator line when voice is processing, a full green bar on
//! success, a full red bar on error, and an exponential-ease progress
//! bar while a long-running task is loading.

use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::ecs::world::World;
use codelord_core::loading::GlobalLoading;
use codelord_core::voice::resources::{VisualizerStatus, VoiceResource};

use eframe::egui;

const GREEN_100: egui::Color32 = egui::Color32::from_rgb(204, 253, 62);
const GREEN_200: egui::Color32 = egui::Color32::from_rgb(6, 208, 1);
const RED_100: egui::Color32 = egui::Color32::from_rgb(221, 3, 3);

/// Render the base separator line plus any active voice/loading
/// progress overlay.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let rect = ui.max_rect();
  let separator_y = rect.bottom() - 1.0;

  // Base separator line (always visible).
  ui.painter().line_segment(
    [
      egui::pos2(rect.left(), separator_y),
      egui::pos2(rect.right(), separator_y),
    ],
    egui::Stroke::new(1.0_f32, egui::Color32::from_gray(30)),
  );

  let (status, processing_start_time) = world
    .get_resource::<VoiceResource>()
    .map(|v| (v.visualizer_status, v.processing_start_time))
    .unwrap_or((VisualizerStatus::Idle, 0));

  let (is_global_loading, is_global_completed, loading_start_time) = world
    .get_resource::<GlobalLoading>()
    .map(|l| (l.is_loading(), l.is_completed(), l.start_time))
    .unwrap_or((false, false, 0));

  match status {
    VisualizerStatus::Processing => {
      let progress_width =
        rect.width() * exponential_progress(processing_start_time);

      let progress_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), separator_y),
        egui::vec2(progress_width, 2.0),
      );

      ui.painter().rect_filled(progress_rect, 0.0, GREEN_100);
    }
    VisualizerStatus::Success => {
      paint_full_bar(ui, rect, separator_y, GREEN_200);
    }
    VisualizerStatus::Error => {
      paint_full_bar(ui, rect, separator_y, RED_100);
    }
    _ if is_global_completed => {
      paint_full_bar(ui, rect, separator_y, GREEN_200);

      if let Some(mut animations) =
        world.get_resource_mut::<ContinuousAnimations>()
      {
        animations.set_loading_bar_active();
      }
    }
    _ if is_global_loading => {
      let progress_width =
        rect.width() * exponential_progress(loading_start_time);

      let progress_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), separator_y),
        egui::vec2(progress_width, 2.0),
      );

      ui.painter().rect_filled(progress_rect, 0.0, GREEN_100);

      if let Some(mut animations) =
        world.get_resource_mut::<ContinuousAnimations>()
      {
        animations.set_loading_bar_active();
      }
    }
    _ => {}
  }
}

fn paint_full_bar(
  ui: &egui::Ui,
  rect: egui::Rect,
  separator_y: f32,
  color: egui::Color32,
) {
  let progress_rect = egui::Rect::from_min_size(
    egui::pos2(rect.left(), separator_y),
    egui::vec2(rect.width(), 2.0),
  );

  ui.painter().rect_filled(progress_rect, 0.0, color);
}

/// Exponential ease-out from 0 to 0.95 based on elapsed time since
/// `start_time_ms` (ms since epoch).
fn exponential_progress(start_time_ms: u64) -> f32 {
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

  let elapsed_secs = now.saturating_sub(start_time_ms) as f32 / 1000.0;

  const K: f32 = 0.5;

  (1.0 - (-K * elapsed_secs).exp()).min(0.95)
}
