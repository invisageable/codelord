//! Feedback indicator component.
//!
//! Displays an icon + label for status feedback.
//! States: ready (gray circle), running (blue spinner), success (green check).

use crate::assets::icon::icon_to_image;

use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::ecs::world::World;
use codelord_core::icon::components::{Feedback as FeedbackIcon, Icon};
use codelord_core::playground::{FeedbackState, PlaygroundFeedback};

use eframe::egui;

/// Get the label for a state.
fn label(state: FeedbackState) -> &'static str {
  match state {
    FeedbackState::Ready => "ready",
    FeedbackState::Running => "running",
    FeedbackState::Success => "success",
  }
}

/// Get the color for a state.
fn color(state: FeedbackState) -> egui::Color32 {
  match state {
    FeedbackState::Ready => egui::Color32::GRAY,
    FeedbackState::Running => egui::Color32::from_rgb(100, 149, 237),
    FeedbackState::Success => egui::Color32::from_rgb(76, 175, 80),
  }
}

/// Render feedback indicator with icon and label.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let state = world
    .get_resource::<PlaygroundFeedback>()
    .map(|f| f.state)
    .unwrap_or_default();

  let color = color(state);
  let label = label(state);

  ui.horizontal(|ui| {
    ui.spacing_mut().item_spacing.x = 4.0;

    match state {
      FeedbackState::Ready => {
        let (rect, _) =
          ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        let center = rect.center();
        ui.painter().circle_stroke(
          center,
          4.0,
          egui::Stroke::new(2.0_f32, color),
        );
      }
      FeedbackState::Running => {
        let (rect, _) =
          ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        let center = rect.center();
        let time = ui.ctx().input(|i| i.time) as f32;
        let angle = time * 4.0;

        let radius = 4.0;
        let stroke = egui::Stroke::new(2.0_f32, color);

        let start_angle = angle;
        let end_angle = angle + std::f32::consts::PI * 1.5;

        let points = (0..20)
          .map(|i| {
            let t = i as f32 / 19.0;
            let a = start_angle + t * (end_angle - start_angle);
            egui::pos2(center.x + radius * a.cos(), center.y + radius * a.sin())
          })
          .collect::<Vec<_>>();

        ui.painter().add(egui::Shape::line(points, stroke));

        if let Some(mut continuous) =
          world.get_resource_mut::<ContinuousAnimations>()
        {
          continuous.set_spinner_active();
        }
      }
      FeedbackState::Success => {
        ui.add(
          icon_to_image(&Icon::Feedback(FeedbackIcon::SuccessRounded))
            .fit_to_exact_size(egui::vec2(16.0, 16.0))
            .tint(color),
        );
      }
    }

    ui.label(egui::RichText::new(label).color(color).size(12.0));
  });
}
