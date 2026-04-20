//! Stagebar component - Segmented tab navigation with animated highlight.
//!
//! A horizontal bar with clickable stages. The highlight pill slides
//! smoothly between selected stages using InOutSine easing.
//!
//! ECS Pattern:
//! - READS state from StagebarResource, DeltaTime, ThemeResource
//! - WRITES to StagebarResource (selection, animation)
//! - Uses ActiveAnimations for repaint tracking

use crate::assets::theme::get_theme;

use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::ActiveAnimations;
use codelord_core::ecs::world::World;
use codelord_core::navigation::resources::StagebarResource;

use eframe::egui;

/// Height of the stagebar.
const HEIGHT: f32 = 32.0;
/// Horizontal padding for each stage.
const STAGE_PADDING_X: f32 = 16.0;

/// Render the stagebar from the ECS world.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let theme = get_theme(world);

  // Theme colors
  let highlight_color = egui::Color32::from_rgba_unmultiplied(
    theme.primary[0],
    theme.primary[1],
    theme.primary[2],
    theme.primary[3],
  );
  let text_inactive = egui::Color32::from_rgba_unmultiplied(
    theme.subtext0[0],
    theme.subtext0[1],
    theme.subtext0[2],
    theme.subtext0[3],
  );
  let text_selected = egui::Color32::from_rgba_unmultiplied(
    theme.crust[0],
    theme.crust[1],
    theme.crust[2],
    255,
  );
  // Use same border color as popup (from visuals)
  let border_color = ui.visuals().widgets.noninteractive.bg_stroke.color;

  // Get delta time for animation
  let dt = world
    .get_resource::<DeltaTime>()
    .map(|d| d.delta())
    .unwrap_or(0.016);

  // Update animation and track transitions
  let animation_transition = world
    .get_resource_mut::<StagebarResource>()
    .map(|mut res| {
      res.update(dt);
      res.check_animation_transition()
    })
    .unwrap_or((false, false));

  // Track animation via ActiveAnimations
  if let Some(mut anims) = world.get_resource_mut::<ActiveAnimations>() {
    if animation_transition.0 {
      anims.increment();
    }
    if animation_transition.1 {
      anims.decrement();
    }
  }

  // Read resource for rendering
  let Some(stagebar) = world.get_resource::<StagebarResource>() else {
    return;
  };

  let stages = stagebar.stages.clone();
  let selected = stagebar.selected;
  let is_animating = stagebar.is_animating;
  let eased_progress = stagebar.eased_progress();
  let start_index = stagebar.start_index;
  let target_index = stagebar.target_index;
  let is_morphing = stagebar.is_morphing;
  let morph_progress = stagebar.eased_morph_progress();
  let morph_from_label = stagebar.morph_from_label;

  // Release borrow
  let _ = stagebar;

  if stages.is_empty() {
    return;
  }

  // Track click for later mutation
  let mut clicked_stage: Option<usize> = None;

  let font_id = egui::FontId::proportional(14.0);

  // First pass: calculate total width needed
  let mut total_width = 8.0; // 4px padding on each side
  for stage in &stages {
    let text_width = ui.fonts_mut(|f| {
      f.layout_no_wrap(stage.label.to_string(), font_id.clone(), text_inactive)
        .rect
        .width()
    });
    total_width += text_width + STAGE_PADDING_X * 2.0;
  }

  ui.horizontal(|ui| {
    ui.spacing_mut().item_spacing.x = 0.0;

    let (response, painter) = ui
      .allocate_painter(egui::vec2(total_width, HEIGHT), egui::Sense::hover());
    let rect = response.rect;

    // Border only (no background fill)
    painter.rect_stroke(
      rect,
      egui::CornerRadius::ZERO,
      egui::Stroke::new(1.0_f32, border_color),
      egui::StrokeKind::Inside,
    );

    // Calculate stage positions
    let mut stage_rects: Vec<egui::Rect> = Vec::with_capacity(stages.len());
    let mut x = rect.left() + 4.0;

    for stage in &stages {
      let text_width = ui.fonts_mut(|f| {
        f.layout_no_wrap(
          stage.label.to_string(),
          font_id.clone(),
          text_inactive,
        )
        .rect
        .width()
      });

      let stage_width = text_width + STAGE_PADDING_X * 2.0;
      let stage_rect = egui::Rect::from_min_size(
        egui::pos2(x, rect.top() + 2.0),
        egui::vec2(stage_width, HEIGHT - 4.0),
      );

      stage_rects.push(stage_rect);
      x += stage_width;
    }

    // Draw animated highlight pill
    if !stage_rects.is_empty() {
      let selected_rect = &stage_rects[selected.min(stage_rects.len() - 1)];

      let highlight_rect = if is_animating {
        // Interpolate between start and target positions
        let start_rect = &stage_rects[start_index.min(stage_rects.len() - 1)];
        let target_rect = &stage_rects[target_index.min(stage_rects.len() - 1)];

        let x = start_rect.left()
          + (target_rect.left() - start_rect.left()) * eased_progress;
        let width = start_rect.width()
          + (target_rect.width() - start_rect.width()) * eased_progress;

        egui::Rect::from_min_size(
          egui::pos2(x, selected_rect.top()),
          egui::vec2(width, selected_rect.height()),
        )
      } else {
        // No animation - use selected rect directly
        *selected_rect
      };

      painter.rect_filled(
        highlight_rect,
        egui::CornerRadius::ZERO,
        highlight_color,
      );
    }

    // Draw stage labels and handle clicks
    let last_index = stages.len().saturating_sub(1);

    for (i, (stage, stage_rect)) in
      stages.iter().zip(stage_rects.iter()).enumerate()
    {
      let is_selected = i == selected;
      let is_last = i == last_index;

      let base_color = if is_selected {
        text_selected
      } else {
        text_inactive
      };

      // Handle morph animation for last stage
      if is_last && is_morphing {
        // Fade out old label (moving up)
        let old_alpha = ((1.0 - morph_progress) * 255.0) as u8;
        let old_color = egui::Color32::from_rgba_unmultiplied(
          base_color.r(),
          base_color.g(),
          base_color.b(),
          old_alpha,
        );
        let old_offset = morph_progress * -8.0; // Slide up

        painter.text(
          stage_rect.center() + egui::vec2(0.0, old_offset),
          egui::Align2::CENTER_CENTER,
          morph_from_label,
          font_id.clone(),
          old_color,
        );

        // Fade in new label (moving up from below)
        let new_alpha = (morph_progress * 255.0) as u8;
        let new_color = egui::Color32::from_rgba_unmultiplied(
          base_color.r(),
          base_color.g(),
          base_color.b(),
          new_alpha,
        );
        let new_offset = (1.0 - morph_progress) * 8.0; // Start below, move to center

        painter.text(
          stage_rect.center() + egui::vec2(0.0, new_offset),
          egui::Align2::CENTER_CENTER,
          stage.label,
          font_id.clone(),
          new_color,
        );
      } else {
        painter.text(
          stage_rect.center(),
          egui::Align2::CENTER_CENTER,
          stage.label,
          font_id.clone(),
          base_color,
        );
      }

      // Click detection
      let click_response =
        ui.interact(*stage_rect, ui.id().with(stage.id), egui::Sense::click());

      if click_response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
      }

      if click_response.clicked() && !is_selected {
        clicked_stage = Some(i);
      }
    }
  });

  // Apply mutations after rendering
  if let Some(index) = clicked_stage
    && let Some(mut res) = world.get_resource_mut::<StagebarResource>()
  {
    res.select(index);
  }
}
