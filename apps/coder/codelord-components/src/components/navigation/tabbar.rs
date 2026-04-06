use crate::assets::icon::icon_to_image;

use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::ecs::prelude::{Entity, Has, With};
use codelord_core::ecs::world::World;
use codelord_core::events::{
  ActivateTabRequest, CloseTabRequest, ToggleZoomRequest,
};
use codelord_core::icon::components::{Arrow, Icon, Zoom};
use codelord_core::popup::resources::{
  PopupAction, PopupCommand, PopupResource,
};
use codelord_core::tabbar::components::{SonarAnimation, Tab, TabMarker};
use codelord_core::tabbar::resources::{TabContextTarget, ZoomState};
use codelord_core::ui::component::{Active, Modified};

use eframe::egui;

/// Button size for tabbar control buttons.
const BUTTON_SIZE: egui::Vec2 = egui::vec2(24.0, 24.0);
/// Icon size within buttons.
const ICON_SIZE: egui::Vec2 = egui::vec2(12.0, 12.0);

/// Generic tabbar renderer with full controls.
/// M is the marker component that identifies which tabs to show.
/// Layout: [<] [>] [tabs...] [+] [zoom]
pub fn show<M: TabMarker>(ui: &mut egui::Ui, world: &mut World) {
  let visuals = ui.style().visuals.clone();
  let separator_color = visuals.widgets.noninteractive.bg_stroke.color;

  // Get zoom state from ECS
  let is_zoomed = world
    .get_resource::<ZoomState>()
    .map(|z| z.is_zoomed)
    .unwrap_or(false);

  // Collect tab data for rendering
  let mut tabs = world
    .query_filtered::<(Entity, &Tab, Has<Active>, Has<Modified>), With<M>>()
    .iter(world)
    .map(|(e, tab, is_active, is_modified)| {
      (e, tab.label.clone(), tab.order, is_active, is_modified)
    })
    .collect::<Vec<(Entity, String, u32, bool, bool)>>();

  // Sort by order to maintain consistent tab positions
  tabs.sort_by_key(|(_, _, order, _, _)| *order);

  let tab_count = tabs.len();
  let active_index = tabs.iter().position(|(_, _, _, is_active, _)| *is_active);
  let current_index = active_index.unwrap_or(0);

  let can_go_left = current_index > 0;
  let can_go_right = current_index < tab_count.saturating_sub(1);

  // Collect events to spawn after rendering
  let mut activate_entity: Option<Entity> = None;
  let mut close_entity: Option<Entity> = None;
  let mut context_menu_entity: Option<(Entity, u32)> = None;
  let mut spawn_new_tab = false;
  let mut spawn_toggle_zoom = false;

  ui.horizontal_centered(|ui| {
    ui.set_height(24.0);
    ui.spacing_mut().item_spacing.x = 0.0;

    // Draw bottom border across entire tab bar
    let full_rect = ui.max_rect();
    ui.painter().rect_filled(
      egui::Rect::from_min_size(
        egui::pos2(full_rect.left(), full_rect.bottom() - 1.0),
        egui::vec2(full_rect.width(), 1.0),
      ),
      egui::CornerRadius::ZERO,
      separator_color,
    );

    // Left side: Arrow navigation buttons
    let arrow_result = controls::arrows(ui, can_go_left, can_go_right);

    if arrow_result.left_clicked
      && current_index > 0
      && let Some((entity, _, _, _, _)) = tabs.get(current_index - 1)
    {
      activate_entity = Some(*entity);
    }

    if arrow_result.right_clicked
      && current_index < tab_count.saturating_sub(1)
      && let Some((entity, _, _, _, _)) = tabs.get(current_index + 1)
    {
      activate_entity = Some(*entity);
    }

    // Center: Scrollable tab area
    egui::ScrollArea::horizontal()
      .id_salt("tabbar_scroll")
      .auto_shrink([false; 2])
      .scroll_bar_visibility(
        egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
      )
      .show(ui, |ui| {
        ui.horizontal_centered(|ui| {
          ui.spacing_mut().item_spacing.x = 0.0;

          for (entity, _label, order, is_active, _is_modified) in &tabs {
            let result = render_tab(ui, world, *entity);

            if result.clicked && !is_active {
              activate_entity = Some(*entity);
            }
            if result.close_clicked {
              close_entity = Some(*entity);
            }
            if result.context_menu_requested {
              context_menu_entity = Some((*entity, *order));
            }
          }

          // Allocate remaining space for double-click to add tab
          let available_rect = ui.available_rect_before_wrap();
          let empty_space_response =
            ui.allocate_rect(available_rect, egui::Sense::click());
          if empty_space_response.double_clicked() {
            spawn_new_tab = true;
          }

          // Add padding at the end so last tab doesn't get hidden under right
          // controls
          ui.add_space(48.0);
        });
      });

    // Right side: Add and Zoom buttons (right-to-left layout)
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      ui.spacing_mut().item_spacing.x = 0.0;

      if controls::zoom(ui, is_zoomed) {
        spawn_toggle_zoom = true;
      }

      if controls::add(ui) {
        spawn_new_tab = true;
      }
    });
  });

  // Spawn ECS events
  if let Some(entity) = activate_entity {
    world.spawn(ActivateTabRequest::new(entity));
  }

  if let Some(entity) = close_entity {
    world.spawn(CloseTabRequest::new(entity));
  }

  if spawn_new_tab {
    M::spawn_new_tab_event(world);
  }

  if spawn_toggle_zoom {
    world.spawn(ToggleZoomRequest {
      source: M::zoom_source(),
    });
  }

  // Handle context menu request
  if let Some((entity, order)) = context_menu_entity {
    // Update context target
    if let Some(mut target) = world.get_resource_mut::<TabContextTarget>() {
      target.entity = Some(entity);
      target.order = order;
    }

    // Show context menu popup at cursor position
    if let Some(popup_entity) = world
      .get_resource::<PopupResource>()
      .and_then(|r| r.tab_context_popup)
    {
      let cursor_pos =
        ui.input(|i| i.pointer.hover_pos().unwrap_or(egui::pos2(0.0, 0.0)));

      world.write_message(PopupCommand {
        action: PopupAction::Show {
          entity: popup_entity,
          anchor_rect: [cursor_pos.x, cursor_pos.y, 0.0, 0.0],
        },
      });
    }
  }
}

