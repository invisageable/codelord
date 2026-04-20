use crate::assets::font;
use crate::assets::icon::icon_to_image;
use crate::assets::theme::get_theme;

use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::ActiveAnimations;
use codelord_core::ecs::world::World;
use codelord_core::keyboard::resources::KeyboardFocus;
use codelord_core::xmb::components::XmbNavigation;
use codelord_core::xmb::resources::{XmbCommand, XmbResource};

use eazy::{Curve, Easing};

use eframe::egui;

/// XMB rendering context - holds all layout constants and theme colors.
struct XmbRenderCtx {
  center: egui::Pos2,
  x_offset: f32,
  text_color: egui::Color32,
  // Layout constants
  category_spacing: f32,
  item_spacing: f32,
  category_icon_unselected: f32,
  category_icon_selected: f32,
  item_icon_unselected: f32,
  item_icon_selected: f32,
}

impl XmbRenderCtx {
  fn new(world: &World, rect: egui::Rect, current_focus_x: f32) -> Self {
    let theme = get_theme(world);
    let text_color = egui::Color32::from_rgba_unmultiplied(
      theme.text[0],
      theme.text[1],
      theme.text[2],
      theme.text[3],
    );

    let category_spacing = 120.0;

    Self {
      center: rect.center(),
      x_offset: -current_focus_x * category_spacing,
      text_color,
      category_spacing,
      item_spacing: 60.0,
      category_icon_unselected: 32.0,
      category_icon_selected: 40.0,
      item_icon_unselected: 16.0,
      item_icon_selected: 24.0,
    }
  }
}

/// XMB (XrossMediaBar) component - PSP-style navigation menu.
///
/// ECS Pattern:
/// - READS state from XmbResource, DeltaTime, ThemeResource
/// - SENDS commands via write_message (XmbCommand)
/// - Uses ActiveAnimations for repaint tracking
pub fn show(ui: &mut egui::Ui, world: &mut World, rect: egui::Rect) {
  let delta = world
    .get_resource::<DeltaTime>()
    .map(|dt| dt.delta())
    .unwrap_or(0.016);

  // Handle keyboard navigation - only if nothing else has focus
  let something_has_focus = world
    .get_resource::<KeyboardFocus>()
    .map(|f| f.is_focused())
    .unwrap_or(false);

  if !something_has_focus {
    ui.ctx().input(|input| {
      let nav = if input.key_pressed(egui::Key::ArrowRight) {
        Some(XmbNavigation::Right)
      } else if input.key_pressed(egui::Key::ArrowLeft) {
        Some(XmbNavigation::Left)
      } else if input.key_pressed(egui::Key::ArrowDown) {
        Some(XmbNavigation::Down)
      } else if input.key_pressed(egui::Key::ArrowUp) {
        Some(XmbNavigation::Up)
      } else if input.key_pressed(egui::Key::Enter) {
        Some(XmbNavigation::Select)
      } else {
        None
      };

      if let Some(n) = nav {
        world.write_message(XmbCommand { navigation: n });
      }
    });
  }

  // Update animation
  let (animation_transition, description_animating) = world
    .get_resource_mut::<XmbResource>()
    .map(|mut xmb| {
      xmb.update_animation(delta);
      let desc_animating = xmb.update_description_animation(delta);
      (xmb.check_animation_transition(), desc_animating)
    })
    .unwrap_or(((false, false), false));

  // Track animation via ActiveAnimations
  if let Some(mut anims) = world.get_resource_mut::<ActiveAnimations>() {
    if animation_transition.0 {
      anims.increment();
    }

    if animation_transition.1 {
      anims.decrement();
    }
  }

  // Track hacker animation for continuous repaint.
  if description_animating
    && let Some(mut continuous) =
      world.get_resource_mut::<codelord_core::animation::resources::ContinuousAnimations>()
    {
      continuous.set_hacker_active();
  }

  // Render
  render(ui, world, rect);
}

