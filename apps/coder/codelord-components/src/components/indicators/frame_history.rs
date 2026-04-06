use eframe::egui;
use egui::mutex::Mutex;
use egui::util::History;

use std::sync::LazyLock;

static FRAME_HISTORY: LazyLock<Mutex<FrameHistory>> =
  LazyLock::new(|| Mutex::new(FrameHistory::default()));

struct FrameHistory {
  frame_times: History<f32>,
}

impl FrameHistory {
  fn on_new_frame(&mut self, now: f64, previous_frame_time: Option<f32>) {
    let previous_frame_time = previous_frame_time.unwrap_or_default();

    if let Some(latest) = self.frame_times.latest_mut() {
      *latest = previous_frame_time;
    }

    self.frame_times.add(now, previous_frame_time);
  }

  fn mean_frame_time(&self) -> f32 {
    self.frame_times.average().unwrap_or_default()
  }

  fn fps(&self) -> f32 {
    1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
  }
}

impl Default for FrameHistory {
  fn default() -> Self {
    let max_age: f32 = 1.0;
    let max_len = (max_age * 300.0).round() as usize;

    Self {
      frame_times: History::new(0..max_len, max_age),
    }
  }
}

pub fn record_frame_time(ctx: &egui::Context, frame: &mut eframe::Frame) {
  FRAME_HISTORY
    .lock()
    .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);
}

pub fn mean_frame_time() -> f32 {
  FRAME_HISTORY.lock().mean_frame_time()
}

pub fn fps() -> f32 {
  FRAME_HISTORY.lock().fps()
}

pub fn show(ui: &mut egui::Ui) {
  ui.label(egui::RichText::new(format!("FPS: {:.1}", fps())).size(10.0));
}