/// Tab layout constants.
const LEFT_SLOT_WIDTH: f32 = 24.0;
const RIGHT_SLOT_WIDTH: f32 = 24.0;
const CLOSE_ICON_SIZE: egui::Vec2 = egui::vec2(10.0, 10.0);
const SONAR_BASE_RADIUS: f32 = 1.5;
/// Codelord green color for modified indicator.
const CODELORD_GREEN: egui::Color32 = egui::Color32::from_rgb(204, 253, 62);

/// Result of rendering a tab.
pub struct TabRenderResult {
  pub clicked: bool,
  pub close_clicked: bool,
  pub context_menu_requested: bool,
}

/// Render a tab with the codelord design using pure ECS.
/// Layout: [modified indicator] [label] [close icon]
fn render_tab(
  ui: &mut egui::Ui,
  world: &mut World,
  entity: Entity,
) -> TabRenderResult {
  let visuals = ui.style().visuals.clone();
  let bg_color = visuals.window_fill;
  let separator_color = visuals.widgets.noninteractive.bg_stroke.color;

  // Query tab data from ECS
  let tab = world.get::<Tab>(entity);
  let is_active = world.get::<Active>(entity).is_some();
  let is_modified = world.get::<Modified>(entity).is_some();

  let label = tab.map(|t| t.label.clone()).unwrap_or_default();

  // Calculate dynamic width based on text content
  let text_width = ui.fonts_mut(|f| {
    f.layout_no_wrap(
      label.clone(),
      egui::FontId::proportional(12.0),
      egui::Color32::WHITE,
    )
    .rect
    .width()
  });

  let tab_width = LEFT_SLOT_WIDTH + text_width + RIGHT_SLOT_WIDTH;
  let tab_height = ui.available_height();

  let mut result = TabRenderResult {
    clicked: false,
    close_clicked: false,
    context_menu_requested: false,
  };

  ui.allocate_ui_with_layout(
    egui::vec2(tab_width, tab_height),
    egui::Layout::left_to_right(egui::Align::Center),
    |ui| {
      let rect = ui.max_rect();

      // Background
      ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, bg_color);

      // Right border separator
      let right_border_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - 1.0, rect.top()),
        egui::vec2(1.0, rect.height()),
      );
      ui.painter().rect_filled(
        right_border_rect,
        egui::CornerRadius::ZERO,
        separator_color,
      );

      // Active tab: cover the bottom border to create "open tab" effect
      if is_active {
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
          egui::Order::Foreground,
          egui::Id::new("active_tab_bottom").with(entity),
        ));
        let bottom_cover_rect = egui::Rect::from_min_size(
          egui::pos2(rect.left(), rect.bottom() - 1.0),
          egui::vec2(rect.width() - 1.0, 1.0),
        );
        painter.rect_filled(
          bottom_cover_rect,
          egui::CornerRadius::ZERO,
          bg_color,
        );
      }

      // Check if hovering over this tab
      let is_hovering = ui
        .ctx()
        .pointer_hover_pos()
        .is_some_and(|pos| rect.contains(pos));

      // Interactive area for the whole tab
      let response =
        ui.interact(rect, ui.id().with(entity), egui::Sense::click());

      // Text color based on active state
      let text_color = if is_active {
        egui::Color32::WHITE
      } else {
        egui::Color32::from_gray(140)
      };

      // Zone 1: Left slot for modified indicator (sonar animation)
      let left_slot_center_x = rect.left() + LEFT_SLOT_WIDTH / 2.0;
      let left_slot_center = egui::pos2(left_slot_center_x, rect.center().y);

      if is_modified {
        // Get current time for animation
        let current_time = ui.input(|i| i.time);

        // Get sonar animation settings (use synchronized timing)
        if let Some(sonar) = world.get::<SonarAnimation>(entity) {
          // Synchronized animation: all modified tabs pulse together
          // using current_time % duration
          let progress =
            ((current_time % sonar.duration) / sonar.duration) as f32;

          // Use eazy easing
          use eazy::Curve;
          let eased = sonar.easing.y(progress);

          let scale = sonar.initial_scale
            + (sonar.final_scale - sonar.initial_scale) * eased;
          let opacity = sonar.initial_opacity
            + (sonar.final_opacity - sonar.initial_opacity) * eased;

          // Mark sonar as active for repaint requests
          if let Some(mut anim) =
            world.get_resource_mut::<ContinuousAnimations>()
          {
            anim.set_sonar_active();
          }

          // Draw expanding wave (same as codelord)
          if opacity > 0.001 {
            let wave_radius = SONAR_BASE_RADIUS * scale;
            let wave_color = egui::Color32::from_rgba_premultiplied(
              204,
              253,
              62,
              (opacity * 255.0) as u8,
            );
            ui.painter().circle_stroke(
              left_slot_center,
              wave_radius,
              egui::Stroke::new(1.5, wave_color),
            );
          }

          // Draw core dot (always visible when modified)
          ui.painter().circle_filled(
            left_slot_center,
            SONAR_BASE_RADIUS,
            CODELORD_GREEN,
          );
        }
      }

      // Zone 2: Tab text
      let text_x = rect.left() + LEFT_SLOT_WIDTH;
      let text_pos = egui::pos2(text_x, rect.center().y - 1.0);
      ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        &label,
        egui::FontId::proportional(12.0),
        text_color,
      );

      // Zone 3: Close button (visible on hover)
      let close_center_x = rect.right() - RIGHT_SLOT_WIDTH / 2.0;
      let close_icon_rect = egui::Rect::from_center_size(
        egui::pos2(close_center_x, rect.center().y),
        CLOSE_ICON_SIZE,
      );

      let close_tint = if is_hovering {
        egui::Color32::LIGHT_GRAY
      } else {
        egui::Color32::TRANSPARENT
      };

      ui.put(
        close_icon_rect,
        icon_to_image(&Icon::Close)
          .fit_to_exact_size(CLOSE_ICON_SIZE)
          .tint(close_tint),
      );

      // Cursor handling
      if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
      }

      // Click handling - check if close button zone was clicked
      if response.clicked()
        && let Some(pos) = ui.ctx().pointer_hover_pos()
      {
        if pos.x > rect.right() - RIGHT_SLOT_WIDTH {
          result.close_clicked = true;
        } else {
          result.clicked = true;
        }
      }

      // Right-click handling - show context menu
      if response.secondary_clicked() {
        result.context_menu_requested = true;
      }
    },
  );

  result
}