fn render(ui: &mut egui::Ui, world: &mut World, rect: egui::Rect) {
  let xmb = match world.get_resource::<XmbResource>() {
    Some(x) => x,
    None => return,
  };

  let ctx = XmbRenderCtx::new(world, rect, xmb.current_focus_pos.0);
  let focused_x = xmb.focused_x;
  let current_focus_pos = xmb.current_focus_pos;
  let categories = &xmb.categories;

  // Click detection + rendering
  let mut category_click: Option<usize> = None;
  let mut item_click: Option<usize> = None;

  for (i, category) in categories.iter().enumerate() {
    let category_x =
      ctx.center.x + ctx.x_offset + i as f32 * ctx.category_spacing;
    let distance = (i as f32 - current_focus_pos.0).abs();

    // Category icon size with easing
    let icon_size = if distance < 0.1 {
      ctx.category_icon_selected
    } else {
      let t = (1.0 - distance.min(1.0)).max(0.0);

      ctx.category_icon_unselected
        + (ctx.category_icon_selected - ctx.category_icon_unselected)
          * Easing::InOutSine.y(t)
    };

    let icon_pos = egui::pos2(category_x, ctx.center.y - 100.0);

    // Click detection
    let click_rect = egui::Rect::from_center_size(
      icon_pos,
      egui::vec2(icon_size * 1.5, icon_size * 1.5),
    );

    let response = ui.allocate_rect(click_rect, egui::Sense::click());

    if response.clicked() && i != focused_x {
      category_click = Some(i);
    }

    if response.hovered() {
      ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
    }

    // Render category icon
    let alpha =
      0.3 + 0.7 * Easing::OutCubic.y((1.0 - distance * 0.5).clamp(0.0, 1.0));

    let tint = egui::Color32::from_rgba_unmultiplied(
      ctx.text_color.r(),
      ctx.text_color.g(),
      ctx.text_color.b(),
      (ctx.text_color.a() as f32 * alpha) as u8,
    );

    let icon_rect =
      egui::Rect::from_center_size(icon_pos, egui::vec2(icon_size, icon_size));

    ui.put(
      icon_rect,
      icon_to_image(&category.icon)
        .fit_to_exact_size(egui::vec2(icon_size, icon_size))
        .tint(tint),
    );

    // Category name
    if distance < 0.5 {
      let name_alpha =
        Easing::OutCubic.y((1.0 - distance * 2.0).clamp(0.0, 1.0));

      let name_color = egui::Color32::from_rgba_unmultiplied(
        ctx.text_color.r(),
        ctx.text_color.g(),
        ctx.text_color.b(),
        (ctx.text_color.a() as f32 * name_alpha) as u8,
      );

      ui.painter().text(
        icon_pos + egui::vec2(0.0, icon_size * 0.8),
        egui::Align2::CENTER_CENTER,
        &category.name,
        font::suisse(14.0),
        name_color,
      );
    }

    // Render items for focused category
    if i == focused_x {
      let block_height = ctx.category_icon_selected * 0.8 + 14.0 + 10.0;
      let focused_y_float = current_focus_pos.1;
      let focused_y_int = focused_y_float.round() as usize;

      for (j, item) in category.items.iter().enumerate() {
        let offset = j as f32 - focused_y_float;
        let item_dist = offset.abs();

        if item_dist > 3.0 {
          continue;
        }

        let item_y = if j < focused_y_int {
          let above = focused_y_int - j;
          ctx.center.y
            - block_height
            - (above as f32) * ctx.item_spacing
            - ctx.item_icon_selected * 1.5
        } else {
          let below = j - focused_y_int;
          ctx.center.y + (below as f32) * ctx.item_spacing
        };

        let item_icon_size = if item_dist < 0.1 {
          ctx.item_icon_selected
        } else {
          let t = (1.0 - item_dist.min(1.0)).max(0.0);
          ctx.item_icon_unselected
            + (ctx.item_icon_selected - ctx.item_icon_unselected)
              * Easing::OutBack.y(t)
        };

        let text_size = if item_dist < 0.1 {
          16.0
        } else {
          let t = (1.0 - item_dist.min(1.0)).max(0.0);
          12.0 + 4.0 * Easing::OutBack.y(t)
        };

        // Item click detection
        let item_rect = egui::Rect::from_center_size(
          egui::pos2(category_x + 80.0, item_y),
          egui::vec2(300.0, 50.0 * (item_icon_size / ctx.item_icon_unselected)),
        );

        let item_response = ui.allocate_rect(item_rect, egui::Sense::click());

        if item_response.clicked() {
          item_click = Some(j);
        }

        if item_response.hovered() {
          ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }

        let item_alpha = 0.2
          + 0.8 * Easing::OutCubic.y((1.0 - item_dist * 0.3).clamp(0.0, 1.0));

        // Selection highlight
        if j == focused_y_int {
          let sel_color = ui.visuals().selection.bg_fill.linear_multiply(0.3);

          ui.painter().rect_filled(
            item_rect,
            egui::CornerRadius::ZERO,
            sel_color,
          );
        }

        // Item icon
        let item_tint = egui::Color32::from_rgba_unmultiplied(
          ctx.text_color.r(),
          ctx.text_color.g(),
          ctx.text_color.b(),
          (ctx.text_color.a() as f32 * item_alpha) as u8,
        );

        let item_icon_rect = egui::Rect::from_center_size(
          egui::pos2(category_x, item_y),
          egui::vec2(item_icon_size, item_icon_size),
        );

        ui.put(
          item_icon_rect,
          icon_to_image(&item.icon)
            .fit_to_exact_size(egui::vec2(item_icon_size, item_icon_size))
            .tint(item_tint),
        );

        // Item text
        let text_alpha =
          Easing::OutCubic.y((1.0 - item_dist * 0.3).clamp(0.0, 1.0));

        let text_color = egui::Color32::from_rgba_unmultiplied(
          ctx.text_color.r(),
          ctx.text_color.g(),
          ctx.text_color.b(),
          (ctx.text_color.a() as f32 * text_alpha) as u8,
        );

        ui.painter().text(
          egui::pos2(category_x + 50.0, item_y),
          egui::Align2::LEFT_CENTER,
          &item.name,
          egui::FontId::proportional(text_size),
          text_color,
        );
      }
    }
  }

  // Send click commands
  if let Some(idx) = category_click {
    world.write_message(XmbCommand {
      navigation: XmbNavigation::JumpToCategory(idx),
    });
  }

  if let Some(idx) = item_click {
    world.write_message(XmbCommand {
      navigation: XmbNavigation::JumpToItem(idx),
    });
  }
}
