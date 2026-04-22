//! Magic-zoom camera: egui-side helpers.
//!
//! The ECS state (zoom factor, camera center, engagement) lives in
//! [`codelord_core::magic_zoom::MagicZoomState`]. This module wraps the
//! egui bits — the sublayer id the main body renders into, the
//! `TSTransform` built from the state, and the per-frame transform
//! propagation to overlay layers (filescope, popups, dialogs, toasts)
//! that render as top-level `egui::Area`s and would otherwise stay at
//! 1x while the main body zooms.

use codelord_core::ecs::world::World;
use codelord_core::magic_zoom::MagicZoomState;

use eframe::egui;

/// Layer id of the magic-zoom sublayer wrapping the main app body.
pub fn layer_id() -> egui::LayerId {
  egui::LayerId::new(egui::Order::Middle, egui::Id::new("magic_zoom_layer"))
}

/// Layer id of the app-border + corner-mask foreground layer. Excluded
/// from zoom propagation so the border stays anchored to the OS window.
pub fn app_border_layer_id() -> egui::LayerId {
  egui::LayerId::new(egui::Order::Foreground, egui::Id::new("app_border"))
}

/// Paint four opaque corner masks that hide the zoomed-content overflow
/// in the rounded-corner "bites" of the viewport.
///
/// Call this on the app-border layer (which is excluded from the zoom
/// transform by [`propagate`]) when the zoom is active. Skip it when
/// the zoom is at identity — painting the masks unconditionally would
/// turn the normally-transparent corner bites into solid `fill`.
pub fn mask_corners(
  painter: &egui::Painter,
  rect: egui::Rect,
  radius: f32,
  fill: egui::Color32,
) {
  // Outer corner, arc center (inset by radius on both axes), and the
  // arc sweep (start → end angle) for each of the four corners.
  let corners: [(egui::Pos2, egui::Pos2, f32, f32); 4] = [
    (
      rect.min,
      rect.min + egui::vec2(radius, radius),
      -std::f32::consts::FRAC_PI_2,
      -std::f32::consts::PI,
    ),
    (
      egui::pos2(rect.max.x, rect.min.y),
      egui::pos2(rect.max.x - radius, rect.min.y + radius),
      -std::f32::consts::FRAC_PI_2,
      0.0,
    ),
    (
      rect.max,
      rect.max - egui::vec2(radius, radius),
      0.0,
      std::f32::consts::FRAC_PI_2,
    ),
    (
      egui::pos2(rect.min.x, rect.max.y),
      egui::pos2(rect.min.x + radius, rect.max.y - radius),
      std::f32::consts::FRAC_PI_2,
      std::f32::consts::PI,
    ),
  ];

  const ARC_STEPS: usize = 12;

  for (outer, arc_center, start, end) in corners {
    let mut points = Vec::with_capacity(ARC_STEPS + 2);

    points.push(outer);

    for i in 0..=ARC_STEPS {
      let t = i as f32 / ARC_STEPS as f32;
      let angle = start + (end - start) * t;
      points.push(
        arc_center + egui::vec2(angle.cos(), angle.sin()) * radius,
      );
    }

    painter.add(egui::Shape::Path(egui::epaint::PathShape {
      points,
      closed: true,
      fill,
      stroke: egui::epaint::PathStroke::NONE,
    }));
  }
}

/// Current camera transform, or `None` if the zoom is effectively 1x.
/// Returning `None` lets callers skip-wrap on the identity case.
pub fn transform(world: &World) -> Option<egui::emath::TSTransform> {
  let state = world.resource::<MagicZoomState>();
  let zoom = state.zoom();

  if (zoom - 1.0).abs() < 0.001 {
    return None;
  }

  let (cx, cy) = state.center();
  let c = egui::vec2(cx, cy);

  Some(
    egui::emath::TSTransform::from_translation(c)
      * egui::emath::TSTransform::from_scaling(zoom)
      * egui::emath::TSTransform::from_translation(-c),
  )
}

/// Propagate the magic-zoom transform to every visible layer except
/// the main-body sublayer (already transformed by the caller).
///
/// On idle frames this pushes `TSTransform::IDENTITY` to clear any
/// stale entries from a just-finished zoom (egui stores transforms
/// across frames).
pub fn propagate(ctx: &egui::Context, world: &World) {
  let magic_id = layer_id();
  let border_id = app_border_layer_id();
  let transform = transform(world).unwrap_or(egui::emath::TSTransform::IDENTITY);

  let layer_ids: Vec<egui::LayerId> = ctx.memory(|m| m.layer_ids().collect());

  for id in layer_ids {
    if id == magic_id || id == border_id {
      continue;
    }

    ctx.set_transform_layer(id, transform);
  }
}