/// Tabbar control buttons module.
/// Provides reusable navigation buttons (arrows, zoom, add) for tabbars.
pub mod controls {
  use super::*;

  /// Result of rendering navigation arrows.
  pub struct ArrowResult {
    pub left_clicked: bool,
    pub right_clicked: bool,
  }

  /// Render left and right navigation arrow buttons.
  /// Returns which arrow was clicked (if any).
  pub fn arrows(
    ui: &mut egui::Ui,
    can_go_left: bool,
    can_go_right: bool,
  ) -> ArrowResult {
    let visuals = ui.style().visuals.clone();
    let separator_color = visuals.widgets.noninteractive.bg_stroke.color;
    let bg_color = visuals.window_fill;
    let hover_bg = visuals.widgets.hovered.bg_fill;
    let enabled_tint = visuals.widgets.inactive.fg_stroke.color;
    let disabled_tint = egui::Color32::from_gray(60);

    let mut result = ArrowResult {
      left_clicked: false,
      right_clicked: false,
    };

    // Left arrow
    let left_sense = if can_go_left {
      egui::Sense::click()
    } else {
      egui::Sense::hover()
    };

    let left_response = ui.allocate_response(BUTTON_SIZE, left_sense);
    let left_rect = left_response.rect;

    // Background
    ui.painter().rect_filled(
      left_rect,
      egui::CornerRadius::ZERO,
      if can_go_left && left_response.hovered() {
        hover_bg
      } else {
        bg_color
      },
    );

    // Icon
    ui.put(
      left_rect,
      icon_to_image(&Icon::Arrow(Arrow::Left))
        .fit_to_exact_size(ICON_SIZE)
        .tint(if can_go_left {
          enabled_tint
        } else {
          disabled_tint
        }),
    );

    // Separator after left arrow
    ui.painter().line_segment(
      [
        egui::pos2(left_rect.right(), left_rect.top()),
        egui::pos2(left_rect.right(), left_rect.bottom()),
      ],
      egui::Stroke::new(1.0, separator_color),
    );

    // Cursor and click handling
    if !can_go_left && left_response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
    }
    if can_go_left && left_response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if can_go_left && left_response.clicked() {
      result.left_clicked = true;
    }

    // Right arrow
    let right_sense = if can_go_right {
      egui::Sense::click()
    } else {
      egui::Sense::hover()
    };

    let right_response = ui.allocate_response(BUTTON_SIZE, right_sense);
    let right_rect = right_response.rect;

    // Background
    ui.painter().rect_filled(
      right_rect,
      egui::CornerRadius::ZERO,
      if can_go_right && right_response.hovered() {
        hover_bg
      } else {
        bg_color
      },
    );

    // Icon
    ui.put(
      right_rect,
      icon_to_image(&Icon::Arrow(Arrow::Right))
        .fit_to_exact_size(ICON_SIZE)
        .tint(if can_go_right {
          enabled_tint
        } else {
          disabled_tint
        }),
    );

    // Separator after right arrow
    ui.painter().line_segment(
      [
        egui::pos2(right_rect.right(), right_rect.top()),
        egui::pos2(right_rect.right(), right_rect.bottom()),
      ],
      egui::Stroke::new(1.0, separator_color),
    );

    // Cursor and click handling
    if !can_go_right && right_response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
    }
    if can_go_right && right_response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if can_go_right && right_response.clicked() {
      result.right_clicked = true;
    }

    result
  }

  /// Render zoom toggle button.
  /// Returns true if clicked.
  pub fn zoom(ui: &mut egui::Ui, is_zoomed: bool) -> bool {
    let visuals = ui.style().visuals.clone();
    let separator_color = visuals.widgets.noninteractive.bg_stroke.color;
    let bg_color = visuals.window_fill;
    let hover_bg = visuals.widgets.hovered.bg_fill;
    let icon_tint = visuals.widgets.inactive.fg_stroke.color;

    let zoom_icon = if is_zoomed {
      Icon::Zoom(Zoom::OutArrow)
    } else {
      Icon::Zoom(Zoom::InArrow)
    };

    let response = ui.allocate_response(BUTTON_SIZE, egui::Sense::click());
    let rect = response.rect;

    // Background
    ui.painter().rect_filled(
      rect,
      egui::CornerRadius::ZERO,
      if response.hovered() {
        hover_bg
      } else {
        bg_color
      },
    );

    // Icon
    ui.put(
      rect,
      icon_to_image(&zoom_icon)
        .fit_to_exact_size(ICON_SIZE)
        .tint(icon_tint),
    );

    // Separator before zoom button
    ui.painter().line_segment(
      [
        egui::pos2(rect.left(), rect.top()),
        egui::pos2(rect.left(), rect.bottom()),
      ],
      egui::Stroke::new(1.0, separator_color),
    );

    // Cursor
    if response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    response.clicked()
  }

  /// Render add button.
  /// Returns true if clicked.
  pub fn add(ui: &mut egui::Ui) -> bool {
    let visuals = ui.style().visuals.clone();
    let separator_color = visuals.widgets.noninteractive.bg_stroke.color;
    let bg_color = visuals.window_fill;
    let hover_bg = visuals.widgets.hovered.bg_fill;
    let icon_tint = visuals.widgets.inactive.fg_stroke.color;

    let response = ui.allocate_response(BUTTON_SIZE, egui::Sense::click());
    let rect = response.rect;

    // Background
    ui.painter().rect_filled(
      rect,
      egui::CornerRadius::ZERO,
      if response.hovered() {
        hover_bg
      } else {
        bg_color
      },
    );

    // Icon
    ui.put(
      rect,
      icon_to_image(&Icon::Add)
        .fit_to_exact_size(ICON_SIZE)
        .tint(icon_tint),
    );

    // Separator before add button
    ui.painter().line_segment(
      [
        egui::pos2(rect.left(), rect.top()),
        egui::pos2(rect.left(), rect.bottom()),
      ],
      egui::Stroke::new(1.0, separator_color),
    );

    // Cursor
    if response.hovered() {
      ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    response.clicked()
  }
}
